use std::collections::HashSet;

use rust_decimal::Decimal;

use crate::{Error, Issue, Portfolio, ProductTargeting, SCHEMA_VERSION};

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
        if self.schema_version > SCHEMA_VERSION {
            return Err(Error::UnsupportedVersion {
                found: self.schema_version,
            });
        }
        if self.schema_version == 0 {
            return corrupt("the schema version is missing");
        }

        let mut ids = HashSet::new();
        let all_ids = self
            .categories
            .iter()
            .map(|c| c.id.0)
            .chain(self.products.iter().map(|p| p.id.0))
            .chain(self.accounts.iter().map(|a| a.id.0));
        for id in all_ids {
            if id == 0 || id >= self.next_id || !ids.insert(id) {
                return corrupt("an id is duplicated or out of range");
            }
        }

        for category in &self.categories {
            name(&category.name, "category").map_err(into_corrupt)?;
            fraction(category.target_weight, "category target").map_err(into_corrupt)?;
        }
        for product in &self.products {
            name(&product.name, "product").map_err(into_corrupt)?;
            currency(&product.currency).map_err(into_corrupt)?;
            if !self.categories.iter().any(|c| c.id == product.category) {
                return corrupt("a product refers to a missing category");
            }
            if let Some(weight) = product.weight {
                positive(weight, "product weight").map_err(into_corrupt)?;
            }
        }
        for account in &self.accounts {
            name(&account.name, "account").map_err(into_corrupt)?;
            positive(account.rounding_unit, "rounding unit").map_err(into_corrupt)?;
            for product in account.tradable.iter().flatten() {
                if !self.products.iter().any(|p| p.id == *product) {
                    return corrupt("an account refers to a missing product");
                }
            }
        }

        let mut seen = HashSet::new();
        for holding in &self.holdings {
            non_negative(holding.value, "holding value").map_err(into_corrupt)?;
            if let Some(quantity) = holding.quantity {
                non_negative(quantity, "quantity").map_err(into_corrupt)?;
            }
            if !self.accounts.iter().any(|a| a.id == holding.account)
                || !self.products.iter().any(|p| p.id == holding.product)
            {
                return corrupt("a holding refers to a missing account or product");
            }
            if !seen.insert((holding.account, holding.product)) {
                return corrupt("a product is held twice in the same account");
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
            if !names.insert(recipe.name.clone())
                || recipe.clone().validated(self).as_ref() != Ok(recipe)
            {
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
                && products.iter().any(|p| p.weight.is_none())
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
