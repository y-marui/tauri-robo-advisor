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

fn account(portfolio: &mut Portfolio, name: &str, kind: AccountKind) -> AccountId {
    apply(
        portfolio,
        Edit::AddAccount {
            name: name.to_owned(),
            kind,
        },
    );
    portfolio.accounts.last().unwrap().id
}

fn sample() -> (Portfolio, CategoryId, ProductId, AccountId) {
    let mut p = Portfolio::default();
    let c = category(&mut p, "Category A", dec!(1));
    let prod = product(&mut p, "Fund X", c);
    let acc = account(&mut p, "Account 1", AccountKind::Taxable);
    apply(
        &mut p,
        Edit::SetHolding {
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
            targeting: None,
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
fn removing_a_product_removes_its_holdings_and_tradable_entries() {
    let (mut p, _, prod, acc) = sample();
    apply(
        &mut p,
        Edit::SetTradable {
            id: acc,
            products: Some(vec![prod]),
        },
    );

    apply(&mut p, Edit::RemoveProduct { id: prod });

    assert!(p.holdings.is_empty());
    assert_eq!(p.accounts[0].tradable, Some(vec![]));
    p.validate_integrity().unwrap();
}

#[test]
fn removing_an_account_removes_its_holdings_and_clears_the_target_account() {
    let (mut p, _, _, acc) = sample();
    apply(&mut p, Edit::SetTargetAccount { account: Some(acc) });

    apply(&mut p, Edit::RemoveAccount { id: acc });

    assert!(p.holdings.is_empty());
    assert_eq!(p.settings.target_account, None);
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
fn nisa_accounts_default_to_no_selling_and_points_accounts_to_points() {
    let mut p = Portfolio::default();
    let nisa = account(&mut p, "Tax-free", AccountKind::Nisa);
    account(&mut p, "Points", AccountKind::Points);
    let taxable = account(&mut p, "Taxable", AccountKind::Taxable);

    let find = |id| p.accounts.iter().find(|a| a.id == id).unwrap();
    assert!(!find(nisa).sellable);
    assert!(find(taxable).sellable);
    assert_eq!(find(taxable).rounding_unit, dec!(1));
}

#[test]
fn set_holding_updates_in_place_and_rejects_negative_values() {
    let (mut p, _, prod, acc) = sample();

    apply(
        &mut p,
        Edit::SetHolding {
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
            account: acc,
            product: prod,
        },
    );
    assert!(p.holdings.is_empty());
}

#[test]
fn set_tradable_checks_products_and_removes_duplicates() {
    let (mut p, _, prod, acc) = sample();

    assert!(
        p.apply(Edit::SetTradable {
            id: acc,
            products: Some(vec![ProductId(999)])
        })
        .is_err()
    );
    apply(
        &mut p,
        Edit::SetTradable {
            id: acc,
            products: Some(vec![prod, prod]),
        },
    );

    assert_eq!(p.accounts[0].tradable, Some(vec![prod]));
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
fn issues_report_weighted_categories_with_missing_weights_or_no_products() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "A", dec!(0.5));
    let empty = category(&mut p, "B", dec!(0.5));
    let prod = product(&mut p, "Fund X", c);
    let acc = account(&mut p, "Main", AccountKind::Taxable);
    for id in [c, empty] {
        apply(
            &mut p,
            Edit::UpdateCategory {
                id,
                name: None,
                target_weight: None,
                targeting: Some(ProductTargeting::Weighted),
            },
        );
    }

    let issues = p.issues();
    assert!(issues.contains(&Issue::WeightedProductMissingWeight { category: c }));
    assert!(issues.contains(&Issue::CategoryHasNoProducts { category: empty }));

    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account: acc,
            product: prod,
            weight: Some(dec!(10)),
        },
    );
    assert!(
        !p.issues()
            .contains(&Issue::WeightedProductMissingWeight { category: c })
    );
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

    let mut p = good;
    p.settings.target_account = Some(AccountId(999));
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
    let other = account(&mut p, "Other", AccountKind::Taxable);
    let second = product(&mut p, "Second", cat);
    apply(
        &mut p,
        Edit::SetHolding {
            account: acc,
            product: prod,
            value: dec!(100),
            quantity: Some(dec!(1)),
        },
    );
    apply(
        &mut p,
        Edit::SetHolding {
            account: other,
            product: prod,
            value: dec!(7),
            quantity: None,
        },
    );

    apply(
        &mut p,
        Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![HoldingInput {
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
            account: acc,
            product: prod,
            value: dec!(100),
            quantity: None,
        },
    );
    let before = p.clone();
    let input = |value| HoldingInput {
        product: prod,
        value,
        quantity: None,
        price: None,
        price_unit: 1,
    };

    assert!(
        p.apply(Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![input(dec!(1)), input(dec!(2))],
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![input(dec!(-1))],
        })
        .is_err()
    );
    assert!(
        p.apply(Edit::ReplaceHoldings {
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
        name: name.to_owned(),
        value: dec!(10),
        quantity: None,
        price: None,
        price_unit: 1,
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
        name: name.to_owned(),
        value,
        quantity: None,
        price: None,
        price_unit: 1,
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
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
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
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
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
            loose_numbers: false,
            quantity_column: None,
            quantity_part: None,
            max_records: None,
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
        loose_numbers: false,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
    }
}

fn recipe_for(account: AccountId) -> Recipe {
    Recipe {
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
        start_site: String::new(),
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
            loose_numbers: false,
            quantity_column: None,
            quantity_part: None,
            max_records: None,
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
            kind: AccountKind::Taxable,
        },
    );
    apply(
        &mut p,
        Edit::AddAccount {
            name: "B".to_owned(),
            kind: AccountKind::Taxable,
        },
    );
    assert_eq!(p.revision, 2);

    assert!(
        p.apply(Edit::AddAccount {
            name: "A".to_owned(),
            kind: AccountKind::Taxable
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
        loose_numbers: m.loose_numbers,
        quantity_column: None,
        quantity_part: None,
        max_records: None,
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
        name: name.to_owned(),
        value: dec!(100),
        quantity,
        price,
        price_unit: 1,
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
        name: "D".to_owned(),
        value: dec!(10),
        quantity: None,
        price: None,
        price_unit: 1,
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
            account: acc,
            holdings: vec![HoldingInput {
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
            product: prod,
            value: dec!(1),
            quantity: None,
            price: None,
            price_unit: 0,
        },
        HoldingInput {
            product: prod,
            value: dec!(1),
            quantity: None,
            price: Some(dec!(-1)),
            price_unit: 1,
        },
    ] {
        assert!(
            p.apply(Edit::ReplaceHoldings {
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
fn a_recipe_may_leave_its_currency_empty() {
    let (mut p, _, _, acc) = sample();
    let mut recipe = recipe_for(acc);
    recipe.currency = " ".to_owned();

    apply(&mut p, Edit::SetRecipe { recipe });

    assert_eq!(p.recipes[0].currency, "");
}

#[test]
fn a_preset_is_never_added_twice_for_one_account() {
    let (mut p, _, _, a) = sample();
    let b = account(&mut p, "Account 2", AccountKind::Taxable);
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
    assert_eq!(SCHEMA_VERSION, 11);
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
    let a = account(&mut p, "A", AccountKind::Taxable);
    let b = account(&mut p, "B", AccountKind::Taxable);
    let c = account(&mut p, "C", AccountKind::Taxable);
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

fn weight_edit(
    account: AccountId,
    product: ProductId,
    weight: Option<rust_decimal::Decimal>,
) -> Edit {
    Edit::SetHoldingWeight {
        account,
        product,
        weight,
    }
}

#[test]
fn a_weight_belongs_to_the_record_of_a_product_in_an_account() {
    let (mut p, _, prod, acc) = sample();
    let other = account(&mut p, "Account 2", AccountKind::Nisa);

    apply(&mut p, weight_edit(acc, prod, Some(dec!(3))));
    // The same product in another account has its own weight; none of it is held yet.
    apply(&mut p, weight_edit(other, prod, Some(dec!(1))));

    assert_eq!(p.record_weight(acc, prod), Some(dec!(3)));
    assert_eq!(p.record_weight(other, prod), Some(dec!(1)));
    assert_eq!(p.product_weight(prod), Some(dec!(4)));
    let made = p
        .holdings
        .iter()
        .find(|h| h.account == other && h.product == prod)
        .unwrap();
    assert_eq!(made.value, dec!(0));
    p.validate_integrity().unwrap();

    // Clearing a weight keeps the record; a weight that is not positive is refused.
    apply(&mut p, weight_edit(acc, prod, None));
    assert_eq!(p.product_weight(prod), Some(dec!(1)));
    let before = p.clone();
    assert!(p.apply(weight_edit(acc, prod, Some(dec!(0)))).is_err());
    assert!(
        p.apply(weight_edit(AccountId(9999), prod, Some(dec!(1))))
            .is_err()
    );
    assert_eq!(p, before);

    // Clearing the weight of a record that does not exist makes nothing.
    let third = account(&mut p, "Account 3", AccountKind::Taxable);
    let count = p.holdings.len();
    apply(&mut p, weight_edit(third, prod, None));
    assert_eq!(p.holdings.len(), count);
    assert_eq!(p.product_weight(prod), Some(dec!(1)));
}

#[test]
fn an_import_keeps_the_weights_of_the_records_it_replaces() {
    let (mut p, c, prod, acc) = sample();
    let sold = product(&mut p, "Fund Y", c);
    let gone = product(&mut p, "Fund Z", c);
    apply(&mut p, weight_edit(acc, prod, Some(dec!(2))));
    apply(&mut p, weight_edit(acc, sold, Some(dec!(5))));
    apply(
        &mut p,
        Edit::SetHolding {
            account: acc,
            product: sold,
            value: dec!(700),
            quantity: Some(dec!(7)),
        },
    );
    apply(
        &mut p,
        Edit::SetHolding {
            account: acc,
            product: gone,
            value: dec!(300),
            quantity: None,
        },
    );

    apply(
        &mut p,
        Edit::ReplaceHoldings {
            account: acc,
            holdings: vec![HoldingInput {
                product: prod,
                value: dec!(1500),
                quantity: None,
                price: None,
                price_unit: 1,
            }],
        },
    );

    let record = |product| p.holdings.iter().find(|h| h.product == product);
    // Listed again: the new value, the old weight.
    assert_eq!(record(prod).unwrap().value, dec!(1500));
    assert_eq!(record(prod).unwrap().weight, Some(dec!(2)));
    // No longer listed but wanted: nothing held, the weight stays.
    let kept = record(sold).unwrap();
    assert_eq!(
        (kept.value, kept.quantity, kept.weight),
        (dec!(0), None, Some(dec!(5)))
    );
    // No longer listed and not wanted: gone.
    assert!(record(gone).is_none());
    p.validate_integrity().unwrap();
}

#[test]
fn removing_a_record_removes_its_weight() {
    let (mut p, _, prod, acc) = sample();
    apply(&mut p, weight_edit(acc, prod, Some(dec!(2))));
    apply(
        &mut p,
        Edit::RemoveHolding {
            account: acc,
            product: prod,
        },
    );
    assert_eq!(p.product_weight(prod), None);
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
fn the_weight_of_a_product_does_not_overflow_when_its_records_add_up() {
    let (mut p, _, prod, acc) = sample();
    let other = account(&mut p, "Account 2", AccountKind::Nisa);
    apply(
        &mut p,
        weight_edit(acc, prod, Some(rust_decimal::Decimal::MAX)),
    );
    apply(
        &mut p,
        weight_edit(other, prod, Some(rust_decimal::Decimal::MAX)),
    );

    assert_eq!(p.product_weight(prod), Some(rust_decimal::Decimal::MAX));
    // The issues are worked out on every view, so they must not panic either.
    let _ = p.issues();
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
