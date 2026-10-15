use rust_decimal::Decimal;
use serde::Deserialize;

use crate::validate::{currency, fraction, name, non_negative, positive};
use crate::{
    Account, AccountId, AccountKind, Alias, Category, CategoryId, Error, Funding, Holding,
    Portfolio, Product, ProductId, ProductTargeting,
};

/// One user action. The frontend sends the intent and Rust validates it, assigns
/// ids, and applies it, so no rule lives in the UI.
#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Edit {
    AddCategory {
        name: String,
        target_weight: Decimal,
    },
    UpdateCategory {
        id: CategoryId,
        name: Option<String>,
        target_weight: Option<Decimal>,
        targeting: Option<ProductTargeting>,
    },
    RemoveCategory {
        id: CategoryId,
    },
    AddProduct {
        name: String,
        category: CategoryId,
        currency: String,
    },
    UpdateProduct {
        id: ProductId,
        name: Option<String>,
        category: Option<CategoryId>,
        currency: Option<String>,
    },
    SetProductWeight {
        id: ProductId,
        weight: Option<Decimal>,
    },
    RemoveProduct {
        id: ProductId,
    },
    AddAlias {
        product: ProductId,
        source: String,
        name: String,
    },
    RemoveAlias {
        product: ProductId,
        source: String,
        name: String,
    },
    AddAccount {
        name: String,
        kind: AccountKind,
    },
    UpdateAccount {
        id: AccountId,
        name: Option<String>,
        kind: Option<AccountKind>,
        sellable: Option<bool>,
        funding: Option<Funding>,
        rounding_unit: Option<Decimal>,
    },
    SetTradable {
        id: AccountId,
        /// `None` means the account can buy any product.
        products: Option<Vec<ProductId>>,
    },
    RemoveAccount {
        id: AccountId,
    },
    SetHolding {
        account: AccountId,
        product: ProductId,
        value: Decimal,
        quantity: Option<Decimal>,
    },
    RemoveHolding {
        account: AccountId,
        product: ProductId,
    },
    /// Makes the account hold exactly these products, as an import does. The
    /// values are in each product's own currency.
    ReplaceHoldings {
        account: AccountId,
        holdings: Vec<HoldingInput>,
    },
    SetExchangeRate {
        usd_jpy: Option<Decimal>,
    },
    SetTolerances {
        category: Decimal,
        product: Decimal,
    },
    SetTargetAccount {
        account: Option<AccountId>,
    },
}

/// One product's value in a [`Edit::ReplaceHoldings`].
#[derive(Debug, Clone, PartialEq, Deserialize)]
pub struct HoldingInput {
    pub product: ProductId,
    pub value: Decimal,
}

const MAX_ITEMS: usize = 10_000;
const MAX_ALIASES_PER_PRODUCT: usize = 50;

impl Portfolio {
    /// Applies one edit. The edit is fully validated before anything changes,
    /// so a rejected edit leaves the portfolio untouched.
    pub fn apply(&mut self, edit: Edit) -> Result<(), Error> {
        match edit {
            Edit::AddCategory {
                name: new_name,
                target_weight,
            } => {
                self.check_capacity(self.categories.len())?;
                let new_name = name(&new_name, "category")?;
                let target_weight = fraction(target_weight, "category target")?;
                self.ensure_category_name_free(&new_name, None)?;
                let id = CategoryId(self.take_id());
                self.categories.push(Category {
                    id,
                    name: new_name,
                    target_weight,
                    targeting: ProductTargeting::default(),
                });
            }
            Edit::UpdateCategory {
                id,
                name: new_name,
                target_weight,
                targeting,
            } => {
                let new_name = new_name.map(|n| name(&n, "category")).transpose()?;
                let target_weight = target_weight
                    .map(|w| fraction(w, "category target"))
                    .transpose()?;
                self.category(id)?;
                if let Some(n) = &new_name {
                    self.ensure_category_name_free(n, Some(id))?;
                }
                let category = self.category_mut(id)?;
                if let Some(n) = new_name {
                    category.name = n;
                }
                if let Some(w) = target_weight {
                    category.target_weight = w;
                }
                if let Some(t) = targeting {
                    category.targeting = t;
                }
            }
            Edit::RemoveCategory { id } => {
                self.category(id)?;
                if self.products.iter().any(|p| p.category == id) {
                    return Err(Error::Conflict(
                        "move or remove the category's products first".to_owned(),
                    ));
                }
                self.categories.retain(|c| c.id != id);
            }
            Edit::AddProduct {
                name: new_name,
                category,
                currency: code,
            } => {
                self.check_capacity(self.products.len())?;
                let new_name = name(&new_name, "product")?;
                let code = currency(&code)?;
                self.category(category)?;
                self.ensure_product_name_free(&new_name, None)?;
                let id = ProductId(self.take_id());
                self.products.push(Product {
                    id,
                    name: new_name,
                    category,
                    currency: code,
                    weight: None,
                    aliases: Vec::new(),
                });
            }
            Edit::UpdateProduct {
                id,
                name: new_name,
                category,
                currency: code,
            } => {
                let new_name = new_name.map(|n| name(&n, "product")).transpose()?;
                let code = code.map(|c| currency(&c)).transpose()?;
                self.product(id)?;
                if let Some(category) = category {
                    self.category(category)?;
                }
                if let Some(n) = &new_name {
                    self.ensure_product_name_free(n, Some(id))?;
                }
                let product = self.product_mut(id)?;
                if let Some(n) = new_name {
                    product.name = n;
                }
                if let Some(c) = category {
                    product.category = c;
                }
                if let Some(c) = code {
                    product.currency = c;
                }
            }
            Edit::SetProductWeight { id, weight } => {
                let weight = weight.map(|w| positive(w, "product weight")).transpose()?;
                self.product_mut(id)?.weight = weight;
            }
            Edit::RemoveProduct { id } => {
                self.product(id)?;
                self.products.retain(|p| p.id != id);
                self.holdings.retain(|h| h.product != id);
                for account in &mut self.accounts {
                    if let Some(tradable) = &mut account.tradable {
                        tradable.retain(|p| *p != id);
                    }
                }
            }
            Edit::AddAlias {
                product,
                source,
                name: alias_name,
            } => {
                let alias = Alias {
                    source: name(&source, "alias source")?,
                    name: name(&alias_name, "alias")?,
                };
                self.product(product)?;
                if let Some(owner) = self
                    .products
                    .iter()
                    .find(|p| p.aliases.contains(&alias))
                    .map(|p| p.id)
                {
                    if owner == product {
                        return Ok(());
                    }
                    return Err(Error::Conflict(
                        "this name is already linked to another product".to_owned(),
                    ));
                }
                let product = self.product_mut(product)?;
                if product.aliases.len() >= MAX_ALIASES_PER_PRODUCT {
                    return Err(Error::Invalid(
                        "too many aliases for one product".to_owned(),
                    ));
                }
                product.aliases.push(alias);
            }
            Edit::RemoveAlias {
                product,
                source,
                name: alias_name,
            } => {
                let product = self.product_mut(product)?;
                product
                    .aliases
                    .retain(|a| !(a.source == source.trim() && a.name == alias_name.trim()));
            }
            Edit::AddAccount {
                name: new_name,
                kind,
            } => {
                self.check_capacity(self.accounts.len())?;
                let new_name = name(&new_name, "account")?;
                self.ensure_account_name_free(&new_name, None)?;
                let id = AccountId(self.take_id());
                self.accounts.push(Account {
                    id,
                    name: new_name,
                    kind,
                    // NISA accounts default to no selling: selling would
                    // forfeit tax-free room.
                    sellable: kind != AccountKind::Nisa,
                    tradable: None,
                    funding: if kind == AccountKind::Points {
                        Funding::Points
                    } else {
                        Funding::Cash
                    },
                    rounding_unit: Decimal::new(1000, 0),
                });
            }
            Edit::UpdateAccount {
                id,
                name: new_name,
                kind,
                sellable,
                funding,
                rounding_unit,
            } => {
                let new_name = new_name.map(|n| name(&n, "account")).transpose()?;
                let rounding_unit = rounding_unit
                    .map(|u| positive(u, "rounding unit"))
                    .transpose()?;
                self.account(id)?;
                if let Some(n) = &new_name {
                    self.ensure_account_name_free(n, Some(id))?;
                }
                let account = self.account_mut(id)?;
                if let Some(n) = new_name {
                    account.name = n;
                }
                if let Some(k) = kind {
                    account.kind = k;
                }
                if let Some(s) = sellable {
                    account.sellable = s;
                }
                if let Some(f) = funding {
                    account.funding = f;
                }
                if let Some(u) = rounding_unit {
                    account.rounding_unit = u;
                }
            }
            Edit::SetTradable { id, products } => {
                self.account(id)?;
                if let Some(list) = &products {
                    for product in list {
                        self.product(*product)?;
                    }
                }
                let products = products.map(|mut list| {
                    list.sort();
                    list.dedup();
                    list
                });
                self.account_mut(id)?.tradable = products;
            }
            Edit::RemoveAccount { id } => {
                self.account(id)?;
                self.accounts.retain(|a| a.id != id);
                self.holdings.retain(|h| h.account != id);
                if self.settings.target_account == Some(id) {
                    self.settings.target_account = None;
                }
            }
            Edit::SetHolding {
                account,
                product,
                value,
                quantity,
            } => {
                let value = non_negative(value, "holding value")?;
                let quantity = quantity.map(|q| non_negative(q, "quantity")).transpose()?;
                self.account(account)?;
                self.product(product)?;
                match self
                    .holdings
                    .iter_mut()
                    .find(|h| h.account == account && h.product == product)
                {
                    Some(holding) => {
                        holding.value = value;
                        holding.quantity = quantity;
                    }
                    None => {
                        self.check_capacity(self.holdings.len())?;
                        self.holdings.push(Holding {
                            account,
                            product,
                            value,
                            quantity,
                        });
                    }
                }
            }
            Edit::RemoveHolding { account, product } => {
                self.holdings
                    .retain(|h| !(h.account == account && h.product == product));
            }
            Edit::ReplaceHoldings { account, holdings } => {
                self.account(account)?;
                let mut seen = std::collections::BTreeSet::new();
                let mut next = Vec::with_capacity(holdings.len());
                for input in holdings {
                    self.product(input.product)?;
                    if !seen.insert(input.product) {
                        return Err(Error::Invalid(
                            "a product appears twice in the import".to_owned(),
                        ));
                    }
                    next.push(Holding {
                        account,
                        product: input.product,
                        value: non_negative(input.value, "holding value")?,
                        quantity: None,
                    });
                }
                let others = self
                    .holdings
                    .iter()
                    .filter(|h| h.account != account)
                    .count();
                if others + next.len() > MAX_ITEMS {
                    return Err(Error::Invalid("too many items".to_owned()));
                }
                self.holdings.retain(|h| h.account != account);
                self.holdings.extend(next);
            }
            Edit::SetExchangeRate { usd_jpy } => {
                self.settings.usd_jpy = usd_jpy
                    .map(|rate| positive(rate, "exchange rate"))
                    .transpose()?;
            }
            Edit::SetTolerances { category, product } => {
                let category = fraction(category, "category tolerance")?;
                let product = fraction(product, "product tolerance")?;
                self.settings.category_tolerance = category;
                self.settings.product_tolerance = product;
            }
            Edit::SetTargetAccount { account } => {
                if let Some(account) = account {
                    self.account(account)?;
                }
                self.settings.target_account = account;
            }
        }
        Ok(())
    }

    fn take_id(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id += 1;
        id
    }

    fn check_capacity(&self, current: usize) -> Result<(), Error> {
        if current >= MAX_ITEMS {
            Err(Error::Invalid("too many items".to_owned()))
        } else {
            Ok(())
        }
    }

    fn category(&self, id: CategoryId) -> Result<&Category, Error> {
        self.categories
            .iter()
            .find(|c| c.id == id)
            .ok_or(Error::NotFound("category"))
    }

    fn category_mut(&mut self, id: CategoryId) -> Result<&mut Category, Error> {
        self.categories
            .iter_mut()
            .find(|c| c.id == id)
            .ok_or(Error::NotFound("category"))
    }

    fn product(&self, id: ProductId) -> Result<&Product, Error> {
        self.products
            .iter()
            .find(|p| p.id == id)
            .ok_or(Error::NotFound("product"))
    }

    fn product_mut(&mut self, id: ProductId) -> Result<&mut Product, Error> {
        self.products
            .iter_mut()
            .find(|p| p.id == id)
            .ok_or(Error::NotFound("product"))
    }

    fn account(&self, id: AccountId) -> Result<&Account, Error> {
        self.accounts
            .iter()
            .find(|a| a.id == id)
            .ok_or(Error::NotFound("account"))
    }

    fn account_mut(&mut self, id: AccountId) -> Result<&mut Account, Error> {
        self.accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or(Error::NotFound("account"))
    }

    fn ensure_category_name_free(
        &self,
        name: &str,
        except: Option<CategoryId>,
    ) -> Result<(), Error> {
        if self
            .categories
            .iter()
            .any(|c| c.name == name && Some(c.id) != except)
        {
            return Err(Error::Conflict(
                "a category with this name exists".to_owned(),
            ));
        }
        Ok(())
    }

    fn ensure_product_name_free(&self, name: &str, except: Option<ProductId>) -> Result<(), Error> {
        if self
            .products
            .iter()
            .any(|p| p.name == name && Some(p.id) != except)
        {
            return Err(Error::Conflict(
                "a product with this name exists".to_owned(),
            ));
        }
        Ok(())
    }

    fn ensure_account_name_free(&self, name: &str, except: Option<AccountId>) -> Result<(), Error> {
        if self
            .accounts
            .iter()
            .any(|a| a.name == name && Some(a.id) != except)
        {
            return Err(Error::Conflict(
                "an account with this name exists".to_owned(),
            ));
        }
        Ok(())
    }
}
