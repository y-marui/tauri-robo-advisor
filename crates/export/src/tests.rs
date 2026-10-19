// Every name and amount in these tests is made up.

use portfolio::{AccountKind, Edit, Portfolio, ProductTargeting};
use rust_decimal::{Decimal, dec};

use crate::*;

const INFO: Info = Info {
    app_version: "0.1.0",
    exported_at: "2026-10-15T01:23:45Z",
};

fn apply(p: &mut Portfolio, edit: Edit) {
    p.apply(edit).unwrap();
}

/// Two categories, two products (one in USD), two accounts, three holdings.
fn sample() -> Portfolio {
    let mut p = Portfolio::default();
    apply(
        &mut p,
        Edit::AddCategory {
            name: "Stocks".to_owned(),
            target_weight: dec!(0.75),
        },
    );
    apply(
        &mut p,
        Edit::AddCategory {
            name: "Bonds".to_owned(),
            target_weight: dec!(0.25),
        },
    );
    let (stocks, bonds) = (p.categories[0].id, p.categories[1].id);
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Stock Fund".to_owned(),
            category: stocks,
            currency: "JPY".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddProduct {
            name: "US Fund".to_owned(),
            category: stocks,
            currency: "USD".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Bond Fund".to_owned(),
            category: bonds,
            currency: "JPY".to_owned(),
        },
    );
    let (s1, us, b1) = (p.products[0].id, p.products[1].id, p.products[2].id);
    apply(
        &mut p,
        Edit::AddAccount {
            name: "Main".to_owned(),
            kind: AccountKind::Taxable,
        },
    );
    apply(
        &mut p,
        Edit::AddAccount {
            name: "Tax-free".to_owned(),
            kind: AccountKind::Nisa,
        },
    );
    let (main, nisa) = (p.accounts[0].id, p.accounts[1].id);
    apply(
        &mut p,
        Edit::SetTradable {
            id: nisa,
            products: Some(vec![s1, b1]),
        },
    );
    for (account, product, value) in [
        (main, s1, dec!(500000)),
        (main, us, dec!(1000)),
        (nisa, b1, dec!(100000)),
    ] {
        apply(
            &mut p,
            Edit::SetHolding {
                account,
                product,
                value,
                quantity: None,
            },
        );
    }
    apply(
        &mut p,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150)),
        },
    );
    apply(
        &mut p,
        Edit::AddAlias {
            product: s1,
            source: "Broker 1".to_owned(),
            name: "STOCK FUND (A)".to_owned(),
        },
    );
    p
}

fn files(p: &Portfolio) -> std::collections::BTreeMap<&'static str, String> {
    build(p, &INFO)
        .unwrap()
        .into_iter()
        .map(|f| (f.name, String::from_utf8(f.content).unwrap()))
        .collect()
}

fn lines(text: &str) -> Vec<&str> {
    text.trim_start_matches('\u{feff}')
        .split("\r\n")
        .filter(|l| !l.is_empty())
        .collect()
}

#[test]
fn every_expected_file_is_produced() {
    let names: Vec<_> = build(&sample(), &INFO)
        .unwrap()
        .into_iter()
        .map(|f| f.name)
        .collect();

    assert_eq!(
        names,
        [
            "holdings.csv",
            "categories.csv",
            "products.csv",
            "aliases.csv",
            "accounts.csv",
            "settings.csv",
            "portfolio.json",
            "README.txt"
        ]
    );
}

#[test]
fn csv_files_start_with_a_byte_order_mark_and_use_crlf() {
    for file in build(&sample(), &INFO)
        .unwrap()
        .iter()
        .filter(|f| f.name.ends_with(".csv"))
    {
        assert_eq!(&file.content[..3], b"\xEF\xBB\xBF", "{}", file.name);
        let text = String::from_utf8(file.content.clone()).unwrap();
        assert!(!text.replace("\r\n", "").contains('\n'), "{}", file.name);
    }
}

#[test]
fn holdings_have_one_row_per_holding_with_jpy_values_and_shares() {
    let rows = lines(&files(&sample())["holdings.csv"])
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();

    assert_eq!(
        rows[0],
        "account_id,account,account_kind,product_id,product,category_id,category,currency,value,quantity,price,price_unit,value_jpy,share_of_total"
    );
    assert_eq!(rows.len(), 4);
    // Total = 500,000 + 150,000 (1,000 USD at 150) + 100,000 = 750,000.
    assert_eq!(
        rows[1],
        "6,Main,taxable,3,Stock Fund,1,Stocks,JPY,500000,,,1,500000,0.666667"
    );
    assert_eq!(
        rows[2],
        "6,Main,taxable,4,US Fund,1,Stocks,USD,1000,,,1,150000,0.2"
    );
    assert_eq!(
        rows[3],
        "7,Tax-free,nisa,5,Bond Fund,2,Bonds,JPY,100000,,,1,100000,0.133333"
    );
}

#[test]
fn categories_products_and_accounts_carry_their_totals() {
    let f = files(&sample());

    assert_eq!(
        lines(&f["categories.csv"]),
        [
            "category_id,category,target_weight,product_targeting,value_jpy,current_share",
            "1,Stocks,0.75,free,650000,0.866667",
            "2,Bonds,0.25,free,100000,0.133333",
        ]
    );
    assert_eq!(
        lines(&f["products.csv"]),
        [
            "product_id,product,category_id,category,currency,relative_weight,value_jpy",
            "3,Stock Fund,1,Stocks,JPY,,500000",
            "4,US Fund,1,Stocks,USD,,150000",
            "5,Bond Fund,2,Bonds,JPY,,100000",
        ]
    );
    assert_eq!(
        lines(&f["accounts.csv"]),
        [
            "account_id,account,kind,sellable,funding,rounding_unit,tradable_product_ids,value_jpy",
            "6,Main,taxable,true,cash,1000,all,650000",
            "7,Tax-free,nisa,false,cash,1000,3;5,100000",
        ]
    );
}

#[test]
fn aliases_and_settings_are_exported() {
    let mut p = sample();
    let main = p.accounts[0].id;
    apply(
        &mut p,
        Edit::SetTargetAccount {
            account: Some(main),
        },
    );
    let f = files(&p);

    assert_eq!(
        lines(&f["aliases.csv"]),
        [
            "product_id,product,source,alias",
            "3,Stock Fund,Broker 1,STOCK FUND (A)"
        ]
    );
    assert_eq!(
        lines(&f["settings.csv"]),
        [
            "setting,value",
            &format!("schema_version,{}", portfolio::SCHEMA_VERSION),
            "usd_jpy,150",
            "category_tolerance,0.01",
            "product_tolerance,0",
            "rebalancing_account_id,6",
            "rebalancing_account,Main",
        ]
    );
}

#[test]
fn a_weighted_category_exports_its_targeting_and_product_weights() {
    let mut p = sample();
    let stocks = p.categories[0].id;
    let s1 = p.products[0].id;
    apply(
        &mut p,
        Edit::UpdateCategory {
            id: stocks,
            name: None,
            target_weight: None,
            targeting: Some(ProductTargeting::Weighted),
        },
    );
    apply(
        &mut p,
        Edit::SetProductWeight {
            id: s1,
            weight: Some(dec!(2.50)),
        },
    );
    let f = files(&p);

    assert!(lines(&f["categories.csv"])[1].contains(",weighted,"));
    assert!(lines(&f["products.csv"])[1].contains(",2.5,"));
}

#[test]
fn names_that_look_like_formulas_are_written_as_text() {
    let mut p = Portfolio::default();
    apply(
        &mut p,
        Edit::AddCategory {
            name: "=cmd|' /C calc'!A0".to_owned(),
            target_weight: dec!(1),
        },
    );
    let c = p.categories[0].id;
    apply(
        &mut p,
        Edit::AddProduct {
            name: "+1+1".to_owned(),
            category: c,
            currency: "JPY".to_owned(),
        },
    );
    let prod = p.products[0].id;
    apply(
        &mut p,
        Edit::AddAccount {
            name: "@SUM(1)".to_owned(),
            kind: AccountKind::Other,
        },
    );
    let acc = p.accounts[0].id;
    apply(
        &mut p,
        Edit::SetHolding {
            account: acc,
            product: prod,
            value: dec!(1),
            quantity: None,
        },
    );
    apply(
        &mut p,
        Edit::AddAlias {
            product: prod,
            source: "-src".to_owned(),
            name: "=alias".to_owned(),
        },
    );
    let f = files(&p);

    let all = f
        .iter()
        .filter(|(name, _)| name.ends_with(".csv"))
        .map(|(_, t)| t.as_str())
        .collect::<Vec<_>>()
        .join("\n");
    for cell in all.split([',', '\r', '\n']) {
        let cell = cell.trim_matches('"');
        assert!(
            !cell.starts_with(['=', '+', '-', '@']) || cell.parse::<f64>().is_ok(),
            "unsafe cell: {cell}"
        );
    }
    assert!(f["holdings.csv"].contains("'@SUM(1)"));
    assert!(f["aliases.csv"].contains("'-src,'=alias"));
}

#[test]
fn a_name_with_a_comma_or_quote_stays_in_one_cell() {
    let mut p = Portfolio::default();
    apply(
        &mut p,
        Edit::AddCategory {
            name: "Stocks, \"core\"".to_owned(),
            target_weight: dec!(1),
        },
    );

    let f = files(&p);

    assert_eq!(
        lines(&f["categories.csv"])[1],
        "1,\"Stocks, \"\"core\"\"\",1,free,0,"
    );
}

#[test]
fn values_that_cannot_be_converted_are_left_empty_not_guessed() {
    let mut p = sample();
    let stocks = p.categories[0].id;
    apply(
        &mut p,
        Edit::AddProduct {
            name: "EU Fund".to_owned(),
            category: stocks,
            currency: "EUR".to_owned(),
        },
    );
    let (eu, main) = (p.products.last().unwrap().id, p.accounts[0].id);
    apply(
        &mut p,
        Edit::SetHolding {
            account: main,
            product: eu,
            value: dec!(10),
            quantity: Some(dec!(2)),
        },
    );
    let f = files(&p);

    // The EUR holding has no JPY value, so every total that includes it is empty.
    let holdings = lines(&f["holdings.csv"])
        .into_iter()
        .map(str::to_owned)
        .collect::<Vec<_>>();
    assert!(
        holdings
            .iter()
            .any(|l| l.contains("EU Fund") && l.ends_with(",10,2,,1,,"))
    );
    assert_eq!(lines(&f["categories.csv"])[1], "1,Stocks,0.75,free,,");
    assert_eq!(
        lines(&f["accounts.csv"])[1],
        "6,Main,taxable,true,cash,1000,all,"
    );
    // Shares need the total, which is also unknown.
    assert!(holdings[1].ends_with(",500000,"));
}

#[test]
fn a_missing_usd_rate_leaves_usd_values_empty() {
    let mut p = sample();
    apply(&mut p, Edit::SetExchangeRate { usd_jpy: None });

    let f = files(&p);

    assert!(lines(&f["holdings.csv"])[2].ends_with(",USD,1000,,,1,,"));
    assert_eq!(lines(&f["settings.csv"])[2], "usd_jpy,");
}

#[test]
fn json_holds_the_complete_data_and_round_trips() {
    let p = sample();

    let json = files(&p)["portfolio.json"].clone();
    let back: Portfolio = serde_json::from_str(&json).unwrap();

    assert_eq!(back, p);
}

#[test]
fn the_readme_states_the_version_time_and_a_privacy_warning() {
    let readme = files(&sample())["README.txt"].clone();

    assert!(readme.contains("App version:    0.1.0"));
    assert!(readme.contains("2026-10-15T01:23:45Z"));
    assert!(readme.contains("contain your holdings"));
    assert!(readme.contains("3 products, 2 accounts, and 3 holdings"));
}

#[test]
fn an_empty_portfolio_exports_headers_only() {
    let f = files(&Portfolio::default());

    for name in [
        "holdings.csv",
        "categories.csv",
        "products.csv",
        "aliases.csv",
        "accounts.csv",
    ] {
        assert_eq!(lines(&f[name]).len(), 1, "{name}");
    }
}

#[test]
fn astronomical_values_do_not_panic() {
    let mut p = Portfolio::default();
    apply(
        &mut p,
        Edit::AddCategory {
            name: "A".to_owned(),
            target_weight: dec!(1),
        },
    );
    let c = p.categories[0].id;
    apply(
        &mut p,
        Edit::AddProduct {
            name: "P".to_owned(),
            category: c,
            currency: "JPY".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddProduct {
            name: "Q".to_owned(),
            category: c,
            currency: "JPY".to_owned(),
        },
    );
    apply(
        &mut p,
        Edit::AddAccount {
            name: "Acc".to_owned(),
            kind: AccountKind::Other,
        },
    );
    let (p1, p2, acc) = (p.products[0].id, p.products[1].id, p.accounts[0].id);
    for product in [p1, p2] {
        apply(
            &mut p,
            Edit::SetHolding {
                account: acc,
                product,
                value: Decimal::MAX,
                quantity: None,
            },
        );
    }

    let f = files(&p);

    // The total overflows, so shares are empty instead of wrong.
    assert!(lines(&f["holdings.csv"])[1].ends_with(","));
}
