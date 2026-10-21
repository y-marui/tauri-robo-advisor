use std::collections::HashSet;

use rust_decimal::Decimal;

use crate::presets::Preset;
use crate::{Error, Issue, MIN_SCHEMA_VERSION, Portfolio, ProductTargeting, SCHEMA_VERSION};

/// The most recipes a data file may hold.
const MAX_RECIPES_ALLOWED: usize = 50;

/// Largest allowed distance between the category weights' sum and 1. Weights
/// are typed by hand, so a tiny tolerance absorbs rounding such as 33.33 + 33.33
/// + 33.34.
const WEIGHT_SUM_TOLERANCE: Decimal = Decimal::from_parts(1, 0, 0, false, 6);

pub(crate) const MAX_NAME_CHARS: usize = 100;

pub(crate) fn name(value: &str, what: &str) -> Result<String, Error> {
    let trimmed = value.trim();
    if trimmed.is_empty() || trimmed.chars().count() > MAX_NAME_CHARS {
        return Err(Error::Invalid(format!(
            "the {what} name must be 1 to {MAX_NAME_CHARS} characters"
        )));
    }
    if trimmed.chars().any(char::is_control) {
        return Err(Error::Invalid(format!(
            "the {what} name must not contain control characters"
        )));
    }
    Ok(trimmed.to_owned())
}

pub(crate) fn fraction(value: Decimal, what: &str) -> Result<Decimal, Error> {
    if value < Decimal::ZERO || value > Decimal::ONE {
        return Err(Error::Invalid(format!(
            "the {what} must be between 0 and 1"
        )));
    }
    Ok(value)
}

pub(crate) fn non_negative(value: Decimal, what: &str) -> Result<Decimal, Error> {
    if value.is_sign_negative() {
        return Err(Error::Invalid(format!("the {what} must not be negative")));
    }
    Ok(value)
}

pub(crate) fn positive(value: Decimal, what: &str) -> Result<Decimal, Error> {
    if value <= Decimal::ZERO {
        return Err(Error::Invalid(format!("the {what} must be positive")));
    }
    Ok(value)
}

pub(crate) fn currency(value: &str) -> Result<String, Error> {
    let valid = value.len() == 3 && value.bytes().all(|b| b.is_ascii_uppercase());
    if valid {
        Ok(value.to_owned())
    } else {
        Err(Error::Invalid(
            "the currency must be three upper-case letters".to_owned(),
        ))
    }
}

impl Portfolio {
    /// Checks everything that must hold for the data to be safe to use: the
    /// version, unique ids, valid references, and value ranges. It is run on
    /// every loaded file, because the file is not trusted to be well formed.
    pub fn validate_integrity(&self) -> Result<(), Error> {
        let corrupt = |message: &str| Err(Error::Corrupt(message.to_owned()));
        if self.schema_version == 0 {
            return corrupt("the schema version is missing");
        }
        if !(MIN_SCHEMA_VERSION..=SCHEMA_VERSION).contains(&self.schema_version) {
            return Err(Error::UnsupportedVersion {
                found: self.schema_version,
            });
        }

        let mut ids = HashSet::new();
        let all_ids = self
            .groups
            .iter()
            .map(|g| g.id.0)
            .chain(self.categories.iter().map(|c| c.id.0))
            .chain(self.products.iter().map(|p| p.id.0))
            .chain(self.accounts.iter().map(|a| a.id.0));
        for id in all_ids {
            if id == 0 || id >= self.next_id || !ids.insert(id) {
                return corrupt("an id is duplicated or out of range");
            }
        }

        let mut group_names = HashSet::new();
        for group in &self.groups {
            name(&group.name, "group").map_err(into_corrupt)?;
            if !group_names.insert(group.name.as_str()) {
                return corrupt("a group name is duplicated");
            }
        }
        for category in &self.categories {
            name(&category.name, "category").map_err(into_corrupt)?;
            if category
                .group
                .is_some_and(|g| !self.groups.iter().any(|x| x.id == g))
            {
                return corrupt("a category refers to a missing group");
            }
            fraction(category.target_weight, "category target").map_err(into_corrupt)?;
        }
        for product in &self.products {
            name(&product.name, "product").map_err(into_corrupt)?;
            currency(&product.currency).map_err(into_corrupt)?;
            if !self.categories.iter().any(|c| c.id == product.category) {
                return corrupt("a product refers to a missing category");
            }
        }
        for account in &self.accounts {
            name(&account.name, "account").map_err(into_corrupt)?;
            positive(account.rounding_unit, "rounding unit").map_err(into_corrupt)?;
            for product in account.tradable.iter().flatten().chain(&account.excluded) {
                if !self.products.iter().any(|p| p.id == *product) {
                    return corrupt("an account refers to a missing product");
                }
            }
        }

        let mut type_ids = HashSet::new();
        let mut type_names = HashSet::new();
        for t in &self.custom_types {
            name(&t.name, "account type").map_err(into_corrupt)?;
            let numbered =
                t.id.strip_prefix("custom-")
                    .and_then(|n| n.parse::<u64>().ok())
                    .is_some_and(|n| n != 0 && n < self.next_id);
            if !numbered || !type_ids.insert(t.id.as_str()) || !type_names.insert(t.name.as_str()) {
                return corrupt("an account type is duplicated or out of range");
            }
        }

        let mut seen = HashSet::new();
        for holding in &self.holdings {
            if holding
                .account_type
                .as_deref()
                .is_some_and(|t| !self.has_account_type(t))
            {
                return corrupt("a holding refers to a missing account type");
            }
            non_negative(holding.value, "holding value").map_err(into_corrupt)?;
            if let Some(quantity) = holding.quantity {
                non_negative(quantity, "quantity").map_err(into_corrupt)?;
            }
            if let Some(price) = holding.price {
                non_negative(price, "price").map_err(into_corrupt)?;
            }
            if let Some(weight) = holding.weight {
                positive(weight, "weight").map_err(into_corrupt)?;
            }
            if holding.price_unit == 0 {
                return corrupt("a holding has a price unit of zero");
            }
            if !self.accounts.iter().any(|a| a.id == holding.account)
                || !self.products.iter().any(|p| p.id == holding.product)
            {
                return corrupt("a holding refers to a missing account or product");
            }
            if !seen.insert((
                holding.account,
                holding.product,
                holding.account_type.as_deref(),
            )) {
                return corrupt("a product is held twice in the same account with the same type");
            }
        }

        let settings = &self.settings;
        fraction(settings.category_tolerance, "category tolerance").map_err(into_corrupt)?;
        fraction(settings.product_tolerance, "product tolerance").map_err(into_corrupt)?;
        if let Some(rate) = settings.usd_jpy {
            positive(rate, "exchange rate").map_err(into_corrupt)?;
        }
        if let Some(account) = settings.target_account
            && !self.accounts.iter().any(|a| a.id == account)
        {
            return corrupt("the target account is missing");
        }
        // Saved mappings and recipes were checked when they were saved; a file is
        // not trusted to have been written by this app, so the same rules apply here.
        let mut sources = HashSet::new();
        for mapping in &self.import_mappings {
            if !sources.insert(mapping.source.clone())
                || mapping.clone().validated().as_ref() != Ok(mapping)
            {
                return corrupt("a saved import mapping is not valid");
            }
        }
        let mut names = HashSet::new();
        if self.recipes.len() > MAX_RECIPES_ALLOWED {
            return corrupt("there are too many recipes");
        }
        for recipe in &self.recipes {
            // A recipe that refers to a preset is only a reference (what it stores is not compared
            // with what the current preset would make of it: an app update may have dropped a
            // link whose destination it remembers). When the preset is not in this version of the
            // app, it is kept as it is, and is an error only where it is used.
            let valid = if recipe.preset.is_some() {
                name(&recipe.name, "recipe").as_ref() == Ok(&recipe.name)
                    && recipe.is_clean_preset_reference()
                    && match recipe.preset.as_deref().and_then(Preset::find) {
                        Some(_) => recipe.clone().validated(self).is_ok(),
                        None => self.accounts.iter().any(|a| a.id == recipe.account),
                    }
            } else {
                recipe.clone().validated(self).as_ref() == Ok(recipe)
            };
            if !names.insert(recipe.name.clone()) || !valid {
                return corrupt("a recipe is not valid");
            }
        }
        Ok(())
    }

    /// Problems that do not stop the data from being saved, but have to be
    /// fixed before suggestions can be computed.
    pub fn issues(&self) -> Vec<Issue> {
        let mut issues = Vec::new();

        if !self.categories.is_empty() {
            let sum = self
                .categories
                .iter()
                .fold(Decimal::ZERO, |acc, c| acc.saturating_add(c.target_weight));
            if (sum - Decimal::ONE).abs() > WEIGHT_SUM_TOLERANCE {
                issues.push(Issue::CategoryWeightsDoNotSumToOne { sum });
            }
        }

        for category in &self.categories {
            let products: Vec<_> = self
                .products
                .iter()
                .filter(|p| p.category == category.id)
                .collect();
            if category.targeting != ProductTargeting::Free && products.is_empty() {
                issues.push(Issue::CategoryHasNoProducts {
                    category: category.id,
                });
            }
            if category.targeting == ProductTargeting::Weighted
                && products.iter().any(|p| self.product_weight(p.id).is_none())
            {
                issues.push(Issue::WeightedProductMissingWeight {
                    category: category.id,
                });
            }
        }

        if self.settings.usd_jpy.is_none() {
            let mut missing: Vec<&str> = self
                .holdings
                .iter()
                .filter_map(|h| self.products.iter().find(|p| p.id == h.product))
                .map(|p| p.currency.as_str())
                .filter(|code| *code != "JPY")
                .collect();
            missing.sort_unstable();
            missing.dedup();
            issues.extend(missing.into_iter().map(|code| Issue::MissingExchangeRate {
                currency: code.to_owned(),
            }));
        }
        issues
    }
}

fn into_corrupt(error: Error) -> Error {
    match error {
        Error::Invalid(message) => Error::Corrupt(message),
        other => other,
    }
}
