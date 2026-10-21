// Every name and amount in these tests is made up.
//
// An account's tab levels by that account's own holdings and weights; the All tab by the holdings
// of every account and its own weights.

use portfolio::{AccountId, Edit, Portfolio, ProductId, SuggestionMode};
use rust_decimal::{Decimal, dec};

use crate::*;

struct Book {
    portfolio: Portfolio,
    a: ProductId,
    b: ProductId,
    x: AccountId,
    y: AccountId,
}

fn apply(p: &mut Portfolio, edit: Edit) {
    p.apply(edit).unwrap();
}

fn hold(p: &mut Portfolio, account: AccountId, product: ProductId, value: Decimal) {
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

fn weigh(p: &mut Portfolio, product: ProductId, account: Option<AccountId>, weight: Decimal) {
    for mode in SuggestionMode::ALL {
        apply(
            p,
            Edit::SetProductShare {
                mode,
                product,
                account,
                weight,
            },
        );
    }
}

/// Two categories of 50% with one product each (A, B). Account X holds A 3,323 and B 5,042;
/// account Y holds a lot of A. Both accounts buy in units of 1 yen and may sell.
fn book(separate_categories: bool) -> Book {
    let mut p = Portfolio::default();
    let c1 = add_category(
        &mut p,
        "C1",
        if separate_categories {
            dec!(0.5)
        } else {
            dec!(1)
        },
    );
    let c2 = if separate_categories {
        add_category(&mut p, "C2", dec!(0.5))
    } else {
        c1
    };
    let a = add_product(&mut p, "A", c1);
    let b = add_product(&mut p, "B", c2);
    apply(&mut p, Edit::AddAccount { name: "X".into() });
    let x = p.accounts.last().unwrap().id;
    apply(&mut p, Edit::AddAccount { name: "Y".into() });
    let y = p.accounts.last().unwrap().id;
    hold(&mut p, x, a, dec!(3323));
    hold(&mut p, x, b, dec!(5042));
    hold(&mut p, y, a, dec!(100000));
    Book {
        portfolio: p,
        a,
        b,
        x,
        y,
    }
}

fn add_category(p: &mut Portfolio, name: &str, weight: Decimal) -> portfolio::CategoryId {
    apply(
        p,
        Edit::AddCategory {
            name: name.into(),
            target_weight: weight,
        },
    );
    p.categories.last().unwrap().id
}

fn add_product(p: &mut Portfolio, name: &str, category: portfolio::CategoryId) -> ProductId {
    apply(
        p,
        Edit::AddProduct {
            name: name.into(),
            category,
            currency: "JPY".into(),
        },
    );
    p.products.last().unwrap().id
}

fn run(p: &Portfolio, account: Option<AccountId>, mode: Mode, method: Method) -> Suggestion {
    suggest(
        p,
        &Request {
            account,
            mode,
            held_only: false,
            method,
        },
    )
    .unwrap()
}

fn delta(s: &Suggestion, product: ProductId) -> Decimal {
    s.products
        .iter()
        .filter(|l| l.product == product)
        .map(|l| l.delta)
        .sum()
}

fn after(s: &Suggestion, account: AccountId, product: ProductId) -> Decimal {
    s.products
        .iter()
        .find(|l| l.account == account && l.product == product)
        .map_or(Decimal::ZERO, |l| l.after)
}

const WEIGHTS: Method = Method::ByProductWeights;

#[test]
fn a_fixed_add_on_a_tab_levels_by_the_accounts_own_holdings() {
    let mut f = book(false);
    weigh(&mut f.portfolio, f.a, Some(f.x), dec!(2));
    weigh(&mut f.portfolio, f.b, Some(f.x), dec!(3));

    let s = run(
        &f.portfolio,
        Some(f.x),
        Mode::FixedAdd {
            amount: dec!(50000),
        },
        WEIGHTS,
    );

    // 8,365 + 50,000 = 58,365 split 2:3 (the other account's 100,000 of A is not counted).
    let (a, b) = (after(&s, f.x, f.a), after(&s, f.x, f.b));
    assert_eq!(a + b, dec!(58365));
    assert!((a * dec!(3) - b * dec!(2)).abs() <= dec!(3), "{a} {b}");
    assert_eq!(a, dec!(23346));
}

#[test]
fn the_all_tab_still_levels_by_the_holdings_of_every_account() {
    let mut f = book(false);
    weigh(&mut f.portfolio, f.a, None, dec!(2));
    weigh(&mut f.portfolio, f.b, None, dec!(3));

    let s = run(
        &f.portfolio,
        None,
        Mode::FixedAdd {
            amount: dec!(50000),
        },
        WEIGHTS,
    );

    // A holds 103,323 across the accounts: B (5,042) is far below its share and takes it all.
    assert_eq!(delta(&s, f.a), dec!(0));
    assert_eq!(delta(&s, f.b), dec!(50000));
}

#[test]
fn a_tab_weight_is_independent_of_all_and_of_other_tabs() {
    let mut f = book(false);
    weigh(&mut f.portfolio, f.a, Some(f.x), dec!(2));
    weigh(&mut f.portfolio, f.b, Some(f.x), dec!(3));
    let mode = Mode::FixedAdd {
        amount: dec!(50000),
    };
    let before = run(&f.portfolio, Some(f.x), mode.clone(), WEIGHTS);

    // Weights of the All tab and of the other account's tab do not touch this tab.
    weigh(&mut f.portfolio, f.b, None, dec!(100));
    weigh(&mut f.portfolio, f.a, Some(f.y), dec!(50));
    assert_eq!(run(&f.portfolio, Some(f.x), mode.clone(), WEIGHTS), before);

    // Its own weight does.
    weigh(&mut f.portfolio, f.b, Some(f.x), dec!(1));
    assert_ne!(run(&f.portfolio, Some(f.x), mode, WEIGHTS), before);
}

#[test]
fn method_one_levels_the_categories_by_the_accounts_own_holdings() {
    let mut f = book(true);
    // X holds A 3,323 and B 5,042 (categories of 50% each): by itself it is short of A. The
    // other account's 100,000 of A would make A the over-weight category for the whole portfolio.
    let s = run(
        &f.portfolio,
        Some(f.x),
        Mode::FixedAdd { amount: dec!(1000) },
        Method::FollowPortfolio,
    );
    assert_eq!(delta(&s, f.a), dec!(1000));
    assert_eq!(delta(&s, f.b), dec!(0));
    assert_eq!(s.total_after, dec!(9365));

    // The All tab sees the whole portfolio: B is the short one there.
    let s = run(
        &f.portfolio,
        None,
        Mode::FixedAdd { amount: dec!(1000) },
        Method::FollowPortfolio,
    );
    assert_eq!(delta(&s, f.a), dec!(0));
    assert_eq!(delta(&s, f.b), dec!(1000));

    // Inside a category the products are leveled by the account's holdings too.
    let c1 = f.portfolio.categories[0].id;
    let a2 = add_product(&mut f.portfolio, "A2", c1);
    hold(&mut f.portfolio, f.y, a2, dec!(0));
    let s = run(
        &f.portfolio,
        Some(f.x),
        Mode::FixedAdd { amount: dec!(1000) },
        Method::FollowPortfolio,
    );
    // A (3,323 here, 103,323 in all) is not treated as full: the account's A2 (0) is raised
    // first, then both are raised together.
    assert_eq!(delta(&s, a2), dec!(1000));
    assert_eq!(delta(&s, f.a), dec!(0));
}

#[test]
fn a_fixed_sell_on_a_tab_levels_by_the_accounts_own_holdings() {
    let mut f = book(false);
    hold(&mut f.portfolio, f.x, f.a, dec!(10000));
    hold(&mut f.portfolio, f.x, f.b, dec!(10000));
    weigh(&mut f.portfolio, f.a, Some(f.x), dec!(2));
    weigh(&mut f.portfolio, f.b, Some(f.x), dec!(3));

    let s = run(
        &f.portfolio,
        Some(f.x),
        Mode::FixedSell { amount: dec!(5000) },
        WEIGHTS,
    );

    // 20,000 - 5,000 = 15,000 split 2:3 is A 6,000 and B 9,000 (not all from A, which the other
    // account's 100,000 would make look far over-weight).
    assert_eq!(delta(&s, f.a), dec!(-4000));
    assert_eq!(delta(&s, f.b), dec!(-1000));
}

#[test]
fn a_fixed_sell_by_categories_on_a_tab_levels_by_the_accounts_own_holdings() {
    let mut f = book(true);
    hold(&mut f.portfolio, f.x, f.a, dec!(10000));
    hold(&mut f.portfolio, f.x, f.b, dec!(10000));

    let s = run(
        &f.portfolio,
        Some(f.x),
        Mode::FixedSell { amount: dec!(2000) },
        Method::FollowPortfolio,
    );

    assert_eq!(delta(&s, f.a), dec!(-1000));
    assert_eq!(delta(&s, f.b), dec!(-1000));
}

#[test]
fn buy_only_and_full_rebalance_by_weights_use_the_accounts_own_holdings() {
    let mut f = book(false);
    hold(&mut f.portfolio, f.x, f.a, dec!(4000));
    hold(&mut f.portfolio, f.x, f.b, dec!(3000));
    weigh(&mut f.portfolio, f.a, Some(f.x), dec!(2));
    weigh(&mut f.portfolio, f.b, Some(f.x), dec!(3));

    // A (4,000 at 40%) sets the level 10,000: B needs 3,000 more.
    let s = run(&f.portfolio, Some(f.x), Mode::BuyOnlyToTarget, WEIGHTS);
    assert_eq!(s.required_total, Some(dec!(3000)));
    assert_eq!(delta(&s, f.a), dec!(0));
    assert_eq!(delta(&s, f.b), dec!(3000));

    hold(&mut f.portfolio, f.x, f.a, dec!(8000));
    hold(&mut f.portfolio, f.x, f.b, dec!(2000));
    let s = run(&f.portfolio, Some(f.x), Mode::FullRebalance, WEIGHTS);
    // 10,000 split 2:3 is A 4,000 and B 6,000.
    assert_eq!(delta(&s, f.a), dec!(-4000));
    assert_eq!(delta(&s, f.b), dec!(4000));
}
