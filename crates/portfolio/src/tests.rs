// All names and amounts below are made up; no real holdings belong in tests.

use rust_decimal::dec;

use crate::*;

fn apply(portfolio: &mut Portfolio, edit: Edit) {
    portfolio.apply(edit).unwrap();
}

fn category(portfolio: &mut Portfolio, name: &str, weight: rust_decimal::Decimal) -> CategoryId {
    apply(
        portfolio,
        Edit::AddCategory {
            name: name.to_owned(),
            target_weight: weight,
        },
    );
    portfolio.categories.last().unwrap().id
}

fn product(portfolio: &mut Portfolio, name: &str, category: CategoryId) -> ProductId {
    apply(
        portfolio,
        Edit::AddProduct {
            name: name.to_owned(),
            category,
            currency: "JPY".to_owned(),
        },
    );
    portfolio.products.last().unwrap().id
}

fn account(portfolio: &mut Portfolio, name: &str) -> AccountId {
    apply(
        portfolio,
        Edit::AddAccount {
            name: name.to_owned(),
        },
    );
    portfolio.accounts.last().unwrap().id
}

fn sample() -> (Portfolio, CategoryId, ProductId, AccountId) {
    let mut p = Portfolio::default();
    let c = category(&mut p, "Category A", dec!(1));
    let prod = product(&mut p, "Fund X", c);
    let acc = account(&mut p, "Account 1");
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(1000),
            quantity: None,
        },
    );
    (p, c, prod, acc)
}

#[test]
fn ids_are_unique_and_never_reused_after_removal() {
    let mut p = Portfolio::default();
    let a = category(&mut p, "A", dec!(0.5));
    apply(&mut p, Edit::RemoveCategory { id: a });
    let b = category(&mut p, "B", dec!(0.5));

    assert_ne!(a, b);
    p.validate_integrity().unwrap();
}

#[test]
fn a_rejected_edit_leaves_the_portfolio_unchanged() {
    let (mut p, c, _, _) = sample();
    let before = p.clone();

    let results = [
        p.apply(Edit::AddCategory {
            name: "  ".to_owned(),
            target_weight: dec!(0.1),
        }),
        p.apply(Edit::AddCategory {
            name: "New".to_owned(),
            target_weight: dec!(1.5),
        }),
        p.apply(Edit::UpdateCategory {
            id: c,
            name: Some("Renamed".to_owned()),
            target_weight: Some(dec!(-1)),
        }),
        p.apply(Edit::AddProduct {
            name: "P".to_owned(),
            category: CategoryId(999),
            currency: "JPY".to_owned(),
        }),
    ];

    assert!(results.iter().all(Result::is_err));
    assert_eq!(p, before);
}

#[test]
fn names_are_trimmed_unique_and_free_of_control_characters() {
    let mut p = Portfolio::default();
    category(&mut p, "  Stocks  ", dec!(0.5));

    assert_eq!(p.categories[0].name, "Stocks");
    assert!(matches!(
        p.apply(Edit::AddCategory {
            name: "Stocks".to_owned(),
            target_weight: dec!(0.5)
        }),
        Err(Error::Conflict(_))
    ));
    assert!(matches!(
        p.apply(Edit::AddCategory {
            name: "bad\nname".to_owned(),
            target_weight: dec!(0.5)
        }),
        Err(Error::Invalid(_))
    ));
    assert!(matches!(
        p.apply(Edit::AddCategory {
            name: "x".repeat(101),
            target_weight: dec!(0.5)
        }),
        Err(Error::Invalid(_))
    ));
}

#[test]
fn a_category_with_products_cannot_be_removed() {
    let (mut p, c, _, _) = sample();

    assert!(matches!(
        p.apply(Edit::RemoveCategory { id: c }),
        Err(Error::Conflict(_))
    ));
}

#[test]
fn removing_an_account_removes_its_holdings() {
    let (mut p, _, _, acc) = sample();

    apply(&mut p, Edit::RemoveAccount { id: acc });

    assert!(p.holdings.is_empty());
    p.validate_integrity().unwrap();
}

#[test]
fn aliases_are_idempotent_per_product_and_exclusive_across_products() {
    let (mut p, c, prod, _) = sample();
    let other = product(&mut p, "Fund Y", c);
    let add = |product, name: &str| Edit::AddAlias {
        product,
        source: "Broker 1".to_owned(),
        name: name.to_owned(),
    };

    apply(&mut p, add(prod, "FUND X (A)"));
    apply(&mut p, add(prod, "FUND X (A)"));

    assert_eq!(p.products[0].aliases.len(), 1);
    assert!(matches!(
        p.apply(add(other, "FUND X (A)")),
        Err(Error::Conflict(_))
    ));

    apply(
        &mut p,
        Edit::RemoveAlias {
            product: prod,
            source: "Broker 1".to_owned(),
            name: "FUND X (A)".to_owned(),
        },
    );
    assert!(p.products[0].aliases.is_empty());
    apply(&mut p, add(other, "FUND X (A)"));
}

#[test]
fn set_holding_updates_in_place_and_rejects_negative_values() {
    let (mut p, _, prod, acc) = sample();

    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(2500),
            quantity: Some(dec!(3)),
        },
    );

    assert_eq!(p.holdings.len(), 1);
    assert_eq!(p.holdings[0].value, dec!(2500));
    assert!(
        p.apply(Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(-1),
            quantity: None
        })
        .is_err()
    );
    apply(
        &mut p,
        Edit::RemoveHolding {
            account_type: None,
            account: acc,
            product: prod,
        },
    );
    assert!(p.holdings.is_empty());
}

#[test]
fn set_excluded_checks_products_and_follows_product_removal() {
    let (mut p, _, prod, acc) = sample();

    assert!(
        p.apply(Edit::SetExcluded {
            id: acc,
            products: vec![ProductId(999)]
        })
        .is_err()
    );
    apply(
        &mut p,
        Edit::SetExcluded {
            id: acc,
            products: vec![prod, prod],
        },
    );
    assert_eq!(p.accounts[0].excluded, vec![prod]);
    p.validate_integrity().unwrap();

    apply(&mut p, Edit::RemoveProduct { id: prod });
    assert!(p.accounts[0].excluded.is_empty());
}

#[test]
fn strategies_are_validated_and_follow_product_changes() {
    let (mut p, _, prod, acc) = sample();
    let item = |product, ratio| StrategyItem { product, ratio };
    let set = |p: &mut Portfolio, items: Vec<StrategyItem>| {
        p.apply(Edit::SetStrategy {
            id: acc,
            strategy: Strategy::Products { items },
        })
    };

    assert!(set(&mut p, vec![]).is_err());
    assert!(set(&mut p, vec![item(ProductId(999), None)]).is_err());
    assert!(set(&mut p, vec![item(prod, None), item(prod, None)]).is_err());
    assert!(set(&mut p, vec![item(prod, Some(dec!(0)))]).is_err());
    assert!(set(&mut p, vec![item(prod, Some(dec!(-1)))]).is_err());
    assert!(set(&mut p, vec![item(prod, Some(dec!(0.0000001)))]).is_err());
    assert!(set(&mut p, vec![item(prod, Some(dec!(2000000)))]).is_err());
    assert_eq!(p.accounts[0].strategy, Strategy::Portfolio);

    set(&mut p, vec![item(prod, Some(dec!(3)))]).unwrap();
    p.validate_integrity().unwrap();
    let json = serde_json::to_string(&p).unwrap();
    let back: Portfolio = serde_json::from_str(&json).unwrap();
    assert_eq!(back.accounts[0].strategy, p.accounts[0].strategy);

    // Back to the portfolio's targets, which are not stored.
    apply(
        &mut p,
        Edit::SetStrategy {
            id: acc,
            strategy: Strategy::Portfolio,
        },
    );
    assert!(!serde_json::to_string(&p).unwrap().contains("strategy"));

    // A removed product leaves the strategy, and an empty strategy falls back to the portfolio.
    set(&mut p, vec![item(prod, None)]).unwrap();
    apply(&mut p, Edit::RemoveProduct { id: prod });
    assert_eq!(p.accounts[0].strategy, Strategy::Portfolio);
}

#[test]
fn merging_products_merges_their_strategy_ratios() {
    let (mut p, cat, prod, acc) = sample();
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Second".to_owned(),
            category: cat,
            currency: "JPY".to_owned(),
        },
    );
    let second = p.products.last().unwrap().id;
    apply(
        &mut p,
        Edit::SetStrategy {
            id: acc,
            strategy: Strategy::Products {
                items: vec![
                    StrategyItem {
                        product: prod,
                        ratio: Some(dec!(3)),
                    },
                    StrategyItem {
                        product: second,
                        ratio: Some(dec!(2)),
                    },
                ],
            },
        },
    );
    apply(
        &mut p,
        Edit::MergeProducts {
            keep: prod,
            remove: second,
            on_conflict: MergeConflict::default(),
        },
    );
    assert_eq!(
        p.accounts[0].strategy,
        Strategy::Products {
            items: vec![StrategyItem {
                product: prod,
                ratio: Some(dec!(5)),
            }]
        }
    );
}

#[test]
fn merging_products_whose_strategy_ratios_would_exceed_the_limit_is_refused() {
    let (mut p, cat, prod, acc) = sample();
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Second".to_owned(),
            category: cat,
            currency: "JPY".to_owned(),
        },
    );
    let second = p.products.last().unwrap().id;
    let item = |product, ratio| StrategyItem {
        product,
        ratio: Some(ratio),
    };
    apply(
        &mut p,
        Edit::SetStrategy {
            id: acc,
            strategy: Strategy::Products {
                items: vec![item(prod, dec!(600000)), item(second, dec!(600000))],
            },
        },
    );
    let before = p.clone();
    assert!(
        p.apply(Edit::MergeProducts {
            keep: prod,
            remove: second,
            on_conflict: MergeConflict::default(),
        })
        .is_err()
    );
    assert_eq!(p, before);
    p.validate_integrity().unwrap();
}

#[test]
fn settings_are_range_checked() {
    let (mut p, _, _, _) = sample();

    assert!(
        p.apply(Edit::SetExchangeRate {
            usd_jpy: Some(dec!(0))
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::SetTolerances {
            category: dec!(2),
            product: dec!(0)
        })
        .is_err()
    );
    apply(
        &mut p,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150.5)),
        },
    );
    apply(
        &mut p,
        Edit::SetTolerances {
            category: dec!(0.02),
            product: dec!(0.01),
        },
    );

    assert_eq!(p.settings.usd_jpy, Some(dec!(150.5)));
    assert_eq!(p.settings.category_tolerance, dec!(0.02));
}

#[test]
fn issues_report_weights_that_do_not_sum_to_one() {
    let mut p = Portfolio::default();
    category(&mut p, "A", dec!(0.6));
    category(&mut p, "B", dec!(0.3));

    assert_eq!(
        p.issues(),
        vec![Issue::CategoryWeightsDoNotSumToOne { sum: dec!(0.9) }]
    );
}

#[test]
fn issues_accept_weights_with_small_rounding_error() {
    let mut p = Portfolio::default();
    category(&mut p, "A", dec!(0.3333));
    category(&mut p, "B", dec!(0.3333));
    category(&mut p, "C", dec!(0.3334));

    assert!(p.issues().is_empty());
}

#[test]
fn issues_report_a_missing_exchange_rate_only_for_held_foreign_products() {
    let (mut p, c, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddProduct {
            name: "US Fund".to_owned(),
            category: c,
            currency: "USD".to_owned(),
        },
    );
    let usd = p.products.last().unwrap().id;
    assert!(p.issues().is_empty());

    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: usd,
            value: dec!(10),
            quantity: None,
        },
    );
    assert_eq!(
        p.issues(),
        vec![Issue::MissingExchangeRate {
            currency: "USD".to_owned()
        }]
    );

    apply(
        &mut p,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150)),
        },
    );
    assert!(p.issues().is_empty());
}

#[test]
fn integrity_check_accepts_a_well_formed_portfolio_and_the_default() {
    let (p, ..) = sample();

    p.validate_integrity().unwrap();
    Portfolio::default().validate_integrity().unwrap();
}

#[test]
fn integrity_check_rejects_newer_schema_versions() {
    let p = Portfolio {
        schema_version: SCHEMA_VERSION + 1,
        ..Portfolio::default()
    };

    assert_eq!(
        p.validate_integrity(),
        Err(Error::UnsupportedVersion {
            found: SCHEMA_VERSION + 1
        })
    );
}

#[test]
fn integrity_check_rejects_dangling_references_and_duplicates() {
    let (good, ..) = sample();

    let mut p = good.clone();
    p.products[0].category = CategoryId(999);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.holdings[0].account = AccountId(999);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.holdings.push(p.holdings[0].clone());
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.products[0].id = ProductId(p.categories[0].id.0);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.next_id = 1;
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));
}

#[test]
fn integrity_check_rejects_out_of_range_values() {
    let (good, ..) = sample();

    let mut p = good.clone();
    p.categories[0].target_weight = dec!(2);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.holdings[0].value = dec!(-1);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good;
    p.accounts[0].rounding_unit = dec!(0);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));
}

#[test]
fn json_round_trip_keeps_amounts_as_exact_strings() {
    let (p, ..) = sample();

    let json = serde_json::to_string(&p).unwrap();
    let back: Portfolio = serde_json::from_str(&json).unwrap();

    assert_eq!(back, p);
    assert!(json.contains(r#""value":"1000""#));
}

#[test]
fn json_with_missing_optional_sections_uses_defaults() {
    let json = format!(r#"{{"schema_version":{SCHEMA_VERSION},"next_id":1}}"#);

    let p: Portfolio = serde_json::from_str(&json).unwrap();

    assert_eq!(p, Portfolio::default());
}

#[test]
fn edits_deserialize_from_the_tagged_json_the_frontend_sends() {
    let json = r#"{"type":"set_holding","account":3,"product":2,"value":"1500.5","quantity":null}"#;

    let edit: Edit = serde_json::from_str(json).unwrap();

    assert_eq!(
        edit,
        Edit::SetHolding {
            account_type: None,
            account: AccountId(3),
            product: ProductId(2),
            value: dec!(1500.5),
            quantity: None,
        }
    );
    assert!(serde_json::from_str::<Edit>(r#"{"type":"drop_everything"}"#).is_err());
}

#[test]
fn replace_holdings_overwrites_only_that_account() {
    let (mut p, cat, prod, acc) = sample();
    let other = account(&mut p, "Other");
    let second = product(&mut p, "Second", cat);
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(100),
            quantity: Some(dec!(1)),
        },
    );
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: other,
            product: prod,
            value: dec!(7),
            quantity: None,
        },
    );

    apply(
        &mut p,
        Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![HoldingInput {
                account_type: None,
                product: second,
                value: dec!(55),
                quantity: None,
                price: None,
                price_unit: 1,
            }],
        },
    );

    let mine: Vec<_> = p.holdings.iter().filter(|h| h.account == acc).collect();
    assert_eq!(mine.len(), 1);
    assert_eq!(mine[0].product, second);
    assert_eq!(mine[0].value, dec!(55));
    assert_eq!(mine[0].quantity, None);
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account == other && h.value == dec!(7))
    );
}

#[test]
fn replace_holdings_rejects_bad_input_and_changes_nothing() {
    let (mut p, _, prod, acc) = sample();
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(100),
            quantity: None,
        },
    );
    let before = p.clone();
    let input = |value| HoldingInput {
        account_type: None,
        product: prod,
        value,
        quantity: None,
        price: None,
        price_unit: 1,
    };

    assert!(
        p.apply(Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![input(dec!(1)), input(dec!(2))],
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![input(dec!(-1))],
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::ReplaceHoldings {
            types: None,
            account: AccountId(9999),
            holdings: vec![],
        })
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn plan_import_matches_only_confirmed_aliases_in_the_right_currency() {
    let (mut p, cat, prod, _) = sample();
    apply(
        &mut p,
        Edit::AddAlias {
            product: prod,
            source: "bitflyer".to_owned(),
            name: "BTC".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Dollar".to_owned(),
            category: cat,
            currency: "USD".to_owned(),
        },
    );
    let dollar = p.products.last().unwrap().id;
    apply(
        &mut p,
        Edit::AddAlias {
            product: dollar,
            source: "bitflyer".to_owned(),
            name: "USDT".to_owned(),
        },
    );
    let row = |name: &str| ImportRow {
        account_type: None,
        name: name.to_owned(),
        value: dec!(10),
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    };

    let plan = p.plan_import("bitflyer", "JPY", &[row("BTC"), row("ETH"), row("USDT")]);

    assert_eq!(plan.matched.len(), 1);
    assert_eq!(plan.matched[0].product, prod);
    assert_eq!(plan.matched[0].names, vec!["BTC"]);
    assert_eq!(plan.unmatched, vec![row("ETH")]);
    assert_eq!(plan.wrong_currency, vec![row("USDT")]);
    // Another source's alias does not count.
    assert_eq!(
        p.plan_import("other", "JPY", &[row("BTC")]).unmatched.len(),
        1
    );
}

#[test]
fn plan_import_suggests_products_with_the_same_or_a_similar_name() {
    let (mut p, cat, _, _) = sample();
    for name in ["AAA", "Poly Gon", "B", "AAB Coin"] {
        apply(
            &mut p,
            Edit::AddProduct {
                name: name.to_owned(),
                category: cat,
                currency: "JPY".to_owned(),
            },
        );
    }
    apply(
        &mut p,
        Edit::AddProduct {
            name: "ZZZ".to_owned(),
            category: cat,
            currency: "USD".to_owned(),
        },
    );
    let id = |name: &str| p.products.iter().find(|x| x.name == name).unwrap().id;
    let row = |name: &str| ImportRow {
        account_type: None,
        name: name.to_owned(),
        value: dec!(10),
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    };
    let rows = [
        row(" aaa "),
        row("poly-gon"),
        row("AAB"),
        row("BBB"),
        row("ZZZ"),
        row("QQQ"),
    ];

    let plan = p.plan_import("src", "JPY", &rows);

    assert_eq!(plan.unmatched.len(), 6);
    let got = |name: &str| plan.suggestions.iter().find(|s| s.name == name);
    // Exact: trimmed, ignoring case; the name is trimmed in the plan as in the row.
    assert_eq!(got("aaa").unwrap().exact, Some(id("AAA")));
    assert_eq!(got("aaa").unwrap().similar, None);
    // Similar: the same ignoring spaces and symbols; one contains the other.
    assert_eq!(got("poly-gon").unwrap().similar, Some(id("Poly Gon")));
    assert_eq!(got("AAB").unwrap().similar, Some(id("AAB Coin")));
    // A single letter hints nothing ("B" is inside "BBB"); another currency is not offered.
    assert!(got("BBB").is_none());
    assert!(got("ZZZ").is_none());
    assert!(got("QQQ").is_none());
    // Without a currency every product can be linked.
    let any = p.plan_import("src", "", &[row("ZZZ")]);
    assert_eq!(any.suggestions[0].exact, Some(id("ZZZ")));
}

#[test]
fn plan_import_prefers_an_exact_name_to_a_similar_one() {
    let (mut p, cat, _, _) = sample();
    for name in ["AAA Plus", "AAA"] {
        apply(
            &mut p,
            Edit::AddProduct {
                name: name.to_owned(),
                category: cat,
                currency: "JPY".to_owned(),
            },
        );
    }
    let exact = p.products.iter().find(|x| x.name == "AAA").unwrap().id;
    let row = ImportRow {
        account_type: None,
        name: "aaa".to_owned(),
        value: dec!(1),
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    };
    let plan = p.plan_import("src", "JPY", &[row]);
    assert_eq!(plan.suggestions[0].exact, Some(exact));
    assert_eq!(plan.suggestions[0].similar, None);
}

#[test]
fn plan_import_adds_up_names_linked_to_the_same_product() {
    let (mut p, _, prod, _) = sample();
    for name in ["BTC", "ETH"] {
        apply(
            &mut p,
            Edit::AddAlias {
                product: prod,
                source: "bitflyer".to_owned(),
                name: name.to_owned(),
            },
        );
    }
    let row = |name: &str, value| ImportRow {
        account_type: None,
        name: name.to_owned(),
        value,
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    };

    let plan = p.plan_import(
        "bitflyer",
        "JPY",
        &[row("BTC", dec!(10)), row("ETH", dec!(5))],
    );

    assert_eq!(plan.matched.len(), 1);
    assert_eq!(plan.matched[0].names, vec!["BTC", "ETH"]);
    assert_eq!(plan.matched[0].value, dec!(15));
}

#[test]
fn import_mappings_are_saved_per_source_and_replaced() {
    let mut p = Portfolio::default();
    let set = |source: &str, name_column, value_column| Edit::SetImportMapping {
        source: source.to_owned(),
        name_column,
        value_column,
        price_column: None,
        price_unit: 1,
        header_rows: 1,
        rows_per_record: 1,
        name_row: 0,
        value_row: 0,
        name_part: None,
        value_part: None,
        price_part: None,
        evaluation_column: None,
        evaluation_part: None,
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
        kind_column: None,
        kind_rules: Vec::new(),
    };

    apply(&mut p, set(" SBI ", 0, 5));
    apply(&mut p, set("Monex", 1, 2));
    apply(&mut p, set("SBI", 0, 6));

    assert_eq!(p.import_mappings.len(), 2);
    assert_eq!(p.import_mappings[0].source, "SBI");
    assert_eq!(p.import_mappings[0].value_column, 6);

    apply(
        &mut p,
        Edit::RemoveImportMapping {
            source: "Monex".to_owned(),
        },
    );
    assert_eq!(p.import_mappings.len(), 1);
}

#[test]
fn import_mappings_reject_bad_columns_and_leave_the_portfolio_unchanged() {
    let mut p = Portfolio::default();
    let before = p.clone();
    #[allow(clippy::too_many_arguments)]
    let bad = |source: &str,
               name_column,
               value_column,
               price_column,
               price_unit,
               rows_per_record,
               name_row,
               value_row| Edit::SetImportMapping {
        source: source.to_owned(),
        name_column,
        value_column,
        price_column,
        price_unit,
        header_rows: 0,
        rows_per_record,
        name_row,
        value_row,
        name_part: None,
        value_part: None,
        price_part: None,
        evaluation_column: None,
        evaluation_part: None,
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
        kind_column: None,
        kind_rules: Vec::new(),
    };

    for edit in [
        bad("SBI", 3, 3, None, 1, 1, 0, 0),
        bad("SBI", 0, 5000, None, 1, 1, 0, 0),
        bad("  ", 0, 1, None, 1, 1, 0, 0),
        // a record of zero or too many rows, or a row outside the record
        bad("SBI", 0, 1, None, 1, 0, 0, 0),
        bad("SBI", 0, 1, None, 1, 11, 0, 0),
        bad("SBI", 0, 1, None, 1, 2, 2, 0),
        bad("SBI", 0, 1, None, 1, 2, 0, 5),
        // the quantity and the price in one column; a zero or huge unit
        bad("SBI", 0, 1, Some(1), 1, 1, 0, 0),
        bad("SBI", 0, 1, Some(2), 0, 1, 0, 0),
        bad("SBI", 0, 1, Some(2), 2_000_000_000, 1, 0, 0),
    ] {
        assert!(p.apply(edit).is_err());
    }
    assert_eq!(p, before);
}

#[test]
fn a_two_row_record_may_keep_the_name_and_the_value_in_one_column() {
    let mut p = Portfolio::default();

    apply(
        &mut p,
        Edit::SetImportMapping {
            source: "SBI".to_owned(),
            name_column: 0,
            value_column: 0,
            price_column: Some(2),
            price_unit: 10_000,
            header_rows: 2,
            rows_per_record: 2,
            name_row: 0,
            value_row: 1,
            name_part: None,
            value_part: None,
            price_part: None,
            evaluation_column: None,
            evaluation_part: None,
            loose_numbers: false,
            quantity_column: None,
            quantity_part: None,
            max_records: None,
            kind_column: None,
            kind_rules: Vec::new(),
        },
    );

    let saved = &p.import_mappings[0];
    assert_eq!(saved.price_column, Some(2));
    assert_eq!(saved.price_unit, 10_000);
    assert_eq!(saved.rows_per_record, 2);
}

#[test]
fn a_portfolio_file_with_an_old_mapping_still_loads() {
    let json = r#"{"source":"SBI","name_column":0,"value_column":1,"header_rows":1}"#;

    let mapping: ImportMapping = serde_json::from_str(json).unwrap();

    assert_eq!(mapping.rows_per_record, 1);
    assert_eq!(mapping.price_unit, 1);
    assert_eq!(mapping.price_column, None);
}

// ---- recipes ---------------------------------------------------------------

fn mapping_for(source: &str) -> ImportMapping {
    ImportMapping {
        source: source.to_owned(),
        name_column: 0,
        value_column: 0,
        price_column: Some(2),
        price_unit: 1,
        header_rows: 2,
        rows_per_record: 2,
        name_row: 0,
        value_row: 1,
        name_part: None,
        value_part: None,
        price_part: None,
        evaluation_column: None,
        evaluation_part: None,
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
        kind_column: None,
        kind_rules: Vec::new(),
    }
}

fn recipe_for(account: AccountId) -> Recipe {
    Recipe {
        type_rules: Vec::new(),
        name: " Domestic stocks ".to_owned(),
        links: vec![
            FetchLink {
                text: " Account ".to_owned(),
                reference: None,
                menu: None,
                heading: None,
            },
            FetchLink {
                text: "Yen account".to_owned(),
                reference: Some("/a/list".to_owned()),
                menu: None,
                heading: None,
            },
        ],
        table_starts_with: " Stocks (".to_owned(),
        currency: "JPY".to_owned(),
        account,
        mapping: mapping_for(" SBI "),
        all_tables: false,
        following_tables: false,
        cards: None,
        start_site: "example.test".to_owned(),
        allowed_sites: Vec::new(),
        preset: None,
    }
}

#[test]
fn a_recipe_is_saved_trimmed_and_replaced_by_name() {
    let (mut p, _, _, acc) = sample();

    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    let mut again = recipe_for(acc);
    again.table_starts_with = "Funds".to_owned();
    apply(&mut p, Edit::SetRecipe { recipe: again });

    assert_eq!(p.recipes.len(), 1);
    let saved = &p.recipes[0];
    assert_eq!(saved.name, "Domestic stocks");
    assert_eq!(saved.links[0].text, "Account");
    assert_eq!(saved.table_starts_with, "Funds");
    assert_eq!(saved.mapping.source, "SBI");
    p.validate_integrity().unwrap();
}

#[test]
fn bad_recipes_are_rejected_and_change_nothing() {
    let (mut p, _, _, acc) = sample();
    let before = p.clone();
    let mut with = |change: &dyn Fn(&mut Recipe)| {
        let mut r = recipe_for(acc);
        change(&mut r);
        p.apply(Edit::SetRecipe { recipe: r }).is_err()
    };

    assert!(with(&|r| r.name = "  ".to_owned()));
    assert!(with(&|r| r.links = vec![
        FetchLink {
            text: "x".repeat(61),
            reference: None,
            menu: None,
            heading: None,
        };
        1
    ]));
    assert!(with(&|r| r.links = vec![
        FetchLink {
            text: "".to_owned(),
            reference: None,
            menu: None,
            heading: None,
        };
        1
    ]));
    assert!(with(&|r| r.links = vec![
        FetchLink {
            text: "a".to_owned(),
            reference: None,
            menu: None,
            heading: None,
        };
        6
    ]));
    assert!(with(&|r| r.links[1].reference = Some("x".repeat(501))));
    assert!(with(&|r| r.table_starts_with = " ".to_owned()));
    assert!(with(&|r| r.currency = "jpy".to_owned()));
    assert!(with(&|r| r.account = AccountId(9999)));
    assert!(with(&|r| r.mapping.rows_per_record = 0));
    assert_eq!(p, before);
}

#[test]
fn removing_an_account_removes_its_recipes_and_a_recipe_can_be_removed() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    apply(
        &mut p,
        Edit::RemoveRecipe {
            name: " Domestic stocks ".to_owned(),
        },
    );
    assert!(p.recipes.is_empty());

    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    apply(&mut p, Edit::RemoveAccount { id: acc });
    assert!(p.recipes.is_empty());
}

#[test]
fn a_file_with_a_recipe_for_a_missing_account_is_not_valid() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    p.recipes[0].account = AccountId(9999);

    assert!(p.validate_integrity().is_err());
}

#[test]
fn a_file_with_duplicate_or_invalid_recipes_and_mappings_is_not_valid() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    p.validate_integrity().unwrap();

    let mut duplicate = p.clone();
    duplicate.recipes.push(duplicate.recipes[0].clone());
    assert!(
        duplicate.validate_integrity().is_err(),
        "two recipes with one name"
    );

    let mut bad_currency = p.clone();
    bad_currency.recipes[0].currency = "jpy".to_owned();
    assert!(bad_currency.validate_integrity().is_err());

    let mut bad_mapping = p.clone();
    bad_mapping.recipes[0].mapping.rows_per_record = 0;
    assert!(bad_mapping.validate_integrity().is_err());

    let mut untrimmed = p.clone();
    untrimmed.recipes[0].name = " padded ".to_owned();
    assert!(
        untrimmed.validate_integrity().is_err(),
        "names are stored trimmed"
    );

    let mut mappings = p.clone();
    apply(
        &mut mappings,
        Edit::SetImportMapping {
            source: "SBI".to_owned(),
            name_column: 0,
            value_column: 1,
            price_column: None,
            price_unit: 1,
            header_rows: 0,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            name_part: None,
            value_part: None,
            price_part: None,
            evaluation_column: None,
            evaluation_part: None,
            loose_numbers: false,
            quantity_column: None,
            quantity_part: None,
            max_records: None,
            kind_column: None,
            kind_rules: Vec::new(),
        },
    );
    mappings.validate_integrity().unwrap();
    mappings
        .import_mappings
        .push(mappings.import_mappings[0].clone());
    assert!(
        mappings.validate_integrity().is_err(),
        "two mappings for one source"
    );
}

#[test]
fn every_accepted_edit_raises_the_revision_and_a_rejected_one_does_not() {
    let mut p = Portfolio::default();
    assert_eq!(p.revision, 0);

    apply(
        &mut p,
        Edit::AddAccount {
            name: "A".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddAccount {
            name: "B".to_owned(),
        },
    );
    assert_eq!(p.revision, 2);

    assert!(
        p.apply(Edit::AddAccount {
            name: "A".to_owned(),
        })
        .is_err()
    );
    assert_eq!(p.revision, 2);
}

#[test]
fn a_file_without_a_revision_loads_as_revision_zero() {
    let json = r#"{"schema_version":1,"next_id":1}"#;

    let p: Portfolio = serde_json::from_str(json).unwrap();

    assert_eq!(p.revision, 0);
}

#[test]
fn the_parts_of_a_mapping_are_limited_and_old_files_get_defaults() {
    let mut p = Portfolio::default();
    let mut mapping = mapping_for("SBI");
    mapping.name_part = Some(1);
    mapping.value_part = Some(0);
    mapping.loose_numbers = true;
    apply(&mut p, set_mapping(mapping.clone()));
    assert_eq!(p.import_mappings[0].name_part, Some(1));
    assert!(p.import_mappings[0].loose_numbers);

    mapping.price_part = Some(500);
    assert!(
        p.apply(set_mapping(mapping)).is_err(),
        "a part number that is too large"
    );

    let old: ImportMapping = serde_json::from_str(
        r#"{"source":"SBI","name_column":0,"value_column":1,"header_rows":1}"#,
    )
    .unwrap();
    assert_eq!(old.name_part, None);
    assert!(!old.loose_numbers);
}

fn set_mapping(m: ImportMapping) -> Edit {
    Edit::SetImportMapping {
        source: m.source,
        name_column: m.name_column,
        value_column: m.value_column,
        price_column: m.price_column,
        price_unit: m.price_unit,
        header_rows: m.header_rows,
        rows_per_record: m.rows_per_record,
        name_row: m.name_row,
        value_row: m.value_row,
        name_part: m.name_part,
        value_part: m.value_part,
        price_part: m.price_part,
        evaluation_column: None,
        evaluation_part: None,
        loose_numbers: m.loose_numbers,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
        kind_column: m.kind_column,
        kind_rules: m.kind_rules,
    }
}

#[test]
fn quantity_and_price_follow_the_rows_linked_to_one_product() {
    let (mut p, _, prod, _) = sample();
    for name in ["A", "B", "C"] {
        apply(
            &mut p,
            Edit::AddAlias {
                product: prod,
                source: "src".to_owned(),
                name: name.to_owned(),
            },
        );
    }
    let row = |name: &str, quantity, price| ImportRow {
        account_type: None,
        name: name.to_owned(),
        value: dec!(100),
        quantity,
        price,
        price_unit: 1,
        computed: None,
    };

    // Same price: the quantities add up and the price stays.
    let same = p.plan_import(
        "src",
        "JPY",
        &[
            row("A", Some(dec!(2)), Some(dec!(50))),
            row("B", Some(dec!(3)), Some(dec!(50))),
        ],
    );
    assert_eq!(same.matched[0].quantity, Some(dec!(5)));
    assert_eq!(same.matched[0].price, Some(dec!(50)));

    // Different prices: there is no single price.
    let different = p.plan_import(
        "src",
        "JPY",
        &[
            row("A", Some(dec!(2)), Some(dec!(50))),
            row("B", Some(dec!(3)), Some(dec!(60))),
        ],
    );
    assert_eq!(different.matched[0].quantity, Some(dec!(5)));
    assert_eq!(different.matched[0].price, None);

    // A row without a quantity makes the total quantity unknown.
    let unknown = p.plan_import(
        "src",
        "JPY",
        &[
            row("A", Some(dec!(2)), Some(dec!(50))),
            row("C", None, Some(dec!(50))),
        ],
    );
    assert_eq!(unknown.matched[0].quantity, None);
}

#[test]
fn an_empty_import_currency_takes_the_products_own_currency() {
    let (mut p, cat, _, _) = sample();
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Dollar Fund".to_owned(),
            category: cat,
            currency: "USD".to_owned(),
        },
    );
    let dollar = p.products.last().unwrap().id;
    apply(
        &mut p,
        Edit::AddAlias {
            product: dollar,
            source: "src".to_owned(),
            name: "D".to_owned(),
        },
    );
    let rows = [ImportRow {
        account_type: None,
        name: "D".to_owned(),
        value: dec!(10),
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    }];

    assert_eq!(p.plan_import("src", "", &rows).matched.len(), 1);
    assert_eq!(p.plan_import("src", "USD", &rows).matched.len(), 1);
    assert_eq!(p.plan_import("src", "JPY", &rows).wrong_currency.len(), 1);
}

#[test]
fn replace_holdings_keeps_the_quantity_and_price_and_a_manual_edit_drops_the_price() {
    let (mut p, _, prod, acc) = sample();
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![HoldingInput {
                account_type: None,
                product: prod,
                value: dec!(125),
                quantity: Some(dec!(10)),
                price: Some(dec!(12.5)),
                price_unit: 1,
            }],
        },
    );
    let held = &p.holdings[0];
    assert_eq!(
        (held.quantity, held.price, held.price_unit),
        (Some(dec!(10)), Some(dec!(12.5)), 1)
    );

    // A bad price unit or a negative price is refused and changes nothing.
    let before = p.clone();
    for bad in [
        HoldingInput {
            account_type: None,
            product: prod,
            value: dec!(1),
            quantity: None,
            price: None,
            price_unit: 0,
        },
        HoldingInput {
            account_type: None,
            product: prod,
            value: dec!(1),
            quantity: None,
            price: Some(dec!(-1)),
            price_unit: 1,
        },
    ] {
        assert!(
            p.apply(Edit::ReplaceHoldings {
                types: None,
                account: acc,
                holdings: vec![bad],
            })
            .is_err()
        );
    }
    assert_eq!(p, before);

    // A value typed by hand no longer follows from the price.
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: None,
            account: acc,
            product: prod,
            value: dec!(200),
            quantity: Some(dec!(10)),
        },
    );
    assert_eq!(p.holdings[0].price, None);
}

#[test]
fn a_file_without_prices_loads_with_defaults() {
    let json = r#"{"account":1,"product":2,"value":"5"}"#;

    let holding: Holding = serde_json::from_str(json).unwrap();

    assert_eq!(
        (holding.quantity, holding.price, holding.price_unit),
        (None, None, 1)
    );
}

#[test]
fn a_recipe_needs_a_site_to_be_saved() {
    let (mut p, _, _, acc) = sample();
    let mut recipe = recipe_for(acc);
    recipe.start_site = " ".to_owned();
    let before = p.clone();
    let error = p.apply(Edit::SetRecipe { recipe }).unwrap_err();
    assert!(error.to_string().contains("site"), "{error}");
    assert_eq!(p, before);
}

#[test]
fn a_stored_recipe_without_a_site_still_loads_and_can_be_removed_or_moved() {
    let (mut p, _, _, a) = sample();
    let b = account(&mut p, "Account 2");
    let mut recipe = recipe_for(a);
    recipe.start_site = String::new();
    // What an older version could have stored.
    recipe.name = "Old".to_owned();
    p.recipes.push(recipe);
    assert!(
        p.issues()
            .iter()
            .all(|i| !format!("{i:?}").contains("recipe"))
    );
    let name = "Old".to_owned();
    apply(
        &mut p,
        Edit::SetRecipeAccount {
            name: name.clone(),
            account: b,
        },
    );
    apply(&mut p, Edit::RemoveRecipe { name });
    assert!(p.recipes.is_empty());
}

#[test]
fn a_recipe_may_leave_its_currency_empty() {
    let (mut p, _, _, acc) = sample();
    let mut recipe = recipe_for(acc);
    recipe.currency = " ".to_owned();

    apply(&mut p, Edit::SetRecipe { recipe });

    assert_eq!(p.recipes[0].currency, "");
}

#[test]
fn a_recipe_can_be_moved_to_another_account_but_not_onto_a_duplicate() {
    let (mut p, _, _, a) = sample();
    let b = account(&mut p, "Account 2");
    let add = |p: &mut Portfolio, account| {
        apply(
            p,
            Edit::AddPresetRecipe {
                preset: "sbi-funds".to_owned(),
                account,
            },
        )
    };
    add(&mut p, a);
    let name = p.recipes[0].name.clone();
    apply(
        &mut p,
        Edit::SetRecipeAccount {
            name: name.clone(),
            account: b,
        },
    );
    assert_eq!(p.recipes[0].account, b);
    assert_eq!(p.recipes[0].preset.as_deref(), Some("sbi-funds"));

    // The same preset for the account it already feeds would add the rows up twice.
    add(&mut p, a);
    assert_eq!(p.recipes.len(), 2);
    let before = p.clone();
    let second = p.recipes[1].name.clone();
    assert!(
        p.apply(Edit::SetRecipeAccount {
            name: second,
            account: b
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::SetRecipeAccount {
            name: name.clone(),
            account: AccountId(9999)
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::SetRecipeAccount {
            name: "missing".to_owned(),
            account: a
        })
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn a_preset_is_never_added_twice_for_one_account() {
    let (mut p, _, _, a) = sample();
    let b = account(&mut p, "Account 2");
    let add = |p: &mut Portfolio, account| {
        apply(
            p,
            Edit::AddPresetRecipe {
                preset: "sbi-funds".to_owned(),
                account,
            },
        )
    };
    add(&mut p, a);
    add(&mut p, b);
    assert_eq!(p.recipes.len(), 2);
    assert_ne!(p.recipes[0].name, p.recipes[1].name);

    // Adding it again, even after the name was changed by a rename of the account, does nothing.
    let before = p.clone();
    add(&mut p, a);
    add(&mut p, b);
    assert_eq!(p, before);

    // Once the first account's recipe is removed, the preset can be added to it again.
    let first = p.recipes[0].name.clone();
    apply(&mut p, Edit::RemoveRecipe { name: first });
    add(&mut p, a);
    assert_eq!(p.recipes.len(), 2);
}

#[test]
fn a_preset_recipe_stores_only_the_reference_and_the_account() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "daiwa-connect-stocks".to_owned(),
            account: acc,
        },
    );
    let stored = &p.recipes[0];
    assert_eq!(stored.preset.as_deref(), Some("daiwa-connect-stocks"));
    assert_eq!(stored.account, acc);
    assert!(stored.links.is_empty());
    assert!(stored.table_starts_with.is_empty() && stored.allowed_sites.is_empty());
    let json = serde_json::to_value(stored).unwrap();
    assert_eq!(json["preset"], "daiwa-connect-stocks");

    // The definition is the app's current preset.
    let effective = stored.effective().unwrap();
    assert_eq!(effective.name, stored.name);
    assert_eq!(effective.account, acc);
    assert_eq!(effective.start_site, "connect-sec.co.jp");
    assert!(effective.links[0].menu.is_some());
    assert_eq!(effective.preset.as_deref(), Some("daiwa-connect-stocks"));

    // It survives a round trip and is a valid file.
    let back: Portfolio = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);
    back.validate_integrity().unwrap();
    assert_eq!(SCHEMA_VERSION, 20);
}

#[test]
fn what_a_preset_recipe_carries_besides_its_account_is_ignored() {
    let (mut p, _, _, acc) = sample();
    // A recipe that names a preset but also writes its own definition (a site that is not vetted,
    // a link to click) gets the preset's definition, and nothing of what it wrote.
    let mut recipe = recipe_for(acc);
    recipe.preset = Some("sbi-funds".to_owned());
    recipe.allowed_sites = vec!["evil.example".to_owned()];
    recipe.start_site = "evil.example".to_owned();
    recipe.table_starts_with = "Other".to_owned();
    apply(&mut p, Edit::SetRecipe { recipe });
    let stored = &p.recipes[0];
    assert!(stored.table_starts_with.is_empty() && stored.allowed_sites.is_empty());
    assert!(stored.start_site.is_empty());
    let effective = stored.effective().unwrap();
    assert_eq!(effective.start_site, "sbisec.co.jp");
    assert_eq!(effective.table_starts_with, "投資信託（金額/");
    // Only a remembered link destination is kept (matched to the preset's link by its text).
    assert!(stored.links.iter().all(|l| l.reference.is_some()));
}

#[test]
fn an_unknown_preset_is_refused_on_save_but_kept_when_the_file_has_it() {
    let (mut p, _, _, acc) = sample();
    let mut recipe = recipe_for(acc);
    recipe.preset = Some("no-such-preset".to_owned());
    assert!(p.apply(Edit::SetRecipe { recipe }).is_err());
    assert!(
        p.apply(Edit::AddPresetRecipe {
            preset: "no-such-preset".to_owned(),
            account: acc
        })
        .is_err()
    );

    // A file that refers to a preset this version does not have is still opened, the other
    // recipes still work, and the recipe is an error only where it is used.
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-funds".to_owned(),
            account: acc,
        },
    );
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    p.recipes[0].preset = Some("removed-in-a-later-version".to_owned());
    p.validate_integrity().unwrap();
    assert!(p.recipes[0].effective().is_err());
    assert!(p.recipes[1].effective().is_ok());
    let before = p.clone();
    // Other edits go through and the recipe is kept as it was.
    apply(
        &mut p,
        Edit::AddCategory {
            name: "Another".to_owned(),
            target_weight: rust_decimal::dec!(0),
        },
    );
    assert_eq!(p.recipes, before.recipes);
    assert!(
        p.apply(Edit::DetachPresetRecipe {
            name: p.recipes[0].name.clone()
        })
        .is_err()
    );
    let name = p.recipes[0].name.clone();
    apply(&mut p, Edit::RemoveRecipe { name });
    assert_eq!(p.recipes.len(), 1);
}

#[test]
fn a_link_destination_of_a_link_the_preset_no_longer_has_does_not_make_the_file_invalid() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-funds".to_owned(),
            account: acc,
        },
    );
    // What an older version of the preset's link left behind.
    p.recipes[0].links = vec![FetchLink {
        text: "A link the preset used to follow".to_owned(),
        reference: Some("h:0123456789abcdef".to_owned()),
        menu: None,
        heading: None,
    }];
    p.validate_integrity().unwrap();
    assert!(
        p.recipes[0].effective().unwrap().links[0]
            .reference
            .is_none()
    );
    // A stored definition is still refused.
    p.recipes[0].table_starts_with = "Smuggled".to_owned();
    assert!(p.validate_integrity().is_err());
}

#[test]
fn a_custom_recipe_stays_next_to_a_preset_one_and_a_preset_can_be_detached() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-domestic-stocks".to_owned(),
            account: acc,
        },
    );
    assert_eq!(p.recipes.len(), 2);
    assert!(p.recipes[0].preset.is_none() && p.recipes[1].preset.is_some());
    // The custom recipe keeps its own definition.
    assert_eq!(p.recipes[0].effective().unwrap(), p.recipes[0]);

    let name = p.recipes[1].name.clone();
    let definition = p.recipes[1].effective().unwrap();
    apply(&mut p, Edit::DetachPresetRecipe { name: name.clone() });
    let detached = &p.recipes[1];
    assert!(detached.preset.is_none());
    assert_eq!(detached.name, name);
    assert_eq!(detached.table_starts_with, definition.table_starts_with);
    assert_eq!(detached.mapping, definition.mapping);
    assert_eq!(detached.start_site, "sbisec.co.jp");
    // It is an ordinary recipe now: it cannot be detached again, and the same preset can be
    // added again for the account (a new reference next to the copy would add the rows twice, so
    // the copy is turned back into a reference).
    assert!(p.apply(Edit::DetachPresetRecipe { name }).is_err());
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-domestic-stocks".to_owned(),
            account: acc,
        },
    );
    assert_eq!(p.recipes.len(), 2);
    assert!(p.recipes[1].preset.is_some());
}

#[test]
fn a_remembered_link_destination_of_a_preset_recipe_is_kept_and_used() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-funds".to_owned(),
            account: acc,
        },
    );
    let mut effective = p.recipes[0].effective().unwrap();
    effective.links[0].reference = Some("h:0123456789abcdef".to_owned());
    apply(&mut p, Edit::SetRecipe { recipe: effective });
    assert_eq!(p.recipes.len(), 1);
    assert_eq!(p.recipes[0].links.len(), 1);
    assert!(p.recipes[0].table_starts_with.is_empty());
    assert_eq!(
        p.recipes[0].effective().unwrap().links[0]
            .reference
            .as_deref(),
        Some("h:0123456789abcdef")
    );
}

#[test]
fn allowed_sites_of_a_recipe_must_be_vetted_sites() {
    let (mut p, _, _, acc) = sample();
    let with = |sites: &[&str]| {
        let mut recipe = recipe_for(acc);
        recipe.allowed_sites = sites.iter().map(|s| (*s).to_owned()).collect();
        recipe
    };

    // A vetted site is kept (trimmed, lower-cased, no duplicates).
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: with(&[" SBISEC.co.jp ", "sbisec.co.jp", "monex.co.jp"]),
        },
    );
    assert_eq!(
        p.recipes[0].allowed_sites,
        vec!["sbisec.co.jp", "monex.co.jp"]
    );

    // Anything else is refused, whatever it looks like: another company, a shared domain, a
    // look-alike, a parent, or a value with a scheme or a path. Nothing changes.
    let before = p.clone();
    for bad in [
        "example.com",
        "blogspot.com",
        "github.io",
        "co.jp",
        "evil-sbisec.co.jp",
        "sbisec.co.jp.evil.test",
        "www.sbisec.co.jp",
        "https://sbisec.co.jp",
        "sbisec.co.jp/path",
        "",
    ] {
        assert!(
            p.apply(Edit::SetRecipe {
                recipe: with(&[bad])
            })
            .is_err(),
            "{bad}"
        );
    }
    assert!(
        p.apply(Edit::SetRecipe {
            recipe: with(&["sbisec.co.jp", "monex.co.jp", "sbisec.co.jp", "monex.co.jp"]),
        })
        .is_err(),
        "too many entries"
    );
    assert_eq!(p, before);
}

#[test]
fn every_login_page_is_a_plain_https_address_of_its_own_site() {
    let pages = login_pages();
    assert!(!pages.is_empty());
    for (site, url) in &pages {
        let host = url
            .strip_prefix("https://")
            .unwrap()
            .split(['/', '?', '#'])
            .next()
            .unwrap();
        assert!(
            host == *site || host.ends_with(&format!(".{site}")),
            "{url}"
        );
        assert!(
            !url.contains(['@', ' ', '\\']) && !host.contains(':'),
            "{url}"
        );
        assert_eq!(login_url_for(site), Some(*url));
    }
    assert_eq!(login_url_for("unknown.example"), None);
}

#[test]
fn every_site_a_preset_reads_has_a_login_page_to_open() {
    for preset in presets() {
        let recipe_site = {
            let (mut p, _, _, acc) = sample();
            apply(
                &mut p,
                Edit::AddPresetRecipe {
                    preset: preset.id.to_owned(),
                    account: acc,
                },
            );
            p.recipes[0].effective().unwrap().start_site
        };
        assert!(!recipe_site.is_empty(), "{}", preset.id);
        assert!(login_url_for(&recipe_site).is_some(), "{}", preset.id);
    }
}

#[test]
fn a_name_becomes_a_product_with_its_alias_in_one_step_or_not_at_all() {
    let (mut p, cat, _, _) = sample();
    let revision = p.revision;

    apply(
        &mut p,
        Edit::AddProductWithAlias {
            name: " New Fund ".to_owned(),
            category: cat,
            currency: "USD".to_owned(),
            source: "SBI".to_owned(),
            alias: "7777 New Fund Co".to_owned(),
        },
    );

    let product = p.products.last().unwrap();
    assert_eq!(
        (product.name.as_str(), product.currency.as_str()),
        ("New Fund", "USD")
    );
    assert_eq!(product.aliases.len(), 1);
    assert_eq!(p.revision, revision + 1, "one accepted edit");
    p.validate_integrity().unwrap();

    // A product name that is taken, an unknown category, or an alias that another product has:
    // refused, and nothing is left behind (no half-made product).
    let before = p.clone();
    for (name, category, alias) in [
        ("New Fund", cat, "other name"),
        ("Another", CategoryId(9999), "another name"),
        ("Third", cat, "7777 New Fund Co"),
    ] {
        assert!(
            p.apply(Edit::AddProductWithAlias {
                name: name.to_owned(),
                category,
                currency: "JPY".to_owned(),
                source: "SBI".to_owned(),
                alias: alias.to_owned(),
            })
            .is_err(),
            "{name}"
        );
    }
    assert_eq!(p, before);
}

#[test]
fn every_preset_makes_a_valid_effective_recipe_that_follows_its_data() {
    let (mut p, _, _, acc) = sample();
    for info in presets() {
        apply(
            &mut p,
            Edit::AddPresetRecipe {
                preset: info.id.to_owned(),
                account: acc,
            },
        );
        let recipe = p
            .recipes
            .iter()
            .find(|r| r.preset.as_deref() == Some(info.id))
            .unwrap_or_else(|| panic!("{}", info.id));
        assert_eq!(recipe.account, acc);
        let effective = recipe.effective().unwrap();
        assert_eq!(effective.table_starts_with, info.table_starts_with);
        assert_eq!(effective.mapping.source, info.source);
        assert!(
            effective
                .links
                .iter()
                .map(|l| l.text.as_str())
                .eq(info.links.iter().copied())
        );
    }
    p.validate_integrity().unwrap();
}

#[test]
fn an_account_moves_up_and_down_and_stops_at_the_ends() {
    let mut p = Portfolio::default();
    let a = account(&mut p, "A");
    let b = account(&mut p, "B");
    let c = account(&mut p, "C");
    let order = |p: &Portfolio| p.accounts.iter().map(|x| x.id).collect::<Vec<_>>();
    assert_eq!(order(&p), vec![a, b, c]);

    apply(&mut p, Edit::MoveAccount { id: c, up: true });
    assert_eq!(order(&p), vec![a, c, b]);
    apply(&mut p, Edit::MoveAccount { id: c, up: true });
    assert_eq!(order(&p), vec![c, a, b]);
    // At the top (and the bottom) nothing changes, and it is not an error.
    apply(&mut p, Edit::MoveAccount { id: c, up: true });
    assert_eq!(order(&p), vec![c, a, b]);
    apply(&mut p, Edit::MoveAccount { id: b, up: false });
    assert_eq!(order(&p), vec![c, a, b]);
    apply(&mut p, Edit::MoveAccount { id: c, up: false });
    assert_eq!(order(&p), vec![a, c, b]);

    // An unknown account is refused and nothing changes.
    let before = p.clone();
    assert!(
        p.apply(Edit::MoveAccount {
            id: AccountId(9999),
            up: true
        })
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn groups_and_categories_move_up_and_down_and_stop_at_the_ends() {
    let mut p = Portfolio::default();
    let a = add_group(&mut p, "A");
    let b = add_group(&mut p, "B");
    let c = add_group(&mut p, "C");
    let order = |p: &Portfolio| p.groups.iter().map(|x| x.id).collect::<Vec<_>>();
    apply(&mut p, Edit::MoveGroup { id: c, up: true });
    assert_eq!(order(&p), vec![a, c, b]);
    apply(&mut p, Edit::MoveGroup { id: c, up: true });
    apply(&mut p, Edit::MoveGroup { id: c, up: true });
    assert_eq!(order(&p), vec![c, a, b]);
    apply(&mut p, Edit::MoveGroup { id: b, up: false });
    assert_eq!(order(&p), vec![c, a, b]);
    apply(&mut p, Edit::MoveGroup { id: c, up: false });
    assert_eq!(order(&p), vec![a, c, b]);
    let before = p.clone();
    assert!(
        p.apply(Edit::MoveGroup {
            id: GroupId(9999),
            up: true
        })
        .is_err()
    );
    assert_eq!(p, before);

    let x = category(&mut p, "X", rust_decimal::Decimal::ZERO);
    let y = category(&mut p, "Y", rust_decimal::Decimal::ZERO);
    let z = category(&mut p, "Z", rust_decimal::Decimal::ZERO);
    let corder = |p: &Portfolio| p.categories.iter().map(|x| x.id).collect::<Vec<_>>();
    apply(&mut p, Edit::MoveCategory { id: z, up: true });
    assert_eq!(corder(&p), vec![x, z, y]);
    apply(&mut p, Edit::MoveCategory { id: x, up: true });
    assert_eq!(corder(&p), vec![x, z, y]);
    apply(&mut p, Edit::MoveCategory { id: y, up: false });
    assert_eq!(corder(&p), vec![x, z, y]);
    // The opposite move undoes a move.
    apply(&mut p, Edit::MoveCategory { id: x, up: false });
    apply(&mut p, Edit::MoveCategory { id: x, up: true });
    assert_eq!(corder(&p), vec![x, z, y]);
    let before = p.clone();
    assert!(
        p.apply(Edit::MoveCategory {
            id: CategoryId(9999),
            up: true
        })
        .is_err()
    );
    assert_eq!(p, before);
    p.validate_integrity().unwrap();
}

#[test]
fn rows_move_to_a_given_position() {
    let mut p = Portfolio::default();
    let a = account(&mut p, "A");
    let b = account(&mut p, "B");
    let c = account(&mut p, "C");
    let order = |p: &Portfolio| p.accounts.iter().map(|x| x.id).collect::<Vec<_>>();
    apply(&mut p, Edit::MoveAccountTo { id: c, index: 0 });
    assert_eq!(order(&p), vec![c, a, b]);
    apply(&mut p, Edit::MoveAccountTo { id: c, index: 2 });
    assert_eq!(order(&p), vec![a, b, c]);
    // The same place changes nothing.
    apply(&mut p, Edit::MoveAccountTo { id: b, index: 1 });
    assert_eq!(order(&p), vec![a, b, c]);
    let before = p.clone();
    assert!(p.apply(Edit::MoveAccountTo { id: a, index: 3 }).is_err());
    assert!(
        p.apply(Edit::MoveAccountTo {
            id: AccountId(9999),
            index: 0
        })
        .is_err()
    );
    assert_eq!(p, before);

    let g1 = add_group(&mut p, "G1");
    let g2 = add_group(&mut p, "G2");
    let g3 = add_group(&mut p, "G3");
    apply(&mut p, Edit::MoveGroupTo { id: g1, index: 2 });
    let gorder = |p: &Portfolio| p.groups.iter().map(|x| x.id).collect::<Vec<_>>();
    assert_eq!(gorder(&p), vec![g2, g3, g1]);
    assert!(p.apply(Edit::MoveGroupTo { id: g1, index: 3 }).is_err());
    assert!(
        p.apply(Edit::MoveGroupTo {
            id: GroupId(9999),
            index: 0
        })
        .is_err()
    );

    let x = category(&mut p, "X", rust_decimal::Decimal::ZERO);
    let y = category(&mut p, "Y", rust_decimal::Decimal::ZERO);
    let z = category(&mut p, "Z", rust_decimal::Decimal::ZERO);
    apply(&mut p, Edit::MoveCategoryTo { id: x, index: 1 });
    let corder = |p: &Portfolio| p.categories.iter().map(|x| x.id).collect::<Vec<_>>();
    assert_eq!(corder(&p), vec![y, x, z]);
    assert!(p.apply(Edit::MoveCategoryTo { id: x, index: 3 }).is_err());
    p.validate_integrity().unwrap();
}

fn share_edit(mode: SuggestionMode, product: ProductId, share: ProductShare) -> Edit {
    Edit::SetProductShare {
        mode,
        product,
        share,
    }
}

#[test]
fn a_product_has_its_own_setting_for_each_mode() {
    let (mut p, _, prod, _) = sample();
    let weighted = ProductShare::Weighted { weight: dec!(3) };

    // Nothing set: an equal share in every mode.
    for mode in SuggestionMode::ALL {
        assert_eq!(p.product_share(mode, prod), ProductShare::Equal);
    }
    apply(&mut p, share_edit(SuggestionMode::FixedAdd, prod, weighted));
    apply(
        &mut p,
        share_edit(SuggestionMode::FixedSell, prod, ProductShare::KeepCurrent),
    );

    assert_eq!(p.product_share(SuggestionMode::FixedAdd, prod), weighted);
    assert_eq!(
        p.product_share(SuggestionMode::FixedSell, prod),
        ProductShare::KeepCurrent
    );
    assert_eq!(
        p.product_share(SuggestionMode::FullRebalance, prod),
        ProductShare::Equal
    );
    p.validate_integrity().unwrap();

    // Setting a mode again replaces it; an equal share is not stored.
    apply(
        &mut p,
        share_edit(SuggestionMode::FixedAdd, prod, ProductShare::Equal),
    );
    assert_eq!(p.product_shares.len(), 1);
    p.validate_integrity().unwrap();
}

#[test]
fn a_product_weight_must_be_positive_and_not_huge() {
    let (mut p, _, prod, _) = sample();
    let before = p.clone();
    for weight in [dec!(0), dec!(-1), MAX_PRODUCT_WEIGHT + dec!(1)] {
        assert!(
            p.apply(share_edit(
                SuggestionMode::FixedAdd,
                prod,
                ProductShare::Weighted { weight }
            ))
            .is_err()
        );
    }
    assert!(
        p.apply(share_edit(
            SuggestionMode::FixedAdd,
            ProductId(9999),
            ProductShare::KeepCurrent
        ))
        .is_err()
    );
    assert_eq!(p, before);
    apply(
        &mut p,
        share_edit(
            SuggestionMode::FixedAdd,
            prod,
            ProductShare::Weighted {
                weight: MAX_PRODUCT_WEIGHT,
            },
        ),
    );
}

#[test]
fn removing_or_merging_a_product_drops_its_settings() {
    let (mut p, c, prod, _) = sample();
    let other = product(&mut p, "Fund Y", c);
    for id in [prod, other] {
        apply(
            &mut p,
            share_edit(SuggestionMode::FullRebalance, id, ProductShare::KeepCurrent),
        );
    }
    apply(
        &mut p,
        Edit::MergeProducts {
            keep: prod,
            remove: other,
            on_conflict: MergeConflict::Keep,
        },
    );
    assert_eq!(p.product_shares.len(), 1);
    assert_eq!(p.product_shares[0].product, prod);

    apply(&mut p, Edit::RemoveProduct { id: prod });
    assert!(p.product_shares.is_empty());
    p.validate_integrity().unwrap();
}

#[test]
fn a_file_with_a_bad_product_setting_is_refused() {
    let (mut p, _, prod, _) = sample();
    let entry = |share| ProductShareEntry {
        mode: SuggestionMode::FixedAdd,
        product: prod,
        share,
    };
    for entries in [
        vec![entry(ProductShare::Equal)],
        vec![entry(ProductShare::Weighted { weight: dec!(0) })],
        vec![
            entry(ProductShare::KeepCurrent),
            entry(ProductShare::KeepCurrent),
        ],
        vec![ProductShareEntry {
            product: ProductId(9999),
            ..entry(ProductShare::KeepCurrent)
        }],
    ] {
        p.product_shares = entries;
        assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));
    }
}

#[test]
fn a_malformed_legacy_weight_is_refused_by_the_migration() {
    for weight in [r#""0""#, r#""-1""#, r#""x""#, "3"] {
        let old = format!(
            r#"{{"schema_version": 19, "next_id": 20,
              "categories": [{{"id": 5, "name": "W", "target_weight": "1", "targeting": "weighted"}}],
              "products": [{{"id": 1, "name": "A", "category": 5, "currency": "JPY"}}],
              "accounts": [{{"id": 10, "name": "M", "sellable": true, "rounding_unit": "1"}}],
              "holdings": [{{"account": 10, "product": 1, "value": "100", "weight": {weight}}}]}}"#
        );
        assert!(parse(old.as_bytes()).is_err(), "{weight}");
    }
}

#[test]
fn the_targeting_of_an_older_file_becomes_a_setting_per_mode() {
    let old = r#"{"schema_version": 19, "next_id": 40,
      "categories": [
        {"id": 5, "name": "W", "target_weight": "0.5", "targeting": "weighted"},
        {"id": 6, "name": "K", "target_weight": "0.3", "targeting": "keep_current"},
        {"id": 7, "name": "E", "target_weight": "0.1", "targeting": "equal"},
        {"id": 8, "name": "F", "target_weight": "0.1"}],
      "products": [
        {"id": 1, "name": "A", "category": 5, "currency": "JPY"},
        {"id": 2, "name": "B", "category": 5, "currency": "JPY"},
        {"id": 3, "name": "C", "category": 6, "currency": "JPY"},
        {"id": 4, "name": "D", "category": 7, "currency": "JPY"},
        {"id": 9, "name": "E", "category": 8, "currency": "JPY"}],
      "accounts": [
        {"id": 10, "name": "M", "sellable": true, "rounding_unit": "1"},
        {"id": 11, "name": "N", "sellable": true, "rounding_unit": "1"}],
      "holdings": [
        {"account": 10, "product": 1, "value": "100", "weight": "2"},
        {"account": 11, "product": 1, "value": "0", "weight": "1"},
        {"account": 10, "product": 3, "value": "50", "weight": "7"}]}"#;

    let mut p = parse(old.as_bytes()).unwrap();
    p.schema_version = SCHEMA_VERSION;

    p.validate_integrity().unwrap();
    for mode in SuggestionMode::ALL {
        // The weights of the records add up; a weighted product without one is an equal share.
        assert_eq!(
            p.product_share(mode, ProductId(1)),
            ProductShare::Weighted { weight: dec!(3) }
        );
        assert_eq!(p.product_share(mode, ProductId(2)), ProductShare::Equal);
        assert_eq!(
            p.product_share(mode, ProductId(3)),
            ProductShare::KeepCurrent
        );
        assert_eq!(p.product_share(mode, ProductId(4)), ProductShare::Equal);
        assert_eq!(p.product_share(mode, ProductId(9)), ProductShare::Equal);
    }
    assert_eq!(p.product_shares.len(), 8);
    // Both old fields are gone, and the result round-trips as a current file.
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["categories"][0].get("targeting").is_none());
    assert!(json["holdings"][0].get("weight").is_none());
    let back = parse(serde_json::to_string(&p).unwrap().as_bytes()).unwrap();
    assert_eq!(back, p);
}

#[test]
fn only_the_supported_schema_versions_are_opened() {
    let mut p = Portfolio::default();
    p.validate_integrity().unwrap();
    for version in [1, MIN_SCHEMA_VERSION - 1, SCHEMA_VERSION + 1] {
        p.schema_version = version;
        assert_eq!(
            p.validate_integrity(),
            Err(Error::UnsupportedVersion { found: version })
        );
    }
    p.schema_version = 0;
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));
}

#[test]
fn a_menu_link_needs_its_heading_a_vetted_site_and_no_mixing() {
    let (mut p, _, _, acc) = sample();
    let menu = |text: &str| FetchLink {
        text: text.to_owned(),
        reference: None,
        menu: Some("Group".to_owned()),
        heading: Some("Heading".to_owned()),
    };
    let plain = |text: &str| FetchLink {
        text: text.to_owned(),
        reference: None,
        menu: None,
        heading: None,
    };
    let before = p.clone();
    let mut with = |change: &dyn Fn(&mut Recipe)| {
        let mut r = recipe_for(acc);
        r.allowed_sites = vec!["connect-sec.co.jp".to_owned()];
        r.links = vec![menu("Entry")];
        change(&mut r);
        p.apply(Edit::SetRecipe { recipe: r }).is_err()
    };
    // The good one is accepted (and a menu link with no extra change is a plain recipe edit).
    assert!(!with(&|_| {}));
    // A group without a heading, or a heading without a group.
    assert!(with(&|r| r.links[0].heading = None));
    assert!(with(&|r| r.links[0].menu = None));
    assert!(with(&|r| r.links[0].heading = Some("  ".to_owned())));
    // A control character, or too long.
    assert!(with(&|r| r.links[0].menu = Some("a\nb".to_owned())));
    assert!(with(&|r| r.links[0].heading = Some("x".repeat(61))));
    // A script link has no saved destination.
    assert!(with(&|r| r.links[0].reference = Some("abc".to_owned())));
    // Clicks only on a vetted site, and never mixed with plain links.
    assert!(with(&|r| r.allowed_sites = vec![]));
    assert!(with(&|r| r.allowed_sites = vec!["example.com".to_owned()]));
    assert!(with(&|r| r.links = vec![plain("A"), menu("B")]));
    assert!(with(&|r| r.links = vec![menu("B"), plain("A")]));
    assert_ne!(p, before, "the good recipe was saved");
}

#[test]
fn the_daiwa_presets_click_their_menu_entries_on_the_vetted_site() {
    let (mut p, _, _, acc) = sample();
    for id in ["daiwa-connect-stocks", "daiwa-connect-cash"] {
        apply(
            &mut p,
            Edit::AddPresetRecipe {
                preset: id.to_owned(),
                account: acc,
            },
        );
    }
    let effective: Vec<Recipe> = p.recipes.iter().map(|r| r.effective().unwrap()).collect();
    let daiwa: Vec<_> = effective
        .iter()
        .filter(|r| r.start_site == "connect-sec.co.jp")
        .collect();
    assert_eq!(daiwa.len(), 2);
    for r in daiwa {
        assert_eq!(r.links.len(), 1);
        assert!(r.links[0].menu.is_some() && r.links[0].heading.is_some());
        assert_eq!(r.allowed_sites, vec!["connect-sec.co.jp".to_owned()]);
        assert!(crate::edit::VETTED_SITES.contains(&"connect-sec.co.jp"));
    }
    // The other presets still follow plain links (or none), never clicks.
    for r in effective
        .iter()
        .filter(|r| r.start_site != "connect-sec.co.jp")
    {
        assert!(r.links.iter().all(|l| l.menu.is_none()));
    }
}

// ---- groups ---------------------------------------------------------------

fn add_group(p: &mut Portfolio, name: &str) -> GroupId {
    apply(
        p,
        Edit::AddGroup {
            name: name.to_owned(),
        },
    );
    p.groups.last().unwrap().id
}

#[test]
fn a_group_is_added_renamed_and_removed_and_its_categories_become_ungrouped() {
    let (mut p, cat, ..) = sample();
    let g = add_group(&mut p, "  Risk  ");
    assert_eq!(p.groups[0].name, "Risk");
    apply(
        &mut p,
        Edit::SetCategoryGroup {
            category: cat,
            group: Some(g),
        },
    );
    assert_eq!(p.categories[0].group, Some(g));

    apply(
        &mut p,
        Edit::RenameGroup {
            id: g,
            name: "Growth".to_owned(),
        },
    );
    assert_eq!(p.groups[0].name, "Growth");

    apply(&mut p, Edit::RemoveGroup { id: g });
    assert!(p.groups.is_empty());
    assert_eq!(p.categories[0].group, None);
    p.validate_integrity().unwrap();
}

#[test]
fn group_edits_are_validated_and_a_rejected_one_changes_nothing() {
    let (mut p, cat, ..) = sample();
    let g = add_group(&mut p, "Risk");
    let before = p.clone();

    let bad = [
        Edit::AddGroup {
            name: "Risk".to_owned(),
        },
        Edit::AddGroup {
            name: " ".to_owned(),
        },
        Edit::RenameGroup {
            id: GroupId(999),
            name: "X".to_owned(),
        },
        Edit::RemoveGroup { id: GroupId(999) },
        Edit::SetCategoryGroup {
            category: cat,
            group: Some(GroupId(999)),
        },
        Edit::SetCategoryGroup {
            category: CategoryId(999),
            group: Some(g),
        },
    ];
    for edit in bad {
        assert!(p.clone().apply(edit.clone()).is_err(), "{edit:?}");
        let mut copy = p.clone();
        let _ = copy.apply(edit);
        assert_eq!(copy, before);
    }
    // Renaming a group to its own name is fine.
    apply(
        &mut p,
        Edit::RenameGroup {
            id: g,
            name: "Risk".to_owned(),
        },
    );
}

#[test]
fn group_ids_come_from_the_shared_counter_and_are_never_reused() {
    let (mut p, ..) = sample();
    let a = add_group(&mut p, "A");
    apply(&mut p, Edit::RemoveGroup { id: a });
    let b = add_group(&mut p, "B");

    assert_ne!(a, b);
    p.validate_integrity().unwrap();
}

#[test]
fn integrity_check_rejects_bad_groups() {
    let (mut good, cat, ..) = sample();
    let g = add_group(&mut good, "Risk");
    apply(
        &mut good,
        Edit::SetCategoryGroup {
            category: cat,
            group: Some(g),
        },
    );
    good.validate_integrity().unwrap();

    let mut p = good.clone();
    p.categories[0].group = Some(GroupId(999));
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.groups.push(Group {
        id: GroupId(p.next_id),
        name: "Other".to_owned(),
    });
    p.next_id += 1;
    p.validate_integrity().unwrap();
    p.groups[1].name = "Risk".to_owned();
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good.clone();
    p.groups[0].id = GroupId(cat.0);
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));

    let mut p = good;
    p.groups[0].name = "\u{7}".to_owned();
    assert!(matches!(p.validate_integrity(), Err(Error::Corrupt(_))));
}

#[test]
fn a_file_without_groups_loads_and_groups_round_trip_through_json() {
    let (mut p, cat, ..) = sample();
    let json = serde_json::to_string(&p).unwrap();
    assert!(!json.contains("\"group\""));
    let loaded: Portfolio = serde_json::from_str(&json).unwrap();
    assert!(loaded.groups.is_empty());

    let g = add_group(&mut p, "Risk");
    apply(
        &mut p,
        Edit::SetCategoryGroup {
            category: cat,
            group: Some(g),
        },
    );
    let back: Portfolio = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);
}

#[test]
fn every_preset_text_has_a_japanese_translation() {
    // The screen shows the label and description of a preset through the dictionary; a text the
    // dictionary lacks used to leave the preset list empty in Japanese.
    let dictionary = include_str!("../../../src/i18n.ts");
    for preset in presets() {
        for text in [preset.label, preset.description] {
            let key = format!("\"{}\"", text.replace('"', "\\\""));
            assert!(dictionary.contains(&key), "no translation for: {text}");
        }
    }
}

fn cards_recipe_for(account: AccountId) -> Recipe {
    Recipe {
        table_starts_with: String::new(),
        all_tables: false,
        following_tables: false,
        cards: Some(Box::new(CardSource {
            heading: " Holdings ".to_owned(),
            item_depth: 2,
        })),
        mapping: ImportMapping {
            header_rows: 0,
            rows_per_record: 1,
            name_row: 0,
            value_row: 0,
            name_column: 0,
            value_column: 1,
            price_column: None,
            ..recipe_for(account).mapping
        },
        ..recipe_for(account)
    }
}

#[test]
fn a_cards_recipe_is_saved_trimmed_and_survives_a_round_trip() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: cards_recipe_for(acc),
        },
    );
    let cards = p.recipes[0].cards.clone().unwrap();
    assert_eq!((cards.heading.as_str(), cards.item_depth), ("Holdings", 2));
    let back: Portfolio = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);
    back.validate_integrity().unwrap();
}

#[test]
fn a_cards_recipe_is_exclusive_with_a_table_title_and_needs_a_valid_source() {
    let (mut p, _, _, acc) = sample();
    let bad = |change: &dyn Fn(&mut Recipe)| {
        let mut recipe = cards_recipe_for(acc);
        change(&mut recipe);
        recipe
    };
    for recipe in [
        bad(&|r| r.table_starts_with = "Stocks".to_owned()),
        bad(&|r| r.all_tables = true),
        bad(&|r| r.following_tables = true),
        bad(&|r| r.cards.as_mut().unwrap().heading = "  ".to_owned()),
        bad(&|r| r.cards.as_mut().unwrap().heading = "a\nb".to_owned()),
        bad(&|r| r.cards.as_mut().unwrap().heading = "x".repeat(61)),
        bad(&|r| r.cards.as_mut().unwrap().item_depth = 0),
        bad(&|r| r.cards.as_mut().unwrap().item_depth = 4),
        bad(&|r| r.mapping.header_rows = 1),
        bad(&|r| r.mapping.rows_per_record = 2),
    ] {
        assert!(p.apply(Edit::SetRecipe { recipe }).is_err());
    }
    assert!(p.recipes.is_empty());
}

#[test]
fn a_file_of_the_previous_schema_without_cards_still_loads() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe_for(acc),
        },
    );
    let mut value = serde_json::to_value(&p).unwrap();
    value["schema_version"] = serde_json::json!(SCHEMA_VERSION - 1);
    assert!(value["recipes"][0].get("cards").is_none());
    let old: Portfolio = serde_json::from_value(value).unwrap();
    old.validate_integrity().unwrap();
    assert_eq!(old.recipes[0].cards, None);
}

fn kind_mapping() -> ImportMapping {
    ImportMapping {
        price_column: Some(3),
        kind_column: Some(0),
        kind_rules: vec![
            KindRule {
                kind: "Fund".to_owned(),
                read: KindRead::QuantityTimesPrice { price_unit: 10_000 },
            },
            KindRule {
                kind: "Stock".to_owned(),
                read: KindRead::Evaluation {
                    column: 4,
                    part: Some(0),
                },
            },
        ],
        ..mapping_for("Mixed")
    }
}

#[test]
fn kind_rules_are_validated_like_the_rest_of_a_mapping() {
    let mut p = Portfolio::default();
    assert!(p.apply(set_mapping(kind_mapping())).is_ok());
    assert_eq!(p.import_mappings[0].kind_rules.len(), 2);

    let refused = |change: &dyn Fn(&mut ImportMapping)| {
        let mut m = kind_mapping();
        change(&mut m);
        Portfolio::default().apply(set_mapping(m)).is_err()
    };
    assert!(
        refused(&|m| m.kind_column = None),
        "rules without a kind column"
    );
    assert!(
        refused(&|m| m.kind_rules.clear()),
        "a kind column without rules"
    );
    assert!(
        refused(&|m| m.kind_rules[1].kind = " Fund ".to_owned()),
        "a kind twice (after the spaces are tidied)"
    );
    assert!(
        refused(&|m| m.kind_rules[0].kind = "  ".to_owned()),
        "an empty kind"
    );
    assert!(
        refused(&|m| m.kind_column = Some(5000)),
        "the kind column out of range"
    );
    assert!(
        refused(&|m| m.kind_rules[1].read = KindRead::Evaluation {
            column: 5000,
            part: None
        }),
        "the evaluation column out of range"
    );
    assert!(
        refused(&|m| m.kind_rules[1].read = KindRead::Evaluation {
            column: 4,
            part: Some(500)
        }),
        "the part out of range"
    );
    assert!(
        refused(&|m| m.kind_rules[0].read = KindRead::QuantityTimesPrice { price_unit: 0 }),
        "a price unit of 0"
    );
    assert!(
        refused(&|m| m.price_column = None),
        "a rule that multiplies by the price without a price column"
    );
}

#[test]
fn kind_rules_survive_the_file_and_older_files_have_none() {
    let mut p = Portfolio::default();
    p.apply(set_mapping(kind_mapping())).unwrap();
    let back: Portfolio = serde_json::from_str(&serde_json::to_string(&p).unwrap()).unwrap();
    assert_eq!(back, p);

    let old: ImportMapping = serde_json::from_str(
        r#"{"source":"SBI","name_column":0,"value_column":1,"header_rows":1}"#,
    )
    .unwrap();
    assert_eq!(old.kind_column, None);
    assert!(old.kind_rules.is_empty());
    // A mapping without kinds is written as before (the older file format is unchanged).
    assert!(!serde_json::to_string(&old).unwrap().contains("kind"));
}

#[test]
fn the_rakuten_preset_reads_trusts_and_stocks_by_kind_and_a_reference_keeps_none() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "rakuten-all".to_owned(),
            account: acc,
        },
    );
    let stored = &p.recipes[0];
    assert_eq!(stored.mapping.kind_column, None);
    assert!(stored.mapping.kind_rules.is_empty());

    let effective = stored.effective().unwrap();
    assert_eq!(effective.mapping.kind_column, Some(0));
    let reads: Vec<KindRead> = effective
        .mapping
        .kind_rules
        .iter()
        .map(|r| r.read)
        .collect();
    assert_eq!(
        reads,
        vec![
            KindRead::QuantityTimesPrice { price_unit: 10_000 },
            KindRead::Evaluation {
                column: 7,
                part: Some(0)
            }
        ]
    );
    // The effective recipe passes the same validation as a recipe the user writes.
    effective.mapping.clone().validated().unwrap();
}

fn hold(p: &mut Portfolio, account: AccountId, product: ProductId, value: rust_decimal::Decimal) {
    apply(
        p,
        Edit::SetHolding {
            account_type: None,
            account,
            product,
            value,
            quantity: None,
        },
    );
}

fn alias(p: &mut Portfolio, product: ProductId, name: &str) {
    apply(
        p,
        Edit::AddAlias {
            product,
            source: "Source".to_owned(),
            name: name.to_owned(),
        },
    );
}

#[test]
fn merging_moves_aliases_and_records_and_removes_the_product() {
    let (mut p, c, a, acc1) = sample();
    let b = product(&mut p, "Fund X dup", c);
    let acc2 = account(&mut p, "Account 2");
    alias(&mut p, b, "FUND X");
    hold(&mut p, acc2, b, dec!(300));
    apply(
        &mut p,
        Edit::SetExcluded {
            id: acc2,
            products: vec![b],
        },
    );

    apply(
        &mut p,
        Edit::MergeProducts {
            keep: a,
            remove: b,
            on_conflict: MergeConflict::Keep,
        },
    );

    assert!(p.products.iter().all(|x| x.id != b));
    assert_eq!(
        p.products.iter().find(|x| x.id == a).unwrap().aliases.len(),
        1
    );
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account == acc2 && h.product == a)
    );
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account == acc1 && h.product == a)
    );
    assert_eq!(p.accounts[1].excluded, vec![a]);
    p.validate_integrity().unwrap();
}

#[test]
fn merging_resolves_a_record_in_the_same_account_as_asked() {
    for (policy, expected) in [
        (MergeConflict::Keep, dec!(1000)),
        (MergeConflict::Take, dec!(300)),
    ] {
        let (mut p, c, a, acc) = sample();
        let b = product(&mut p, "Fund X dup", c);
        hold(&mut p, acc, b, dec!(300));

        apply(
            &mut p,
            Edit::MergeProducts {
                keep: a,
                remove: b,
                on_conflict: policy,
            },
        );

        let own: Vec<_> = p.holdings.iter().filter(|h| h.account == acc).collect();
        assert_eq!(own.len(), 1);
        assert_eq!((own[0].product, own[0].value), (a, expected));
        p.validate_integrity().unwrap();
    }
}

#[test]
fn merging_refuses_different_currencies_and_the_same_product() {
    let (mut p, c, a, _) = sample();
    let b = product(&mut p, "Fund Y", c);
    apply(
        &mut p,
        Edit::UpdateProduct {
            id: b,
            name: None,
            category: None,
            currency: Some("USD".to_owned()),
        },
    );
    let before = p.clone();

    assert!(
        p.apply(Edit::MergeProducts {
            keep: a,
            remove: b,
            on_conflict: MergeConflict::Keep
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::MergeProducts {
            keep: a,
            remove: a,
            on_conflict: MergeConflict::Keep
        })
        .is_err()
    );
    assert_eq!(p, before);
}

#[test]
fn splitting_cuts_out_an_alias_with_the_picked_records() {
    let (mut p, _, a, acc1) = sample();
    let acc2 = account(&mut p, "Account 2");
    hold(&mut p, acc2, a, dec!(200));
    alias(&mut p, a, "Name 1");
    alias(&mut p, a, "Name 2");
    apply(
        &mut p,
        Edit::SetExcluded {
            id: acc1,
            products: vec![a],
        },
    );

    apply(
        &mut p,
        Edit::SplitProduct {
            product: a,
            source: "Source".to_owned(),
            alias: "Name 2".to_owned(),
            name: "Fund Z".to_owned(),
            accounts: vec![acc2],
        },
    );

    let z = p.products.iter().find(|x| x.name == "Fund Z").unwrap();
    assert_eq!(z.aliases.len(), 1);
    assert_eq!(z.aliases[0].name, "Name 2");
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account == acc2 && h.product == z.id)
    );
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account == acc1 && h.product == a)
    );
    assert_eq!(
        p.products.iter().find(|x| x.id == a).unwrap().aliases.len(),
        1
    );
    assert_eq!(p.accounts[0].excluded, vec![a, z.id]);
    p.validate_integrity().unwrap();
}

#[test]
fn a_split_is_undone_by_merging_the_new_product_back() {
    let (mut p, _, a, acc) = sample();
    alias(&mut p, a, "Name 1");
    let before = p.clone();

    apply(
        &mut p,
        Edit::SplitProduct {
            product: a,
            source: "Source".to_owned(),
            alias: "Name 1".to_owned(),
            name: "Fund Z".to_owned(),
            accounts: vec![acc],
        },
    );
    let z = p.products.iter().find(|x| x.name == "Fund Z").unwrap().id;
    apply(
        &mut p,
        Edit::MergeProducts {
            keep: a,
            remove: z,
            on_conflict: MergeConflict::Keep,
        },
    );

    assert_eq!(p.products, before.products);
    assert_eq!(p.holdings, before.holdings);
}

#[test]
fn a_split_with_an_unknown_alias_taken_name_or_account_is_refused() {
    let (mut p, _, a, acc) = sample();
    alias(&mut p, a, "Name 1");
    let before = p.clone();
    let split = |alias: &str, name: &str, accounts: Vec<AccountId>| Edit::SplitProduct {
        product: a,
        source: "Source".to_owned(),
        alias: alias.to_owned(),
        name: name.to_owned(),
        accounts,
    };

    assert!(p.apply(split("Nope", "Fund Z", vec![acc])).is_err());
    assert!(p.apply(split("Name 1", "Fund X", vec![acc])).is_err());
    assert!(
        p.apply(split("Name 1", "Fund Z", vec![AccountId(9999)]))
            .is_err()
    );
    assert_eq!(p, before);
}

fn typed(
    account: AccountId,
    product: ProductId,
    t: Option<&str>,
    value: rust_decimal::Decimal,
) -> Edit {
    Edit::SetHolding {
        account,
        product,
        account_type: t.map(str::to_owned),
        value,
        quantity: None,
    }
}

#[test]
fn one_account_holds_the_same_product_under_several_types() {
    let (mut p, _, prod, acc) = sample();
    apply(&mut p, typed(acc, prod, Some("specific"), dec!(100)));
    apply(&mut p, typed(acc, prod, Some("nisa"), dec!(50)));
    apply(&mut p, typed(acc, prod, Some("nisa"), dec!(70)));
    // `sample` already holds an untyped record of the product.
    assert_eq!(p.holdings.len(), 3);
    assert_eq!(
        p.holdings
            .iter()
            .find(|h| h.account_type.as_deref() == Some("nisa"))
            .unwrap()
            .value,
        dec!(70)
    );
    p.validate_integrity().unwrap();

    apply(
        &mut p,
        Edit::RemoveHolding {
            account: acc,
            product: prod,
            account_type: Some("nisa".to_owned()),
        },
    );
    assert_eq!(p.holdings.len(), 2);
    assert!(
        p.apply(typed(acc, prod, Some("no-such-type"), dec!(1)))
            .is_err()
    );
}

#[test]
fn a_record_type_can_be_changed_unless_the_target_exists() {
    let (mut p, _, prod, acc) = sample();
    apply(&mut p, typed(acc, prod, Some("nisa"), dec!(50)));
    let change = |from: Option<&str>, to: Option<&str>| Edit::SetRecordType {
        account: acc,
        product: prod,
        from: from.map(str::to_owned),
        to: to.map(str::to_owned),
    };
    // The untyped record of `sample` is there: moving the NISA record onto it is refused.
    assert!(matches!(
        p.apply(change(Some("nisa"), None)),
        Err(Error::Conflict(_))
    ));
    apply(&mut p, change(Some("nisa"), Some("old_nisa")));
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account_type.as_deref() == Some("old_nisa"))
    );
    assert!(p.apply(change(Some("nisa"), Some("general"))).is_err());
    assert!(p.apply(change(None, Some("custom-99"))).is_err());
}

#[test]
fn custom_types_are_added_renamed_and_removed() {
    let (mut p, _, prod, acc) = sample();
    apply(
        &mut p,
        Edit::AddAccountType {
            name: "iDeCo".to_owned(),
            sellable: false,
        },
    );
    let id = p.custom_types[0].id.clone();
    assert!(id.starts_with("custom-"));
    assert!(!p.type_sellable(Some(&id)));
    assert!(p.type_sellable(Some("general")) && !p.type_sellable(Some("old_nisa")));
    for nisa in ["nisa", "nisa_growth", "nisa_tsumitate", "old_nisa"] {
        assert!(
            p.has_account_type(nisa) && !p.type_sellable(Some(nisa)),
            "{nisa}"
        );
    }
    assert!(p.type_sellable(Some("deposit")) && p.type_sellable(Some("crypto")));
    assert!(p.type_sellable(None));
    assert!(
        p.apply(Edit::AddAccountType {
            name: "iDeCo".to_owned(),
            sellable: true,
        })
        .is_err()
    );
    apply(
        &mut p,
        Edit::UpdateAccountType {
            id: id.clone(),
            name: Some("Pension".to_owned()),
            sellable: Some(true),
        },
    );
    assert!(p.type_sellable(Some(&id)));

    apply(&mut p, typed(acc, prod, Some(&id), dec!(10)));
    p.validate_integrity().unwrap();
    apply(&mut p, Edit::RemoveAccountType { id: id.clone() });
    // The typed record is folded into the untyped one of the same product.
    assert!(p.custom_types.is_empty());
    assert_eq!(p.holdings.len(), 1);
    assert!(p.holdings[0].account_type.is_none());
    assert!(p.apply(Edit::RemoveAccountType { id }).is_err());
}

#[test]
fn an_import_keeps_the_type_of_the_only_existing_record() {
    let (mut p, _, prod, acc) = sample();
    apply(
        &mut p,
        Edit::RemoveHolding {
            account: acc,
            product: prod,
            account_type: None,
        },
    );
    apply(&mut p, typed(acc, prod, Some("nisa"), dec!(50)));
    let input = |t: Option<&str>| HoldingInput {
        product: prod,
        account_type: t.map(str::to_owned),
        value: dec!(80),
        quantity: None,
        price: None,
        price_unit: 1,
    };
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![input(None)],
        },
    );
    assert_eq!(p.holdings[0].account_type.as_deref(), Some("nisa"));
    assert_eq!(p.holdings[0].value, dec!(80));

    // A read type wins, and the same product under two types is two records.
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![input(Some("specific")), input(Some("old_nisa"))],
        },
    );
    assert_eq!(p.holdings.len(), 2);
    assert!(
        p.apply(Edit::ReplaceHoldings {
            types: None,
            account: acc,
            holdings: vec![input(Some("nisa")), input(Some("nisa"))],
        })
        .is_err()
    );
}

#[test]
fn the_integrity_check_rejects_unknown_types_and_duplicates() {
    let (mut p, _, prod, acc) = sample();
    apply(&mut p, typed(acc, prod, Some("nisa"), dec!(5)));
    p.validate_integrity().unwrap();

    let mut bad = p.clone();
    bad.holdings[0].account_type = Some("custom-9".to_owned());
    assert!(bad.validate_integrity().is_err());

    let mut twice = p.clone();
    twice.holdings[1].account_type = twice.holdings[0].account_type.clone();
    assert!(twice.validate_integrity().is_err());

    let mut types = p.clone();
    types.custom_types = vec![CustomType {
        id: "custom-1".to_owned(),
        name: "X".to_owned(),
        sellable: true,
    }];
    types.validate_integrity().unwrap();
    types.custom_types.push(CustomType {
        id: "custom-1".to_owned(),
        name: "Y".to_owned(),
        sellable: true,
    });
    assert!(types.validate_integrity().is_err());
}

// Made-up version 13 data: five accounts of different kinds, each with one record.
const V13: &str = r#"{
  "schema_version": 13, "next_id": 20,
  "categories": [{"id": 1, "name": "Cat", "target_weight": "1"}],
  "products": [{"id": 2, "name": "Fund X", "category": 1, "currency": "JPY"}],
  "accounts": [
    {"id": 3, "name": "A", "kind": "taxable", "sellable": true, "rounding_unit": "1"},
    {"id": 4, "name": "B", "kind": "nisa", "sellable": false, "rounding_unit": "1"},
    {"id": 5, "name": "C", "kind": "ideco", "sellable": true, "rounding_unit": "1"},
    {"id": 6, "name": "D", "kind": "points", "sellable": true, "rounding_unit": "1"},
    {"id": 7, "name": "E", "kind": "other", "sellable": true, "rounding_unit": "1"},
    {"id": 8, "name": "F", "kind": "ideco", "sellable": true, "rounding_unit": "1"}
  ],
  "holdings": [
    {"account": 3, "product": 2, "value": "10"},
    {"account": 4, "product": 2, "value": "20"},
    {"account": 5, "product": 2, "value": "30"},
    {"account": 6, "product": 2, "value": "40"},
    {"account": 7, "product": 2, "value": "50"},
    {"account": 8, "product": 2, "value": "60"}
  ]
}"#;

#[test]
fn a_version_13_file_gets_the_record_types_of_its_account_kinds() {
    let p = parse(V13.as_bytes()).unwrap();
    p.validate_integrity().unwrap();
    let t = |account: u64| {
        p.holdings
            .iter()
            .find(|h| h.account == AccountId(account))
            .unwrap()
            .account_type
            .clone()
    };
    assert_eq!(t(3).as_deref(), Some("specific"));
    assert_eq!(t(4).as_deref(), Some("nisa"));
    assert!(p.has_account_type("nisa") && !p.type_sellable(t(4).as_deref()));
    assert_eq!(t(7), None);
    // iDeCo and Points become custom types, one each, from the id counter.
    assert_eq!(p.custom_types.len(), 2);
    assert_eq!(t(5), t(8));
    assert_ne!(t(5), t(6));
    let ideco = p
        .custom_types
        .iter()
        .find(|c| Some(&c.id) == t(5).as_ref())
        .unwrap();
    assert_eq!(ideco.name, "iDeCo");
    assert_eq!(p.next_id, 22);
    // The accounts keep their selling switch.
    assert!(!p.accounts[1].sellable);
}

#[test]
fn a_file_without_custom_kinds_gets_no_custom_types_and_a_current_file_is_untouched() {
    let simple = r#"{"schema_version": 13, "next_id": 5,
      "categories": [{"id": 1, "name": "Cat", "target_weight": "1"}],
      "products": [{"id": 2, "name": "Fund X", "category": 1, "currency": "JPY"}],
      "accounts": [{"id": 3, "name": "A", "kind": "deposit", "sellable": true, "rounding_unit": "1"}],
      "holdings": [{"account": 3, "product": 2, "value": "10"}]}"#;
    let p = parse(simple.as_bytes()).unwrap();
    assert!(p.custom_types.is_empty());
    assert_eq!(p.next_id, 5);
    assert_eq!(p.holdings[0].account_type.as_deref(), Some("deposit"));

    // A current file is read as it is (a stray `kind` is not converted).
    let current = simple.replace("13", "14");
    let q = parse(current.as_bytes()).unwrap();
    assert_eq!(q.holdings[0].account_type, None);
    // The migrated data round-trips as a current file.
    let mut p = p;
    p.schema_version = SCHEMA_VERSION;
    let back = parse(serde_json::to_string(&p).unwrap().as_bytes()).unwrap();
    assert_eq!(back, p);
}

#[test]
fn a_newer_file_is_refused() {
    let newer = r#"{"schema_version": 21, "next_id": 1}"#;
    let p = parse(newer.as_bytes()).unwrap();
    assert!(matches!(
        p.validate_integrity(),
        Err(Error::UnsupportedVersion { found: 21 })
    ));
}

#[test]
fn setting_the_category_of_many_products_moves_them_all() {
    let (mut p, c, a, _) = sample();
    let c2 = category(&mut p, "Bonds", dec!(0));
    let b = product(&mut p, "Fund Y", c);
    let other = product(&mut p, "Fund Z", c);

    apply(
        &mut p,
        Edit::SetProductsCategory {
            products: vec![a, b, b],
            category: Some(c2),
            new_category: None,
        },
    );

    let cat = |id| p.products.iter().find(|x| x.id == id).unwrap().category;
    assert_eq!((cat(a), cat(b), cat(other)), (c2, c2, c));
    p.validate_integrity().unwrap();
}

#[test]
fn setting_the_category_can_make_a_new_one_in_the_same_edit() {
    let (mut p, _, a, _) = sample();
    let count = p.categories.len();

    apply(
        &mut p,
        Edit::SetProductsCategory {
            products: vec![a],
            category: None,
            new_category: Some("  Gold ".to_owned()),
        },
    );

    assert_eq!(p.categories.len(), count + 1);
    let made = p.categories.last().unwrap();
    assert_eq!(made.name, "Gold");
    assert_eq!((made.group, made.target_weight), (None, dec!(0)));
    assert_eq!(
        p.products.iter().find(|x| x.id == a).unwrap().category,
        made.id
    );
    p.validate_integrity().unwrap();
}

#[test]
fn setting_the_category_refuses_bad_input_and_changes_nothing() {
    let (mut p, c, a, _) = sample();
    let existing = p.categories[0].name.clone();
    let before = p.clone();
    let cases = [
        (vec![], Some(c), None),
        (vec![ProductId(9999)], Some(c), None),
        (vec![a], Some(CategoryId(9999)), None),
        (vec![a], None, None),
        (vec![a], Some(c), Some("Both".to_owned())),
        (vec![a], None, Some(existing)),
        (vec![a], None, Some("   ".to_owned())),
        (vec![a, ProductId(9999)], None, Some("New".to_owned())),
    ];
    for (products, category, new_category) in cases {
        assert!(
            p.apply(Edit::SetProductsCategory {
                products,
                category,
                new_category,
            })
            .is_err()
        );
        assert_eq!(p, before);
    }
}

fn checked_row(
    name: &str,
    value: rust_decimal::Decimal,
    computed: Option<rust_decimal::Decimal>,
) -> ImportRow {
    ImportRow {
        account_type: None,
        name: name.to_owned(),
        value,
        quantity: Some(dec!(10)),
        price: Some(dec!(100)),
        price_unit: 1,
        computed,
    }
}

#[test]
fn a_difference_beyond_the_smallest_unit_is_listed_and_the_value_is_kept() {
    let p = Portfolio::default();
    let plan = p.plan_import(
        "page",
        "JPY",
        &[
            // Within one yen: accepted.
            checked_row("Same", dec!(1000), Some(dec!(1000.6))),
            checked_row("OneOff", dec!(1000), Some(dec!(1001))),
            // More than a yen: listed.
            checked_row("Off", dec!(1000), Some(dec!(1002))),
            // Not checked: no computed value.
            checked_row("Unchecked", dec!(5), None),
        ],
    );
    assert_eq!(
        plan.differences,
        vec![ValueDifference {
            name: "Off".to_owned(),
            value: dec!(1000),
            computed: dec!(1002),
        }]
    );
    // The row itself is untouched: the evaluation is the value of record.
    assert!(
        plan.unmatched
            .iter()
            .any(|r| r.name == "Off" && r.value == dec!(1000))
    );
}

#[test]
fn the_tolerance_follows_the_currency_of_the_values() {
    let p = Portfolio::default();
    let rows = [
        checked_row("A", dec!(10.00), Some(dec!(10.01))),
        checked_row("B", dec!(10.00), Some(dec!(10.02))),
    ];
    let plan = p.plan_import("page", "USD", &rows);
    assert_eq!(plan.differences.len(), 1);
    assert_eq!(plan.differences[0].name, "B");
    // With no currency named and no product linked, a whole unit is the tolerance.
    assert!(p.plan_import("page", "", &rows).differences.is_empty());
}

#[test]
fn an_evaluation_column_needs_a_price_column_and_its_own_cells() {
    let mapping = |price_column, evaluation_column, evaluation_part| ImportMapping {
        source: "page".to_owned(),
        name_column: 0,
        value_column: 1,
        price_column,
        evaluation_column,
        evaluation_part,
        ..ImportMapping::default()
    };
    assert!(mapping(Some(2), Some(3), Some(0)).validated().is_ok());
    assert!(mapping(None, Some(3), None).validated().is_err());
    assert!(mapping(Some(2), Some(1), None).validated().is_err());
    assert!(mapping(Some(2), Some(2), None).validated().is_err());
    assert!(mapping(Some(2), None, Some(0)).validated().is_err());
    assert!(mapping(Some(2), Some(100_000), None).validated().is_err());
}

#[test]
fn a_file_of_the_previous_version_loads_and_a_mapping_without_the_check_stays_as_it_is() {
    let json = r#"{"schema_version":14,"next_id":1,"import_mappings":[{"source":"page","name_column":0,"value_column":1}]}"#;
    let p = parse(json.as_bytes()).unwrap();
    assert_eq!(p.import_mappings[0].evaluation_column, None);
    assert_eq!(p.import_mappings[0].evaluation_part, None);
    // Saved again without the new fields.
    let text = serde_json::to_string(&p.import_mappings[0]).unwrap();
    assert!(!text.contains("evaluation"));
}

fn typed_input(
    product: ProductId,
    account_type: Option<&str>,
    value: rust_decimal::Decimal,
) -> HoldingInput {
    HoldingInput {
        account_type: account_type.map(str::to_owned),
        product,
        value,
        quantity: None,
        price: None,
        price_unit: 1,
    }
}

#[test]
fn an_import_with_types_replaces_only_those_types_of_the_account() {
    let (mut p, _, prod, acc) = sample();
    // The sample holds the product untyped; add the same product under two types.
    for (ty, value) in [("nisa", dec!(10)), ("specific", dec!(20))] {
        apply(
            &mut p,
            Edit::SetHolding {
                account_type: Some(ty.to_owned()),
                account: acc,
                product: prod,
                value,
                quantity: None,
            },
        );
    }
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![typed_input(prod, Some("nisa"), dec!(99))],
            types: Some(vec![Some("nisa".to_owned())]),
        },
    );
    let value_of = |p: &Portfolio, ty: Option<&str>| {
        p.holdings
            .iter()
            .find(|h| h.account_type.as_deref() == ty)
            .map(|h| h.value)
    };
    assert_eq!(value_of(&p, Some("nisa")), Some(dec!(99)));
    assert_eq!(value_of(&p, Some("specific")), Some(dec!(20)));
    assert_eq!(value_of(&p, None), Some(dec!(1000)));
    assert_eq!(p.holdings.len(), 3);

    // A type that is in scope but no longer on the page is removed; the others stay.
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![],
            types: Some(vec![Some("nisa".to_owned())]),
        },
    );
    assert_eq!(value_of(&p, Some("nisa")), None);
    assert_eq!(p.holdings.len(), 2);
}

#[test]
fn an_import_with_types_rejects_a_record_outside_them_and_keeps_no_inferred_type() {
    let (mut p, _, prod, acc) = sample();
    let before = p.clone();
    assert!(
        p.apply(Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![typed_input(prod, Some("specific"), dec!(5))],
            types: Some(vec![Some("nisa".to_owned())]),
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![],
            types: Some(vec![Some("no-such-type".to_owned())]),
        })
        .is_err()
    );
    assert_eq!(p, before);

    // An untyped input is untyped when the import is limited to types (no type is inferred from
    // the existing record, which is the whole-account behavior).
    apply(
        &mut p,
        Edit::SetHolding {
            account_type: Some("nisa".to_owned()),
            account: acc,
            product: prod,
            value: dec!(7),
            quantity: None,
        },
    );
    apply(
        &mut p,
        Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![typed_input(prod, None, dec!(3))],
            types: Some(vec![None]),
        },
    );
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account_type.is_none() && h.value == dec!(3))
    );
    assert!(
        p.holdings
            .iter()
            .any(|h| h.account_type.as_deref() == Some("nisa") && h.value == dec!(7))
    );
}

#[test]
fn plan_import_keeps_rows_of_one_product_with_two_types_apart() {
    let (mut p, _, prod, _) = sample();
    apply(
        &mut p,
        Edit::AddAlias {
            product: prod,
            source: "Made Up".to_owned(),
            name: "Fund X".to_owned(),
        },
    );
    let row = |ty: Option<&str>, value| ImportRow {
        account_type: ty.map(str::to_owned),
        name: "Fund X".to_owned(),
        value,
        quantity: None,
        price: None,
        price_unit: 1,
        computed: None,
    };
    let rows = [
        row(Some("nisa"), dec!(1)),
        row(Some("specific"), dec!(2)),
        row(Some("nisa"), dec!(4)),
    ];
    let plan = p.plan_import("Made Up", "", &rows);
    assert_eq!(plan.matched.len(), 2);
    assert_eq!(plan.matched[0].account_type.as_deref(), Some("nisa"));
    assert_eq!(plan.matched[0].value, dec!(5));
    assert_eq!(plan.matched[1].value, dec!(2));
}

#[test]
fn a_table_title_stands_for_the_type_of_the_longest_matching_rule() {
    let mut recipe = recipe_for(AccountId(1));
    recipe.type_rules = vec![
        TypeRule {
            title: "Stocks (cash/ NISA".to_owned(),
            account_type: "nisa".to_owned(),
        },
        TypeRule {
            title: "Stocks (cash/NISA (growth))".to_owned(),
            account_type: "nisa_growth".to_owned(),
        },
    ];
    assert_eq!(
        recipe.type_of_title("Stocks (cash/ NISA (growth))"),
        Some("nisa_growth")
    );
    assert_eq!(recipe.type_of_title("Stocks (cash/NISA)"), Some("nisa"));
    assert_eq!(recipe.type_of_title("Stocks (cash/ taxable)"), None);
}

#[test]
fn type_rules_are_checked_when_a_recipe_is_saved() {
    let (mut p, _, _, acc) = sample();
    let rules = |title: &str, ty: &str| {
        vec![TypeRule {
            title: title.to_owned(),
            account_type: ty.to_owned(),
        }]
    };
    let mut recipe = recipe_for(acc);
    recipe.all_tables = true;
    recipe.type_rules = rules(" Stocks (cash/NISA) ", "nisa");
    apply(
        &mut p,
        Edit::SetRecipe {
            recipe: recipe.clone(),
        },
    );
    assert_eq!(p.recipes[0].type_rules[0].title, "Stocks (cash/NISA)");

    for bad in [
        rules("Stocks", "no-such-type"),
        rules("  ", "nisa"),
        [rules("A b", "nisa"), rules("Ab", "specific")].concat(),
    ] {
        let mut r = recipe.clone();
        r.type_rules = bad;
        assert!(p.apply(Edit::SetRecipe { recipe: r }).is_err());
    }
    // Rules name the tables of a page read with every matching table.
    let mut r = recipe;
    r.all_tables = false;
    assert!(p.apply(Edit::SetRecipe { recipe: r }).is_err());
}

#[test]
fn removing_a_custom_type_removes_the_rules_that_name_it() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddAccountType {
            name: "Made Up Type".to_owned(),
            sellable: true,
        },
    );
    let id = p.custom_types[0].id.clone();
    let mut recipe = recipe_for(acc);
    recipe.all_tables = true;
    recipe.type_rules = vec![TypeRule {
        title: "Stocks (x".to_owned(),
        account_type: id.clone(),
    }];
    apply(&mut p, Edit::SetRecipe { recipe });
    apply(&mut p, Edit::RemoveAccountType { id });
    assert!(p.recipes[0].type_rules.is_empty());
}

#[test]
fn the_sbi_presets_read_the_type_from_the_table_title() {
    let (mut p, _, _, acc) = sample();
    apply(
        &mut p,
        Edit::AddPresetRecipe {
            preset: "sbi-funds".to_owned(),
            account: acc,
        },
    );
    // The stored recipe holds no definition; the effective one has the rules.
    assert!(p.recipes[0].type_rules.is_empty());
    let effective = p.recipes[0].effective().unwrap();
    assert!(effective.all_tables);
    assert_eq!(
        effective.type_of_title("投資信託（金額/NISA預り（成長投資枠））"),
        Some("nisa_growth")
    );
    assert_eq!(
        effective.type_of_title("投資信託（金額/旧NISA預り）"),
        Some("old_nisa")
    );
    for rule in &effective.type_rules {
        assert!(p.has_account_type(&rule.account_type));
    }
}

#[test]
fn the_rebalancing_account_of_an_older_file_is_dropped() {
    let old = r#"{"schema_version": 16, "next_id": 5,
      "accounts": [{"id": 3, "name": "A", "sellable": true, "rounding_unit": "1"}],
      "settings": {"category_tolerance": "0.01", "product_tolerance": "0", "target_account": 3}}"#;

    let mut p = parse(old.as_bytes()).unwrap();
    p.schema_version = SCHEMA_VERSION;

    p.validate_integrity().unwrap();
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["settings"].get("target_account").is_none());
}

#[test]
fn the_buyable_products_of_an_older_file_join_the_exclusion_list() {
    let old = r#"{"schema_version": 18, "next_id": 20,
      "products": [
        {"id": 1, "name": "A", "category": 5, "currency": "JPY"},
        {"id": 2, "name": "B", "category": 5, "currency": "JPY"},
        {"id": 3, "name": "C", "category": 5, "currency": "JPY"}],
      "categories": [{"id": 5, "name": "X", "target_weight": "1"}],
      "accounts": [
        {"id": 10, "name": "Limited", "sellable": true, "rounding_unit": "1", "tradable": [1, 2], "excluded": [2]},
        {"id": 11, "name": "Any", "sellable": true, "rounding_unit": "1", "tradable": null, "excluded": [3]},
        {"id": 12, "name": "Unset", "sellable": true, "rounding_unit": "1"}]}"#;

    let mut p = parse(old.as_bytes()).unwrap();
    p.schema_version = SCHEMA_VERSION;

    p.validate_integrity().unwrap();
    // Takes part = buyable (listed, or no list) and not excluded.
    assert_eq!(p.accounts[0].excluded, vec![ProductId(2), ProductId(3)]);
    assert_eq!(p.accounts[1].excluded, vec![ProductId(3)]);
    assert!(p.accounts[2].excluded.is_empty());
    let json = serde_json::to_value(&p).unwrap();
    assert!(json["accounts"][0].get("tradable").is_none());
}

#[test]
fn a_malformed_exclusion_list_is_not_rewritten_by_the_migration() {
    let old = r#"{"schema_version": 18, "next_id": 20,
      "products": [{"id": 1, "name": "A", "category": 5, "currency": "JPY"}],
      "categories": [{"id": 5, "name": "X", "target_weight": "1"}],
      "accounts": [{"id": 10, "name": "A", "sellable": true, "rounding_unit": "1", "tradable": [1], "excluded": "oops"}]}"#;

    assert!(parse(old.as_bytes()).is_err());
}
