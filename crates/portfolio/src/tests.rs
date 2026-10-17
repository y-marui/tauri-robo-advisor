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
    let points = account(&mut p, "Points", AccountKind::Points);
    let taxable = account(&mut p, "Taxable", AccountKind::Taxable);

    let find = |id| p.accounts.iter().find(|a| a.id == id).unwrap();
    assert!(!find(nisa).sellable);
    assert_eq!(find(points).funding, Funding::Points);
    assert!(find(taxable).sellable);
    assert_eq!(find(taxable).rounding_unit, dec!(1000));
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
        Edit::SetProductWeight {
            id: prod,
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
    }
}

fn recipe_for(account: AccountId) -> Recipe {
    Recipe {
        name: " Domestic stocks ".to_owned(),
        links: vec![
            FetchLink {
                text: " Account ".to_owned(),
                reference: None,
            },
            FetchLink {
                text: "Yen account".to_owned(),
                reference: Some("/a/list".to_owned()),
            },
        ],
        table_starts_with: " Stocks (".to_owned(),
        currency: "JPY".to_owned(),
        account,
        mapping: mapping_for(" SBI "),
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
            reference: None
        };
        1
    ]));
    assert!(with(&|r| r.links = vec![
        FetchLink {
            text: "".to_owned(),
            reference: None
        };
        1
    ]));
    assert!(with(&|r| r.links = vec![
        FetchLink {
            text: "a".to_owned(),
            reference: None
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
