//! Converts a stored file of an older schema version to the current layout before it is read.
//! The file is not trusted: anything unexpected is left as it is and the integrity check rejects
//! it.

use serde_json::{Value, json};

use crate::{Error, MIN_SCHEMA_VERSION, Portfolio};

/// Reads a stored file: parses it, converts an older supported version to the current layout
/// (the version number itself is raised by the caller after the integrity check), and returns the
/// portfolio. A version newer than this app's is not touched (the integrity check refuses it).
pub fn parse(bytes: &[u8]) -> Result<Portfolio, Error> {
    let mut value: Value = serde_json::from_slice(bytes)
        .map_err(|_| Error::Corrupt("the file is not valid".to_owned()))?;
    to_current(&mut value);
    serde_json::from_value(value).map_err(|_| Error::Corrupt("the file is not valid".to_owned()))
}

fn to_current(value: &mut Value) {
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
    // Version 15 (the evaluation column of a mapping) needs no conversion: a mapping without it
    // reads the value as before and has nothing to check, and a preset recipe follows the shipped
    // preset. The version is raised by the caller; an older app refuses the new file.
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
        let mut excluded: Vec<Value> = object
            .get("excluded")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        for product in &products {
            if !tradable.contains(product) && !excluded.contains(product) {
                excluded.push(product.clone());
            }
        }
        excluded.sort_by_key(Value::as_u64);
        object.insert("excluded".to_owned(), Value::Array(excluded));
    }
}
