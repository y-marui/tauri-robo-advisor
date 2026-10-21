// Every name and amount in these tests is made up.

use portfolio::{AccountId, Edit, Portfolio, ProductId, Strategy, StrategyItem};
use rust_decimal::{Decimal, dec};

use crate::*;

struct Fixture {
    portfolio: Portfolio,
    funds: Vec<ProductId>,
    other: ProductId,
    main: AccountId,
    a: AccountId,
    b: AccountId,
}

fn account(p: &mut Portfolio, name: &str, sellable: bool) -> AccountId {
    p.apply(Edit::AddAccount {
        name: name.to_owned(),
    })
    .unwrap();
    let id = p.accounts.last().unwrap().id;
    p.apply(Edit::UpdateAccount {
        id,
        name: None,
        sellable: Some(sellable),
        rounding_unit: Some(dec!(1000)),
    })
    .unwrap();
    id
}

fn hold(p: &mut Portfolio, account: AccountId, product: ProductId, value: Decimal) {
    p.apply(Edit::SetHolding {
        account_type: None,
        account,
        product,
        value,
        quantity: None,
    })
    .unwrap();
}

fn items(list: &[(ProductId, Option<Decimal>)]) -> Strategy {
    Strategy::Products {
        items: list
            .iter()
            .map(|(product, ratio)| StrategyItem {
                product: *product,
                ratio: *ratio,
            })
            .collect(),
    }
}

fn set(p: &mut Portfolio, id: AccountId, strategy: Strategy) {
    p.apply(Edit::SetStrategy { id, strategy }).unwrap();
}

/// Five funds and one more product, in one category (the category targets are not used by the
/// strategy accounts). Three accounts: main (sellable), A (buy only), B (sellable).
fn fixture() -> Fixture {
    let mut p = Portfolio::default();
    p.apply(Edit::AddCategory {
        name: "All".to_owned(),
        target_weight: dec!(1),
    })
    .unwrap();
    let category = p.categories[0].id;
    let mut ids = Vec::new();
    for name in ["F1", "F2", "F3", "F4", "F5", "Other"] {
        p.apply(Edit::AddProduct {
            name: name.to_owned(),
            category,
            currency: "JPY".to_owned(),
        })
        .unwrap();
        ids.push(p.products.last().unwrap().id);
    }
    let other = ids.pop().unwrap();
    let main = account(&mut p, "Main", true);
    let a = account(&mut p, "A", false);
    let b = account(&mut p, "B", true);
    Fixture {
        portfolio: p,
        funds: ids,
        other,
        main,
        a,
        b,
    }
}

fn run(p: &Portfolio, account: AccountId, mode: Mode) -> Result<Suggestion, Error> {
    suggest(
        p,
        &Request {
            account: Some(account),
            mode,
            held_only: true,
            method: Default::default(),
        },
    )
}

fn delta(s: &Suggestion, product: ProductId) -> Decimal {
    s.products
        .iter()
        .filter(|l| l.product == product)
        .map(|l| l.delta)
        .sum()
}

#[test]
fn main_account_follows_the_ratios_of_its_savings_plan() {
    let mut f = fixture();
    let [f1, f2, f3, f4, ..] = f.funds[..] else {
        unreachable!()
    };
    set(
        &mut f.portfolio,
        f.main,
        items(&[
            (f1, Some(dec!(3))),
            (f2, Some(dec!(2))),
            (f3, Some(dec!(1))),
        ]),
    );
    hold(&mut f.portfolio, f.main, f1, dec!(300000));
    hold(&mut f.portfolio, f.main, f2, dec!(100000));
    hold(&mut f.portfolio, f.main, f3, dec!(100000));
    hold(&mut f.portfolio, f.main, f4, dec!(50000));

    // F2 is furthest below its 1/3 share, so it takes the whole amount; F4 is outside the plan.
    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(60000),
        },
    )
    .unwrap();
    assert_eq!(delta(&s, f2), dec!(60000));
    assert_eq!(s.products.len(), 1);
    assert!(s.categories.is_empty());

    // Enough money to reach every ratio: 1,100,000 in all, so F1 gets 250,000 more (half), F4 none.
    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(600000),
        },
    )
    .unwrap();
    assert_eq!(delta(&s, f1), dec!(250000));
    assert_eq!(delta(&s, f4), Decimal::ZERO);
    assert_eq!(
        s.products.iter().map(|l| l.delta).sum::<Decimal>(),
        dec!(600000)
    );

    // Without selling, the plan is reached by raising F2 (the level is set by F1 and F3).
    let s = run(&f.portfolio, f.main, Mode::BuyOnlyToTarget).unwrap();
    assert_eq!(s.applied, dec!(100000));
    assert_eq!(delta(&s, f2), dec!(100000));
}

#[test]
fn sub_account_a_spreads_over_funds_one_to_five_equally() {
    let mut f = fixture();
    let list: Vec<_> = f.funds.iter().map(|p| (*p, None)).collect();
    set(&mut f.portfolio, f.a, items(&list));
    for fund in &f.funds[..4] {
        hold(&mut f.portfolio, f.a, *fund, dec!(100000));
    }
    hold(&mut f.portfolio, f.a, f.other, dec!(70000));

    let balance = account_balance(&f.portfolio, f.a).unwrap();
    let strategy = balance.strategy.unwrap();
    assert!(balance.categories.is_empty());
    assert_eq!(strategy.total, dec!(400000));
    for line in &strategy.products {
        assert_eq!(line.target, Some(dec!(80000)));
    }
    assert_eq!(strategy.products[4].shortfall, Some(dec!(80000)));
    assert_eq!(strategy.products[0].shortfall, Some(dec!(-20000)));

    // The fifth fund is empty, so it takes the money even though the account holds none of it.
    let s = run(
        &f.portfolio,
        f.a,
        Mode::FixedAdd {
            amount: dec!(50000),
        },
    )
    .unwrap();
    assert_eq!(delta(&s, f.funds[4]), dec!(50000));
    // The account cannot sell.
    let s = run(&f.portfolio, f.a, Mode::FullRebalance).unwrap();
    assert!(s.products.is_empty());
    assert!(
        s.notes
            .iter()
            .any(|n| matches!(n, Note::ProductNotFilled { .. }))
    );
    assert_eq!(
        run(&f.portfolio, f.a, Mode::FixedSell { amount: dec!(1000) }).unwrap_err(),
        Error::NotSellable
    );
}

#[test]
fn sub_account_b_holds_fund_one_and_two_at_three_to_two() {
    let mut f = fixture();
    let [f1, f2, f3, ..] = f.funds[..] else {
        unreachable!()
    };
    set(
        &mut f.portfolio,
        f.b,
        items(&[(f1, Some(dec!(3))), (f2, Some(dec!(2)))]),
    );
    hold(&mut f.portfolio, f.b, f1, dec!(100000));
    hold(&mut f.portfolio, f.b, f2, dec!(100000));
    hold(&mut f.portfolio, f.b, f3, dec!(90000));

    // 200,000 in the plan: 120,000 and 80,000 are wanted, so 20,000 moves from F2 to F1.
    let s = run(&f.portfolio, f.b, Mode::FullRebalance).unwrap();
    assert_eq!(delta(&s, f1), dec!(20000));
    assert_eq!(delta(&s, f2), dec!(-20000));
    assert_eq!(delta(&s, f3), Decimal::ZERO);
    assert_eq!(s.applied, dec!(20000));

    // A sale starts with the fund above its share and never touches the others.
    let s = run(
        &f.portfolio,
        f.b,
        Mode::FixedSell {
            amount: dec!(10000),
        },
    )
    .unwrap();
    assert_eq!(delta(&s, f2), dec!(-10000));
    assert_eq!(s.products.len(), 1);
}

#[test]
fn blank_ratios_mean_equal_shares_and_mixing_is_refused() {
    let mut f = fixture();
    let [f1, f2, ..] = f.funds[..] else {
        unreachable!()
    };
    set(&mut f.portfolio, f.b, items(&[(f1, None), (f2, None)]));
    hold(&mut f.portfolio, f.b, f1, dec!(30000));
    hold(&mut f.portfolio, f.b, f2, dec!(10000));
    let s = run(&f.portfolio, f.b, Mode::FullRebalance).unwrap();
    assert_eq!(delta(&s, f1), dec!(-10000));
    assert_eq!(delta(&s, f2), dec!(10000));
    assert!(
        f.portfolio
            .apply(Edit::SetStrategy {
                id: f.b,
                strategy: items(&[(f1, None), (f2, Some(dec!(1)))]),
            })
            .is_err()
    );
}

#[test]
fn excluded_products_are_never_traded_but_count_toward_the_total() {
    let mut f = fixture();
    let [f1, f2, ..] = f.funds[..] else {
        unreachable!()
    };
    set(
        &mut f.portfolio,
        f.b,
        items(&[(f1, Some(dec!(1))), (f2, Some(dec!(1)))]),
    );
    f.portfolio
        .apply(Edit::SetExcluded {
            id: f.b,
            products: vec![f2],
            all: false,
        })
        .unwrap();
    hold(&mut f.portfolio, f.b, f1, dec!(0));
    hold(&mut f.portfolio, f.b, f2, dec!(100000));

    // F1 is short by 50,000; F2 is over but cannot be sold, so nothing can be paid for by a sale.
    let s = run(
        &f.portfolio,
        f.b,
        Mode::FixedAdd {
            amount: dec!(80000),
        },
    )
    .unwrap();
    assert_eq!(delta(&s, f1), dec!(80000));
    assert_eq!(delta(&s, f2), Decimal::ZERO);
    let s = run(&f.portfolio, f.b, Mode::FullRebalance).unwrap();
    assert!(s.products.is_empty());
    assert!(
        s.notes
            .iter()
            .any(|n| matches!(n, Note::ProductNotFilled { .. }))
    );
}

#[test]
fn the_all_tab_leaves_a_strategy_account_alone() {
    let mut f = fixture();
    let [f1, f2, ..] = f.funds[..] else {
        unreachable!()
    };
    set(&mut f.portfolio, f.b, items(&[(f1, None)]));
    hold(&mut f.portfolio, f.main, f2, dec!(100000));
    hold(&mut f.portfolio, f.b, f1, dec!(100000));
    let s = suggest(
        &f.portfolio,
        &Request {
            account: None,
            mode: Mode::FixedAdd {
                amount: dec!(10000),
            },
            held_only: true,
            method: Default::default(),
        },
    )
    .unwrap();
    assert!(s.products.iter().all(|l| l.account != f.b));
    assert!(!s.products.is_empty());
}
