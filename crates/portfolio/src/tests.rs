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
    let json = r#"{"schema_version":1,"next_id":1}"#;

    let p: Portfolio = serde_json::from_str(json).unwrap();

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
