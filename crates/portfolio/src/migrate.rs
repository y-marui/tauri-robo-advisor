//! Converts a stored file of an older schema version to the current layout before it is read.
//! The file is not trusted: anything unexpected is left as it is and the integrity check rejects
//! it.

use std::collections::BTreeMap;

use rust_decimal::Decimal;
use serde_json::{Value, json};

use crate::{Error, MAX_PRODUCT_WEIGHT, MIN_SCHEMA_VERSION, Portfolio};

/// Reads a stored file: parses it, converts an older supported version to the current layout
/// (the version number itself is raised by the caller after the integrity check), and returns the
/// portfolio. A version newer than this app's is not touched (the integrity check refuses it).
pub fn parse(bytes: &[u8]) -> Result<Portfolio, Error> {
    let mut value: Value = serde_json::from_slice(bytes)
        .map_err(|_| Error::Corrupt("the file is not valid".to_owned()))?;
    to_current(&mut value)?;
    serde_json::from_value(value).map_err(|_| Error::Corrupt("the file is not valid".to_owned()))
}

fn to_current(value: &mut Value) -> Result<(), Error> {
    let version = value.get("schema_version").and_then(Value::as_u64);
    if version.is_some_and(|v| (u64::from(MIN_SCHEMA_VERSION)..14).contains(&v)) {
        account_kind_to_record_type(value);
    }
    if version.is_some_and(|v| (u64::from(MIN_SCHEMA_VERSION)..17).contains(&v))
        && let Some(settings) = value.get_mut("settings").and_then(Value::as_object_mut)
    {
        // Version 17: the rebalancing account is gone.
        settings.remove("target_account");
    }
    if version.is_some_and(|v| (u64::from(MIN_SCHEMA_VERSION)..19).contains(&v)) {
        tradable_into_excluded(value);
    }
    if version.is_some_and(|v| (u64::from(MIN_SCHEMA_VERSION)..20).contains(&v)) {
        targeting_into_product_shares(value)?;
    }
    // Version 15 (the evaluation column of a mapping) needs no conversion: a mapping without it
    // reads the value as before and has nothing to check, and a preset recipe follows the shipped
    // preset. The version is raised by the caller; an older app refuses the new file.
    Ok(())
}

/// Version 14: the type moves from the account (`kind`) to its records. The type of a record is
/// worked out from the kind of its account: `taxable` is `specific` (the common case; general and
/// specific cannot be told apart), `nisa`, `deposit` and `crypto` are the same, `ideco` and
/// `points` become custom types (made only if an account has that kind), `other` leaves the
/// records without a type.
fn account_kind_to_record_type(value: &mut Value) {
    let mut next_id = value.get("next_id").and_then(Value::as_u64).unwrap_or(0);
    let mut custom: Vec<Value> = Vec::new();
    let mut types_by_account: Vec<(u64, String)> = Vec::new();
    let mut ideco: Option<String> = None;
    let mut points: Option<String> = None;
    if let Some(accounts) = value.get_mut("accounts").and_then(Value::as_array_mut) {
        for account in accounts {
            let Some(object) = account.as_object_mut() else {
                continue;
            };
            let kind = object.remove("kind");
            let (Some(id), Some(kind)) = (
                object.get("id").and_then(Value::as_u64),
                kind.as_ref().and_then(Value::as_str),
            ) else {
                continue;
            };
            let mut custom_type = |slot: &mut Option<String>, name: &str| {
                slot.get_or_insert_with(|| {
                    let id = format!("custom-{next_id}");
                    next_id += 1;
                    custom.push(json!({ "id": id, "name": name, "sellable": true }));
                    id
                })
                .clone()
            };
            let record_type = match kind {
                "taxable" => Some("specific".to_owned()),
                "nisa" => Some("nisa".to_owned()),
                "deposit" => Some("deposit".to_owned()),
                "crypto" => Some("crypto".to_owned()),
                "ideco" => Some(custom_type(&mut ideco, "iDeCo")),
                "points" => Some(custom_type(&mut points, "Points")),
                _ => None,
            };
            if let Some(t) = record_type {
                types_by_account.push((id, t));
            }
        }
    }
    if let Some(holdings) = value.get_mut("holdings").and_then(Value::as_array_mut) {
        for holding in holdings {
            let Some(object) = holding.as_object_mut() else {
                continue;
            };
            let account = object.get("account").and_then(Value::as_u64);
            if let Some((_, t)) = types_by_account.iter().find(|(a, _)| Some(*a) == account) {
                object.insert("account_type".to_owned(), Value::String(t.clone()));
            }
        }
    }
    if !custom.is_empty() {
        value["custom_types"] = Value::Array(custom);
        value["next_id"] = json!(next_id);
    }
}

/// Version 19: the buyable products (`tradable`, `null` or absent = any) and the left-out products
/// (`excluded`) become one list. A product takes part when it is buyable and not excluded, so the
/// new exclusion list is the old one plus every product outside a set `tradable`.
fn tradable_into_excluded(value: &mut Value) {
    let products: Vec<Value> = value
        .get("products")
        .and_then(Value::as_array)
        .map(|list| list.iter().filter_map(|p| p.get("id").cloned()).collect())
        .unwrap_or_default();
    let Some(accounts) = value.get_mut("accounts").and_then(Value::as_array_mut) else {
        return;
    };
    for account in accounts {
        let Some(object) = account.as_object_mut() else {
            continue;
        };
        let tradable = object.remove("tradable");
        let Some(tradable) = tradable.as_ref().and_then(Value::as_array) else {
            continue;
        };
        let mut excluded: Vec<Value> = match object.get("excluded") {
            None | Some(Value::Null) => Vec::new(),
            Some(Value::Array(list)) => list.clone(),
            // Malformed: leave it as it is so the file is rejected instead of changing what trades.
            Some(_) => continue,
        };
        for product in &products {
            if !tradable.contains(product) && !excluded.contains(product) {
                excluded.push(product.clone());
            }
        }
        excluded.sort_by_key(Value::as_u64);
        object.insert("excluded".to_owned(), Value::Array(excluded));
    }
}

/// Version 20: the targeting of the products of a category (`targeting`) and the weights of the
/// records (`weight`) become a setting per product in each of the four suggestion modes
/// (`product_shares`). Each mode gets what the category had, so nothing changes for the user: a
/// weighted category gives each product the sum of its record weights (a product without a weight
/// gets an equal share, which is what a missing weight asked for once it was refused as a problem),
/// a keep-current category keeps the current values, and a free or equal category stores nothing
/// (an equal share is the default). Then both old fields are dropped.
fn targeting_into_product_shares(value: &mut Value) -> Result<(), Error> {
    let mut weights: BTreeMap<u64, Decimal> = BTreeMap::new();
    if let Some(holdings) = value.get_mut("holdings").and_then(Value::as_array_mut) {
        for holding in holdings {
            let Some(object) = holding.as_object_mut() else {
                continue;
            };
            let product = object.get("product").and_then(Value::as_u64);
            // A malformed weight is refused, as it was before: dropping it would silently turn the
            // product into an equal share.
            let weight = match object.remove("weight") {
                None | Some(Value::Null) => None,
                Some(Value::String(text)) => match text.parse::<Decimal>() {
                    Ok(w) if w > Decimal::ZERO => Some(w),
                    _ => return Err(Error::Corrupt("the file is not valid".to_owned())),
                },
                Some(_) => return Err(Error::Corrupt("the file is not valid".to_owned())),
            };
            if let (Some(product), Some(weight)) = (product, weight) {
                let sum = weights.entry(product).or_default();
                *sum = sum.checked_add(weight).unwrap_or(Decimal::MAX);
            }
        }
    }
    let mut kinds: BTreeMap<u64, String> = BTreeMap::new();
    if let Some(categories) = value.get_mut("categories").and_then(Value::as_array_mut) {
        for category in categories {
            let Some(object) = category.as_object_mut() else {
                continue;
            };
            let targeting = object.remove("targeting");
            if let (Some(id), Some(kind)) = (
                object.get("id").and_then(Value::as_u64),
                targeting.as_ref().and_then(Value::as_str),
            ) {
                kinds.insert(id, kind.to_owned());
            }
        }
    }
    let mut entries: Vec<(u64, Value)> = Vec::new();
    let products = value
        .get("products")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for product in &products {
        let (Some(id), Some(category)) = (
            product.get("id").and_then(Value::as_u64),
            product.get("category").and_then(Value::as_u64),
        ) else {
            continue;
        };
        let share = match kinds.get(&category).map(String::as_str) {
            Some("keep_current") => Some(json!({ "kind": "keep_current" })),
            Some("weighted") => weights.get(&id).map(|w| {
                let w = (*w).min(MAX_PRODUCT_WEIGHT);
                json!({ "kind": "weighted", "weight": w.to_string() })
            }),
            _ => None,
        };
        if let Some(share) = share {
            entries.push((id, share));
        }
    }
    if entries.is_empty() {
        return Ok(());
    }
    entries.sort_by_key(|(id, _)| *id);
    let mut shares = Vec::new();
    for mode in [
        "fixed_add",
        "buy_only_to_target",
        "full_rebalance",
        "fixed_sell",
    ] {
        for (id, share) in &entries {
            shares.push(json!({ "mode": mode, "product": id, "share": share }));
        }
    }
    value["product_shares"] = Value::Array(shares);
    Ok(())
}
