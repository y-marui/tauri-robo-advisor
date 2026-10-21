// Every name and amount in these tests is made up.

use portfolio::{AccountId, Edit, Portfolio, ProductId, Strategy, StrategyItem};
use rust_decimal::{Decimal, dec};

use crate::*;

struct Fixture {
    portfolio: Portfolio,
    product: ProductId,
    main: AccountId,
}

fn add_account(p: &mut Portfolio, name: &str, unit: Decimal) -> AccountId {
    p.apply(Edit::AddAccount {
        name: name.to_owned(),
    })
    .unwrap();
    let id = p.accounts.last().unwrap().id;
    p.apply(Edit::UpdateAccount {
        id,
        name: None,
        sellable: None,
        rounding_unit: Some(unit),
    })
    .unwrap();
    id
}

fn hold(p: &mut Portfolio, account: AccountId, product: ProductId, kind: Option<&str>, v: Decimal) {
    p.apply(Edit::SetHolding {
        account_type: kind.map(str::to_owned),
        account,
        product,
        value: v,
        quantity: None,
    })
    .unwrap();
}

fn sellable(p: &mut Portfolio, id: &str, sellable: bool) {
    p.apply(Edit::SetTypeSellable {
        id: id.to_owned(),
        sellable,
    })
    .unwrap();
}

/// One category with one product; the main account (unit 1,000) holds it as general and NISA.
fn fixture(general: Decimal, nisa: Decimal) -> Fixture {
    let mut p = Portfolio::default();
    p.apply(Edit::AddCategory {
        name: "All".to_owned(),
        target_weight: dec!(1),
    })
    .unwrap();
    let category = p.categories[0].id;
    p.apply(Edit::AddProduct {
        name: "Fund".to_owned(),
        category,
        currency: "JPY".to_owned(),
    })
    .unwrap();
    let product = p.products[0].id;
    let main = add_account(&mut p, "Main", dec!(1000));
    hold(&mut p, main, product, Some("general"), general);
    hold(&mut p, main, product, Some("nisa"), nisa);
    Fixture {
        portfolio: p,
        product,
        main,
    }
}

fn sell(
    p: &Portfolio,
    account: Option<AccountId>,
    method: Method,
    amount: Decimal,
) -> Result<Suggestion, Error> {
    suggest(
        p,
        &Request {
            account,
            mode: Mode::FixedSell { amount },
            held_only: false,
            method,
        },
    )
}

/// (type, delta, after) of the lines of an account, in the order of the result.
fn lines(s: &Suggestion, account: AccountId) -> Vec<(Option<String>, Decimal, Decimal)> {
    s.products
        .iter()
        .filter(|l| l.account == account)
        .map(|l| (l.account_type.clone(), l.delta, l.after))
        .collect()
}

#[test]
fn nisa_cannot_be_sold_until_the_user_allows_it() {
    let mut f = fixture(dec!(300000), dec!(500000));
    let err = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(600000),
    );
    assert!(matches!(err, Err(Error::NotEnoughToSell)));
    sellable(&mut f.portfolio, "nisa", true);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(600000),
    )
    .unwrap();
    assert_eq!(
        lines(&s, f.main),
        vec![
            (Some("general".to_owned()), dec!(-300000), dec!(0)),
            (Some("nisa".to_owned()), dec!(-300000), dec!(200000)),
        ]
    );
    assert!(s.products.iter().all(|l| l.by_type));
}

#[test]
fn non_nisa_is_sold_first_and_nisa_only_for_the_remainder() {
    let mut f = fixture(dec!(300000), dec!(500000));
    sellable(&mut f.portfolio, "nisa", true);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(200000),
    )
    .unwrap();
    assert_eq!(
        lines(&s, f.main),
        vec![(Some("general".to_owned()), dec!(-200000), dec!(100000))]
    );
    // Turning a default-sellable type off removes it from the sale.
    sellable(&mut f.portfolio, "general", false);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(200000),
    )
    .unwrap();
    assert_eq!(
        lines(&s, f.main),
        vec![(Some("nisa".to_owned()), dec!(-200000), dec!(300000))]
    );
}

#[test]
fn both_allocation_methods_split_alike() {
    let mut f = fixture(dec!(300000), dec!(500000));
    sellable(&mut f.portfolio, "nisa", true);
    for method in [Method::FollowPortfolio, Method::ByProductWeights] {
        let s = sell(&f.portfolio, Some(f.main), method, dec!(400000)).unwrap();
        let l = lines(&s, f.main);
        assert_eq!(l.len(), 2, "{method:?}");
        assert_eq!(l.iter().map(|x| x.1).sum::<Decimal>(), dec!(-400000));
        assert_eq!(l[0], (Some("general".to_owned()), dec!(-300000), dec!(0)));
        assert_eq!(l[1], (Some("nisa".to_owned()), dec!(-100000), dec!(400000)));
    }
}

#[test]
fn the_parts_respect_the_unit_and_the_holdings() {
    // Neither group holds a whole unit more than once: 600 + 600 allows selling 1,000.
    let mut f = fixture(dec!(600), dec!(600));
    sellable(&mut f.portfolio, "nisa", true);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(1000),
    )
    .unwrap();
    let l = lines(&s, f.main);
    assert_eq!(l.iter().map(|x| x.1).sum::<Decimal>(), dec!(-1000));
    assert_eq!(l[0], (Some("general".to_owned()), dec!(-600), dec!(0)));
    assert_eq!(l[1], (Some("nisa".to_owned()), dec!(-400), dec!(200)));

    // With room in the first group the parts are whole units and nothing exceeds a group.
    let mut f = fixture(dec!(2500), dec!(5000));
    sellable(&mut f.portfolio, "nisa", true);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(4000),
    )
    .unwrap();
    let l = lines(&s, f.main);
    assert_eq!(l[0], (Some("general".to_owned()), dec!(-2000), dec!(500)));
    assert_eq!(l[1], (Some("nisa".to_owned()), dec!(-2000), dec!(3000)));
    let _ = f.product;
}

#[test]
fn a_custom_type_sells_after_the_builtin_ones_and_before_nisa() {
    let mut f = fixture(dec!(100000), dec!(500000));
    f.portfolio
        .apply(Edit::AddAccountType {
            name: "Mine".to_owned(),
            sellable: true,
        })
        .unwrap();
    let id = f.portfolio.custom_types[0].id.clone();
    hold(&mut f.portfolio, f.main, f.product, Some(&id), dec!(200000));
    sellable(&mut f.portfolio, "nisa", true);
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(400000),
    )
    .unwrap();
    assert_eq!(
        lines(&s, f.main),
        vec![
            (Some("general".to_owned()), dec!(-100000), dec!(0)),
            (Some(id.clone()), dec!(-200000), dec!(0)),
            (Some("nisa".to_owned()), dec!(-100000), dec!(400000)),
        ]
    );
    // SetTypeSellable switches a custom type too.
    sellable(&mut f.portfolio, &id, false);
    assert!(!f.portfolio.type_sellable(Some(&id)));
}

#[test]
fn the_all_tab_splits_each_account_by_its_own_types() {
    let mut f = fixture(dec!(300000), dec!(300000));
    sellable(&mut f.portfolio, "nisa", true);
    let second = add_account(&mut f.portfolio, "Second", dec!(1000));
    hold(
        &mut f.portfolio,
        second,
        f.product,
        Some("specific"),
        dec!(100000),
    );
    hold(
        &mut f.portfolio,
        second,
        f.product,
        Some("nisa_growth"),
        dec!(300000),
    );
    sellable(&mut f.portfolio, "nisa_growth", true);
    let s = sell(&f.portfolio, None, Method::FollowPortfolio, dec!(1000000)).unwrap();
    let mut total = Decimal::ZERO;
    for account in [f.main, second] {
        let l = lines(&s, account);
        let nisa_at = l
            .iter()
            .position(|x| x.0.as_deref().is_some_and(|t| t.starts_with("nisa")));
        // Within an account the NISA part is last, and only used when the other is exhausted.
        if let Some(i) = nisa_at {
            assert_eq!(i, l.len() - 1);
            assert_eq!(l[0].2, dec!(0));
        }
        total += l.iter().map(|x| x.1).sum::<Decimal>();
    }
    assert_eq!(total, dec!(-1000000));
    assert_eq!(s.applied, dec!(1000000));
}

#[test]
fn a_strategy_account_sells_non_nisa_first_too() {
    let mut f = fixture(dec!(100000), dec!(700000));
    sellable(&mut f.portfolio, "nisa", true);
    let other = {
        let category = f.portfolio.categories[0].id;
        f.portfolio
            .apply(Edit::AddProduct {
                name: "Other".to_owned(),
                category,
                currency: "JPY".to_owned(),
            })
            .unwrap();
        f.portfolio.products.last().unwrap().id
    };
    hold(
        &mut f.portfolio,
        f.main,
        other,
        Some("general"),
        dec!(100000),
    );
    f.portfolio
        .apply(Edit::SetStrategy {
            id: f.main,
            strategy: Strategy::Products {
                items: [f.product, other]
                    .iter()
                    .map(|p| StrategyItem {
                        product: *p,
                        ratio: Some(dec!(1)),
                    })
                    .collect(),
            },
        })
        .unwrap();
    let s = sell(
        &f.portfolio,
        Some(f.main),
        Method::FollowPortfolio,
        dec!(300000),
    )
    .unwrap();
    let l: Vec<_> = s
        .products
        .iter()
        .filter(|x| x.product == f.product)
        .collect();
    assert_eq!(l[0].account_type.as_deref(), Some("general"));
    assert_eq!(l[0].delta, dec!(-100000));
    assert_eq!(l[1].account_type.as_deref(), Some("nisa"));
    assert_eq!(l.iter().map(|x| x.delta).sum::<Decimal>(), dec!(-300000));
}

#[test]
fn the_balance_lists_the_value_per_type_in_sale_order() {
    let f = fixture(dec!(300000), dec!(500000));
    let b = account_balance(&f.portfolio, f.main).unwrap();
    let row = &b.categories[0].products[0];
    assert_eq!(row.current, dec!(800000));
    let kinds: Vec<_> = row
        .types
        .iter()
        .map(|t| (t.account_type.clone(), t.current))
        .collect();
    assert_eq!(
        kinds,
        vec![
            (Some("general".to_owned()), dec!(300000)),
            (Some("nisa".to_owned()), dec!(500000)),
        ]
    );
}

#[test]
fn a_purchase_stays_one_line_without_a_type() {
    let f = fixture(dec!(300000), dec!(500000));
    let s = suggest(
        &f.portfolio,
        &Request {
            account: Some(f.main),
            mode: Mode::FixedAdd {
                amount: dec!(10000),
            },
            held_only: false,
            method: Method::FollowPortfolio,
        },
    )
    .unwrap();
    assert_eq!(s.products.len(), 1);
    assert!(!s.products[0].by_type && s.products[0].account_type.is_none());
    assert_eq!(s.products[0].after, dec!(810000));
}
