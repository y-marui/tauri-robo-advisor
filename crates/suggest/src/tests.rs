// Every name and amount in these tests is made up.

use portfolio::{AccountId, CategoryId, Edit, Portfolio, ProductId, ProductTargeting};
use rust_decimal::{Decimal, dec};

use crate::engine::{MAX_AMOUNT, MAX_WEIGHT};
use crate::*;

struct Fixture {
    portfolio: Portfolio,
    stocks: CategoryId,
    bonds: CategoryId,
    s1: ProductId,
    b1: ProductId,
    main: AccountId,
    sub: AccountId,
}

fn apply(p: &mut Portfolio, edit: Edit) {
    p.apply(edit).unwrap();
}

fn category(p: &mut Portfolio, name: &str, weight: Decimal) -> CategoryId {
    apply(
        p,
        Edit::AddCategory {
            name: name.to_owned(),
            target_weight: weight,
        },
    );
    p.categories.last().unwrap().id
}

fn product(p: &mut Portfolio, name: &str, category: CategoryId, currency: &str) -> ProductId {
    apply(
        p,
        Edit::AddProduct {
            name: name.to_owned(),
            category,
            currency: currency.to_owned(),
        },
    );
    p.products.last().unwrap().id
}

fn account(p: &mut Portfolio, name: &str) -> AccountId {
    apply(
        p,
        Edit::AddAccount {
            name: name.to_owned(),
        },
    );
    let id = p.accounts.last().unwrap().id;
    // These tests are written for accounts that buy in units of 1,000 yen (the default of a
    // new account is 1 yen).
    apply(
        p,
        Edit::UpdateAccount {
            id,
            name: None,
            sellable: None,
            rounding_unit: Some(dec!(1000)),
        },
    );
    id
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

/// Stocks 60% and bonds 40%. The main account holds 700,000 of stocks and
/// 100,000 of bonds; the sub account holds another 100,000 of bonds.
/// Total 900,000: stocks are over target, bonds under.
fn fixture() -> Fixture {
    let mut p = Portfolio::default();
    let stocks = category(&mut p, "Stocks", dec!(0.6));
    let bonds = category(&mut p, "Bonds", dec!(0.4));
    let s1 = product(&mut p, "Stock Fund", stocks, "JPY");
    let b1 = product(&mut p, "Bond Fund", bonds, "JPY");
    let main = account(&mut p, "Main");
    let sub = account(&mut p, "Sub");
    apply(
        &mut p,
        Edit::UpdateAccount {
            id: sub,
            name: None,
            sellable: Some(false),
            rounding_unit: None,
        },
    );
    hold(&mut p, main, s1, dec!(700000));
    hold(&mut p, main, b1, dec!(100000));
    hold(&mut p, sub, b1, dec!(100000));
    Fixture {
        portfolio: p,
        stocks,
        bonds,
        s1,
        b1,
        main,
        sub,
    }
}

fn run(p: &Portfolio, account: AccountId, mode: Mode) -> Result<Suggestion, Error> {
    suggest(
        p,
        &Request {
            account,
            mode,
            held_only: false,
        },
    )
}

fn deltas(s: &Suggestion) -> Vec<(ProductId, Decimal)> {
    s.products.iter().map(|l| (l.product, l.delta)).collect()
}

fn category_line(s: &Suggestion, id: CategoryId) -> &CategoryLine {
    s.categories.iter().find(|c| c.category == id).unwrap()
}

#[test]
fn limits_are_what_the_documentation_says() {
    assert_eq!(MAX_AMOUNT, dec!(1000000000000000));
    assert_eq!(MAX_WEIGHT, dec!(1000000000));
}

// ---- fixed add ------------------------------------------------------------

#[test]
fn a_small_amount_goes_entirely_to_the_most_underweight_category() {
    let f = fixture();

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();

    assert_eq!(deltas(&s), vec![(f.b1, dec!(100000))]);
    assert_eq!(s.applied, dec!(100000));
    assert_eq!(category_line(&s, f.bonds).delta, dec!(100000));
    assert_eq!(category_line(&s, f.stocks).delta, dec!(0));
    assert_eq!(s.total_after, dec!(1000000));
}

#[test]
fn a_large_amount_reaches_the_target_split_in_both_categories() {
    let f = fixture();

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(1000000),
        },
    )
    .unwrap();

    // Total after = 1,900,000: stocks 1,140,000 (+440,000), bonds 760,000 (+560,000).
    assert_eq!(deltas(&s), vec![(f.s1, dec!(440000)), (f.b1, dec!(560000))]);
    assert_eq!(category_line(&s, f.stocks).after, dec!(1140000));
    assert_eq!(category_line(&s, f.stocks).target_after, dec!(1140000));
}

#[test]
fn nothing_is_ever_sold_by_a_fixed_add() {
    let f = fixture();

    for amount in (1..=40).map(|n| Decimal::from(n * 37_000)) {
        let s = run(&f.portfolio, f.main, Mode::FixedAdd { amount }).unwrap();

        assert!(s.products.iter().all(|l| l.delta > Decimal::ZERO));
        assert!(s.products.iter().all(|l| (l.delta % dec!(1000)).is_zero()));
        let total: Decimal = s.products.iter().map(|l| l.delta).sum();
        assert_eq!(total, s.applied);
        assert_eq!(s.applied, (amount / dec!(1000)).floor() * dec!(1000));
    }
}

#[test]
fn an_amount_is_rounded_down_to_the_unit_and_the_change_is_reported() {
    let f = fixture();

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100500),
        },
    )
    .unwrap();

    assert_eq!(s.applied, dec!(100000));
    assert_eq!(
        s.notes,
        vec![Note::AmountRounded {
            requested: dec!(100500),
            applied: dec!(100000)
        }]
    );
}

#[test]
fn an_amount_below_the_unit_is_an_error() {
    let f = fixture();

    assert_eq!(
        run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(999) }),
        Err(Error::AmountBelowUnit)
    );
}

#[test]
fn only_products_the_account_can_buy_receive_money() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::SetTradable {
            id: f.sub,
            products: Some(vec![f.s1]),
        },
    );

    let s = run(
        &f.portfolio,
        f.sub,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();

    // Bonds need money more, but this account can only buy the stock fund.
    assert_eq!(deltas(&s), vec![(f.s1, dec!(100000))]);
}

#[test]
fn an_account_that_can_buy_nothing_gets_an_error() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::SetTradable {
            id: f.sub,
            products: Some(vec![]),
        },
    );

    assert_eq!(
        run(
            &f.portfolio,
            f.sub,
            Mode::FixedAdd {
                amount: dec!(100000)
            }
        ),
        Err(Error::NothingToBuy)
    );
}

#[test]
fn a_rounding_unit_of_one_hundred_gives_finer_lines() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateAccount {
            id: f.main,
            name: None,
            sellable: None,
            rounding_unit: Some(dec!(100)),
        },
    );

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(333300),
        },
    )
    .unwrap();

    let total: Decimal = s.products.iter().map(|l| l.delta).sum();
    assert_eq!(total, dec!(333300));
    assert!(s.products.iter().all(|l| (l.delta % dec!(100)).is_zero()));
}

// ---- products inside a category -------------------------------------------

#[test]
fn weighted_products_fill_toward_their_relative_weights() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "All", dec!(1));
    let a = product(&mut p, "A", c, "JPY");
    let b = product(&mut p, "B", c, "JPY");
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, a, dec!(100000));
    hold(&mut p, acc, b, dec!(100000));
    apply(
        &mut p,
        Edit::UpdateCategory {
            id: c,
            name: None,
            target_weight: None,
            targeting: Some(ProductTargeting::Weighted),
        },
    );
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: acc,
            product: a,
            weight: Some(dec!(3)),
        },
    );
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: acc,
            product: b,
            weight: Some(dec!(1)),
        },
    );

    let small = run(
        &p,
        acc,
        Mode::FixedAdd {
            amount: dec!(200000),
        },
    )
    .unwrap();
    let large = run(
        &p,
        acc,
        Mode::FixedAdd {
            amount: dec!(400000),
        },
    )
    .unwrap();

    // 3:1 of 400,000 is 300,000 / 100,000, so only A needs money.
    assert_eq!(deltas(&small), vec![(a, dec!(200000))]);
    // 3:1 of 600,000 is 450,000 / 150,000.
    assert_eq!(deltas(&large), vec![(a, dec!(350000)), (b, dec!(50000))]);
}

#[test]
fn equal_and_keep_current_products_split_new_money_sensibly() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "All", dec!(1));
    let a = product(&mut p, "A", c, "JPY");
    let b = product(&mut p, "B", c, "JPY");
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, a, dec!(300000));
    hold(&mut p, acc, b, dec!(100000));

    let equal = |targeting| {
        let mut q = p.clone();
        apply(
            &mut q,
            Edit::UpdateCategory {
                id: c,
                name: None,
                target_weight: None,
                targeting: Some(targeting),
            },
        );
        run(
            &q,
            acc,
            Mode::FixedAdd {
                amount: dec!(200000),
            },
        )
        .unwrap()
    };

    // Equal: the smaller product is filled first up to 300,000, then evenly.
    assert_eq!(
        deltas(&equal(ProductTargeting::Equal)),
        vec![(b, dec!(200000))]
    );
    // Keep current: new money follows the current 3:1 proportions.
    assert_eq!(
        deltas(&equal(ProductTargeting::KeepCurrent)),
        vec![(a, dec!(150000)), (b, dec!(50000))]
    );
    // Free: no product ratios were set, so equal weights are used and what is held counts: the
    // smaller product is raised first, not an even split of the purchase.
    assert_eq!(
        deltas(&equal(ProductTargeting::Free)),
        vec![(b, dec!(200000))]
    );
}

#[test]
fn excluded_products_are_never_traded_but_still_count_for_the_category() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "All", dec!(1));
    let a = product(&mut p, "A", c, "JPY");
    let b = product(&mut p, "B", c, "JPY");
    let d = product(&mut p, "D", c, "JPY");
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, a, dec!(300000));
    hold(&mut p, acc, b, dec!(100000));
    hold(&mut p, acc, d, dec!(100000));
    apply(
        &mut p,
        Edit::SetExcluded {
            id: acc,
            products: vec![b],
        },
    );

    // B would be the first to be raised; it is left out and D takes the amount instead.
    let buy = run(
        &p,
        acc,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();
    assert_eq!(deltas(&buy), vec![(d, dec!(100000))]);
    // The held value of B is part of the category.
    assert_eq!(category_line(&buy, c).current, dec!(500000));

    // Selling: B is the smallest but A is above its share and gives first; B is never sold.
    let sell = run(
        &p,
        acc,
        Mode::FixedSell {
            amount: dec!(400000),
        },
    )
    .unwrap();
    assert!(sell.products.iter().all(|l| l.product != b));
    assert_eq!(
        sell.products.iter().map(|l| l.delta).sum::<Decimal>(),
        dec!(-400000)
    );

    // Nothing left to trade in the category.
    apply(
        &mut p,
        Edit::SetExcluded {
            id: acc,
            products: vec![a, b, d],
        },
    );
    assert!(
        run(
            &p,
            acc,
            Mode::FixedAdd {
                amount: dec!(100000)
            }
        )
        .is_err()
    );
}

#[test]
fn a_category_goes_one_way_and_products_are_brought_toward_equal() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "Stocks", dec!(0.5));
    let o = category(&mut p, "Bonds", dec!(0.5));
    let a = product(&mut p, "A", c, "JPY");
    let b = product(&mut p, "B", c, "JPY");
    let x = product(&mut p, "X", o, "JPY");
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, a, dec!(100000));
    hold(&mut p, acc, b, dec!(300000));
    hold(&mut p, acc, x, dec!(500000));

    // Only stocks are below target, and inside them the purchase goes to the smaller product
    // (A), not half to each.
    let buy = run(
        &p,
        acc,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();
    assert!(buy.products.iter().all(|l| l.delta > Decimal::ZERO));
    assert_eq!(deltas(&buy), vec![(a, dec!(100000))]);
}

// ---- buy only to target ---------------------------------------------------

#[test]
fn buy_only_to_target_computes_the_amount_without_selling() {
    let f = fixture();

    let s = run(&f.portfolio, f.main, Mode::BuyOnlyToTarget).unwrap();

    // Stocks (700,000 / 0.6) set the level: the target total is 1,166,666.67,
    // so 266,666.67 is needed, rounded up to 267,000 to spend.
    let required = s.required_total.unwrap();
    assert!((required - dec!(266666.666666666666666666667)).abs() < dec!(0.000001));
    assert_eq!(s.applied, dec!(267000));
    assert!(s.products.iter().all(|l| l.delta > Decimal::ZERO));
    assert_eq!(category_line(&s, f.stocks).delta, dec!(0));
}

#[test]
fn buy_only_to_target_on_a_balanced_portfolio_has_nothing_to_do() {
    let mut f = fixture();
    hold(&mut f.portfolio, f.main, f.s1, dec!(600000));
    hold(&mut f.portfolio, f.main, f.b1, dec!(300000));

    let s = run(&f.portfolio, f.main, Mode::BuyOnlyToTarget).unwrap();

    assert!(s.products.is_empty());
    assert_eq!(s.required_total, Some(Decimal::ZERO));
    assert_eq!(s.notes, vec![Note::AlreadyBalanced]);
}

#[test]
fn buy_only_to_target_reports_what_the_account_cannot_buy() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::SetTradable {
            id: f.sub,
            products: Some(vec![f.s1]),
        },
    );

    let s = run(&f.portfolio, f.sub, Mode::BuyOnlyToTarget).unwrap();

    assert!(
        s.notes
            .iter()
            .any(|n| matches!(n, Note::CategoryNotFilled { category, .. } if *category == f.bonds))
    );
}

#[test]
fn a_zero_target_category_that_holds_value_blocks_buy_only() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.bonds,
            name: None,
            target_weight: Some(dec!(0)),
            targeting: None,
        },
    );
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.stocks,
            name: None,
            target_weight: Some(dec!(1)),
            targeting: None,
        },
    );

    assert_eq!(
        run(&f.portfolio, f.main, Mode::BuyOnlyToTarget),
        Err(Error::CannotReachWithoutSelling)
    );
}

// ---- fixed sell -----------------------------------------------------------

#[test]
fn a_fixed_sell_takes_from_the_most_overweight_category_first() {
    let f = fixture();

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedSell {
            amount: dec!(100000),
        },
    )
    .unwrap();

    assert_eq!(deltas(&s), vec![(f.s1, dec!(-100000))]);
    assert_eq!(s.total_after, dec!(800000));
}

#[test]
fn selling_everything_the_account_holds_is_allowed_but_not_more() {
    let f = fixture();

    let all = run(
        &f.portfolio,
        f.main,
        Mode::FixedSell {
            amount: dec!(800000),
        },
    )
    .unwrap();
    let too_much = run(
        &f.portfolio,
        f.main,
        Mode::FixedSell {
            amount: dec!(801000),
        },
    );

    assert_eq!(
        deltas(&all),
        vec![(f.s1, dec!(-700000)), (f.b1, dec!(-100000))]
    );
    assert_eq!(too_much, Err(Error::NotEnoughToSell));
}

#[test]
fn an_account_that_may_not_sell_cannot_get_a_sell_suggestion() {
    let f = fixture();

    assert_eq!(
        run(&f.portfolio, f.sub, Mode::FixedSell { amount: dec!(1000) }),
        Err(Error::NotSellable)
    );
}

#[test]
fn a_record_of_a_type_that_may_not_be_sold_is_not_sold() {
    let mut f = fixture();
    // The stock fund of the main account (700,000) becomes a NISA record: only the 100,000 of
    // the bond fund is left to sell.
    apply(
        &mut f.portfolio,
        Edit::SetRecordType {
            account: f.main,
            product: f.s1,
            from: None,
            to: Some("nisa".to_owned()),
        },
    );

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedSell {
            amount: dec!(100000),
        },
    )
    .unwrap();
    let too_much = run(
        &f.portfolio,
        f.main,
        Mode::FixedSell {
            amount: dec!(101000),
        },
    );

    assert_eq!(deltas(&s), vec![(f.b1, dec!(-100000))]);
    assert_eq!(too_much, Err(Error::NotEnoughToSell));
}

#[test]
fn a_sale_never_exceeds_what_is_held_of_a_product_even_after_rounding() {
    let mut f = fixture();
    // 1,500 held, unit 1,000: rounding must not sell 2,000.
    hold(&mut f.portfolio, f.main, f.b1, dec!(1500));
    hold(&mut f.portfolio, f.main, f.s1, dec!(1500));

    for amount in [1000, 2000, 3000] {
        let result = run(
            &f.portfolio,
            f.main,
            Mode::FixedSell {
                amount: Decimal::from(amount),
            },
        );
        if let Ok(s) = result {
            for line in &s.products {
                assert!(-line.delta <= dec!(1500), "{line:?}");
            }
        }
    }
}

#[test]
fn a_fixed_sell_always_adds_up_and_stays_within_holdings() {
    let f = fixture();

    for amount in (1..=20).map(|n| Decimal::from(n * 40_000)) {
        let s = run(&f.portfolio, f.main, Mode::FixedSell { amount }).unwrap();

        assert!(s.products.iter().all(|l| l.delta < Decimal::ZERO));
        let sold: Decimal = s.products.iter().map(|l| -l.delta).sum();
        assert_eq!(sold, s.applied);
        for line in &s.products {
            let held = if line.product == f.s1 {
                dec!(700000)
            } else {
                dec!(100000)
            };
            assert!(-line.delta <= held);
        }
    }
}

// ---- full rebalance -------------------------------------------------------

#[test]
fn a_full_rebalance_sells_the_excess_and_buys_the_shortfall() {
    let f = fixture();

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    // Total 900,000: stocks should be 540,000 (sell 160,000), bonds 360,000.
    assert_eq!(
        deltas(&s),
        vec![(f.s1, dec!(-160000)), (f.b1, dec!(160000))]
    );
    assert_eq!(s.total_after, dec!(900000));
    assert_eq!(category_line(&s, f.stocks).after, dec!(540000));
    assert!(s.notes.is_empty());
}

#[test]
fn a_full_rebalance_works_for_any_account_that_may_sell() {
    let mut f = fixture();
    // No rebalancing account is set. The sub account may sell and holds the stocks.
    apply(
        &mut f.portfolio,
        Edit::UpdateAccount {
            id: f.sub,
            name: None,
            sellable: Some(true),
            rounding_unit: None,
        },
    );
    hold(&mut f.portfolio, f.main, f.s1, dec!(50000));
    hold(&mut f.portfolio, f.sub, f.s1, dec!(650000));

    let s = run(&f.portfolio, f.sub, Mode::FullRebalance).unwrap();

    // Only the sub account trades: it sells the stock excess and buys what it holds of bonds.
    assert_eq!(s.account, f.sub);
    assert_eq!(
        deltas(&s),
        vec![(f.s1, dec!(-160000)), (f.b1, dec!(160000))]
    );
}

#[test]
fn a_full_rebalance_in_an_account_that_may_not_sell_only_reports() {
    let f = fixture();

    let s = run(&f.portfolio, f.sub, Mode::FullRebalance).unwrap();

    assert!(s.products.is_empty());
    assert!(
        s.notes
            .iter()
            .any(|n| matches!(n, Note::CategoryNotFilled { .. }))
    );
}

#[test]
fn deviations_within_the_tolerance_are_left_alone() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::SetTolerances {
            category: dec!(0.2),
            product: dec!(0),
        },
    );

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    assert!(s.products.is_empty());
    assert_eq!(s.notes, vec![Note::AlreadyBalanced]);
}

#[test]
fn a_full_rebalance_reports_what_the_account_cannot_do() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateAccount {
            id: f.main,
            name: None,
            sellable: Some(false),
            rounding_unit: None,
        },
    );

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    assert!(s.products.is_empty());
    assert_eq!(
        s.notes
            .iter()
            .filter(|n| matches!(n, Note::CategoryNotFilled { .. }))
            .count(),
        2
    );
}

#[test]
fn a_partial_full_rebalance_balances_buys_and_sells() {
    let mut f = fixture();
    // The account holds only 50,000 of stocks, so it can sell at most that.
    hold(&mut f.portfolio, f.main, f.s1, dec!(50000));
    hold(&mut f.portfolio, f.sub, f.s1, dec!(650000));

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    let bought: Decimal = s
        .products
        .iter()
        .filter(|l| l.delta > Decimal::ZERO)
        .map(|l| l.delta)
        .sum();
    let sold: Decimal = s
        .products
        .iter()
        .filter(|l| l.delta < Decimal::ZERO)
        .map(|l| -l.delta)
        .sum();
    assert_eq!(bought, sold);
    assert_eq!(sold, dec!(50000));
    assert!(
        s.notes
            .iter()
            .any(|n| matches!(n, Note::CategoryNotFilled { .. }))
    );
}

// ---- valuation and guards -------------------------------------------------

#[test]
fn usd_holdings_are_valued_with_the_exchange_rate() {
    let mut f = fixture();
    let usd = product(&mut f.portfolio, "US Fund", f.stocks, "USD");
    hold(&mut f.portfolio, f.main, usd, dec!(1000));
    apply(
        &mut f.portfolio,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150)),
        },
    );

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();

    // 1,000 USD at 150 = 150,000 JPY: the total is now 1,050,000.
    assert_eq!(s.total_after, dec!(1150000));
    assert_eq!(category_line(&s, f.stocks).current, dec!(850000));
}

#[test]
fn a_missing_rate_or_unsupported_currency_is_reported() {
    let mut f = fixture();
    let usd = product(&mut f.portfolio, "US Fund", f.stocks, "USD");
    hold(&mut f.portfolio, f.main, usd, dec!(1000));

    assert!(matches!(
        run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(1000) }),
        Err(Error::Blocked(_))
    ));

    let mut f = fixture();
    let eur = product(&mut f.portfolio, "EU Fund", f.stocks, "EUR");
    hold(&mut f.portfolio, f.main, eur, dec!(10));
    apply(
        &mut f.portfolio,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150)),
        },
    );
    assert_eq!(
        run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(1000) }),
        Err(Error::UnsupportedCurrency("EUR".to_owned()))
    );
}

#[test]
fn problems_in_the_portfolio_block_suggestions() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.stocks,
            name: None,
            target_weight: Some(dec!(0.5)),
            targeting: None,
        },
    );

    let result = run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(1000) });

    assert!(matches!(result, Err(Error::Blocked(issues)) if !issues.is_empty()));
}

#[test]
fn bad_requests_are_rejected() {
    let f = fixture();

    assert_eq!(
        run(&f.portfolio, AccountId(999), Mode::BuyOnlyToTarget),
        Err(Error::UnknownAccount)
    );
    for amount in [dec!(0), dec!(-5)] {
        assert_eq!(
            run(&f.portfolio, f.main, Mode::FixedAdd { amount }),
            Err(Error::InvalidAmount)
        );
        assert_eq!(
            run(&f.portfolio, f.main, Mode::FixedSell { amount }),
            Err(Error::InvalidAmount)
        );
    }
    assert_eq!(
        run(
            &f.portfolio,
            f.main,
            Mode::FixedAdd {
                amount: MAX_AMOUNT + dec!(1)
            }
        ),
        Err(Error::TooLarge)
    );
}

#[test]
fn an_empty_portfolio_has_nothing_to_suggest() {
    let mut p = Portfolio::default();
    category(&mut p, "All", dec!(1));
    let acc = account(&mut p, "Main");

    assert_eq!(
        run(&p, acc, Mode::BuyOnlyToTarget),
        Err(Error::EmptyPortfolio)
    );
}

#[test]
fn huge_holdings_and_weights_are_refused_instead_of_overflowing() {
    let mut f = fixture();
    hold(&mut f.portfolio, f.main, f.s1, MAX_AMOUNT + dec!(1));
    assert_eq!(
        run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(1000) }),
        Err(Error::TooLarge)
    );

    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::SetHoldingWeight {
            account_type: None,
            account: f.main,
            product: f.s1,
            weight: Some(MAX_WEIGHT + dec!(1)),
        },
    );
    assert_eq!(
        run(&f.portfolio, f.main, Mode::FixedAdd { amount: dec!(1000) }),
        Err(Error::TooLarge)
    );
}

#[test]
fn suggestions_never_change_the_portfolio() {
    let f = fixture();
    let before = f.portfolio.clone();

    run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();
    run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    assert_eq!(f.portfolio, before);
}

#[test]
fn requests_and_suggestions_use_exact_decimal_strings_in_json() {
    let f = fixture();
    let request: Request =
        serde_json::from_str(r#"{"account":5,"mode":{"type":"fixed_add","amount":"100000"}}"#)
            .unwrap();
    assert_eq!(
        request,
        Request {
            account: AccountId(5),
            mode: Mode::FixedAdd {
                amount: dec!(100000)
            },
            held_only: false
        }
    );
    assert!(
        serde_json::from_str::<Request>(r#"{"account":5,"mode":{"type":"sell_all"}}"#).is_err()
    );

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();
    let json = serde_json::to_string(&s).unwrap();
    assert!(json.contains(r#""applied":"100000""#));
    assert!(json.contains(r#""mode":"fixed_add""#));
}

// ---- overview -------------------------------------------------------------

#[test]
fn overview_reports_value_and_share_per_category() {
    let f = fixture();

    let o = overview(&f.portfolio).unwrap();

    assert_eq!(o.total, dec!(900000));
    let stocks = o
        .categories
        .iter()
        .find(|c| c.category == f.stocks)
        .unwrap();
    assert_eq!(stocks.current, dec!(700000));
    assert_eq!(stocks.current_share, dec!(0.7778));
    assert_eq!(stocks.target_share, dec!(0.6));
    assert_eq!(stocks.difference, dec!(0.1778));
    let bonds = o.categories.iter().find(|c| c.category == f.bonds).unwrap();
    assert_eq!(bonds.current, dec!(200000));
}

#[test]
fn overview_totals_every_account_and_they_add_up_to_the_whole() {
    let f = fixture();

    let o = overview(&f.portfolio).unwrap();

    assert_eq!(o.accounts.len(), f.portfolio.accounts.len());
    let sum: Decimal = o.accounts.iter().map(|a| a.value).sum();
    assert_eq!(sum, o.total);
    for account in &o.accounts {
        let own: Decimal = o
            .holdings
            .iter()
            .filter(|h| h.account == account.account)
            .map(|h| h.value)
            .sum();
        assert_eq!(account.value, own);
    }
}

#[test]
fn overview_of_an_empty_portfolio_has_zero_shares() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "A", dec!(1));

    let o = overview(&p).unwrap();

    assert_eq!(o.total, Decimal::ZERO);
    assert_eq!(o.categories[0].category, c);
    assert_eq!(o.categories[0].current_share, Decimal::ZERO);
}

#[test]
fn overview_works_while_the_targets_are_incomplete() {
    let mut p = Portfolio::default();
    category(&mut p, "A", dec!(0.5));

    assert!(overview(&p).is_ok());
}

#[test]
fn overview_lists_each_holding_with_its_share() {
    let f = fixture();

    let o = overview(&f.portfolio).unwrap();

    assert_eq!(o.holdings.len(), 3);
    let main_stock = o
        .holdings
        .iter()
        .find(|h| h.account == f.main && h.product == f.s1)
        .unwrap();
    assert_eq!(main_stock.value, dec!(700000));
    assert_eq!(main_stock.share, dec!(0.7778));
}

#[test]
fn overview_totals_each_type_and_they_add_up_to_the_whole() {
    let mut f = fixture();
    let (account, product) = (f.main, f.s1);
    f.portfolio
        .apply(Edit::SetHolding {
            account_type: Some("nisa".to_owned()),
            account,
            product,
            value: dec!(100000),
            quantity: None,
        })
        .unwrap();

    let o = overview(&f.portfolio).unwrap();

    let sum: Decimal = o.types.iter().map(|t| t.value).sum();
    assert_eq!(sum, o.total);
    let nisa = o
        .types
        .iter()
        .find(|t| t.account_type.as_deref() == Some("nisa"))
        .unwrap();
    assert_eq!(nisa.value, dec!(100000));
    // The records without a type are listed too, first.
    assert_eq!(o.types[0].account_type, None);
}

// ---- account balance ------------------------------------------------------

/// One category with products A (weight 3) and B (weight 1); the account
/// holds 100,000 of A and 100,000 of B.
fn weighted_pair() -> (Portfolio, AccountId, ProductId, ProductId, CategoryId) {
    let mut p = Portfolio::default();
    let c = category(&mut p, "All", dec!(1));
    let a = product(&mut p, "A", c, "JPY");
    let b = product(&mut p, "B", c, "JPY");
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, a, dec!(100000));
    hold(&mut p, acc, b, dec!(100000));
    apply(
        &mut p,
        Edit::UpdateCategory {
            id: c,
            name: None,
            target_weight: None,
            targeting: Some(ProductTargeting::Weighted),
        },
    );
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: acc,
            product: a,
            weight: Some(dec!(3)),
        },
    );
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: acc,
            product: b,
            weight: Some(dec!(1)),
        },
    );
    (p, acc, a, b, c)
}

#[test]
fn account_balance_compares_products_with_their_weights() {
    let (p, acc, a, b, c) = weighted_pair();

    let balance = account_balance(&p, acc).unwrap();

    let line = &balance.categories.iter().find(|x| x.category == c).unwrap();
    assert_eq!(line.total, dec!(200000));
    let for_product = |id| line.products.iter().find(|x| x.product == id).unwrap();
    assert_eq!(for_product(a).target, Some(dec!(150000)));
    assert_eq!(for_product(a).shortfall, Some(dec!(50000)));
    assert_eq!(for_product(b).target, Some(dec!(50000)));
    assert_eq!(for_product(b).shortfall, Some(dec!(-50000)));
}

#[test]
fn account_balance_has_no_target_when_the_category_does_not_split() {
    let (mut p, acc, a, _, c) = weighted_pair();
    apply(
        &mut p,
        Edit::UpdateCategory {
            id: c,
            name: None,
            target_weight: None,
            targeting: Some(ProductTargeting::Free),
        },
    );

    let balance = account_balance(&p, acc).unwrap();

    let line = &balance.categories[0];
    let product = line.products.iter().find(|x| x.product == a).unwrap();
    assert_eq!(product.current, dec!(100000));
    assert_eq!(product.target, None);
    assert_eq!(product.shortfall, None);
}

#[test]
fn account_balance_leaves_out_products_the_account_does_not_hold() {
    let (mut p, acc, a, b, c) = weighted_pair();
    let other = product(&mut p, "C", c, "JPY");
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: acc,
            product: other,
            weight: Some(dec!(4)),
        },
    );
    apply(
        &mut p,
        Edit::SetTradable {
            id: acc,
            products: Some(vec![a, other]),
        },
    );

    let balance = account_balance(&p, acc).unwrap();

    let ids: Vec<_> = balance.categories[0]
        .products
        .iter()
        .map(|x| x.product)
        .collect();
    // C is buyable and has a weight, but the account holds none of it: out of scope.
    assert_eq!(ids, vec![a, b]);
}

#[test]
fn account_balance_rejects_an_unknown_account() {
    let (p, _, _, _, _) = weighted_pair();

    assert_eq!(
        account_balance(&p, AccountId(9999)),
        Err(Error::UnknownAccount)
    );
}

#[test]
fn account_balance_targets_add_up_to_the_category_total() {
    let mut p = Portfolio::default();
    let c = category(&mut p, "All", dec!(1));
    let ids: Vec<_> = ["A", "B", "C"]
        .into_iter()
        .map(|n| product(&mut p, n, c, "JPY"))
        .collect();
    let acc = account(&mut p, "Main");
    hold(&mut p, acc, ids[0], dec!(100));
    apply(
        &mut p,
        Edit::UpdateCategory {
            id: c,
            name: None,
            target_weight: None,
            targeting: Some(ProductTargeting::Equal),
        },
    );

    let balance = account_balance(&p, acc).unwrap();

    let line = &balance.categories[0];
    let sum: Decimal = line.products.iter().filter_map(|x| x.target).sum();
    assert_eq!(sum, dec!(100));
    let shortfalls: Decimal = line.products.iter().filter_map(|x| x.shortfall).sum();
    assert_eq!(shortfalls, Decimal::ZERO);
}

#[test]
fn the_same_product_has_its_own_weight_in_each_account() {
    let (mut p, acc, a, b, c) = weighted_pair();
    let other = account(&mut p, "Second");
    hold(&mut p, other, a, dec!(100000));
    hold(&mut p, other, b, dec!(100000));
    // In the second account B is wanted three times as much as A.
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: other,
            product: a,
            weight: Some(dec!(1)),
        },
    );
    apply(
        &mut p,
        Edit::SetHoldingWeight {
            account_type: None,
            account: other,
            product: b,
            weight: Some(dec!(3)),
        },
    );

    let target = |account, product| {
        let balance = account_balance(&p, account).unwrap();
        let line = balance.categories.iter().find(|x| x.category == c).unwrap();
        line.products
            .iter()
            .find(|x| x.product == product)
            .unwrap()
            .target
    };
    assert_eq!(target(acc, a), Some(dec!(150000)));
    assert_eq!(target(other, a), Some(dec!(50000)));
    assert_eq!(target(other, b), Some(dec!(150000)));
}

#[test]
fn overview_has_no_groups_while_no_group_exists() {
    let f = fixture();

    assert!(overview(&f.portfolio).unwrap().groups.is_empty());
}

#[test]
fn overview_sums_the_categories_of_a_group_and_shows_the_rest_as_ungrouped() {
    let mut f = fixture();
    let extra = category(&mut f.portfolio, "Gold", dec!(0));
    let g = product(&mut f.portfolio, "Gold Fund", extra, "JPY");
    hold(&mut f.portfolio, f.main, g, dec!(100000));
    apply(
        &mut f.portfolio,
        Edit::AddGroup {
            name: "Safe".to_owned(),
        },
    );
    let safe = f.portfolio.groups[0].id;
    for category in [f.bonds, extra] {
        apply(
            &mut f.portfolio,
            Edit::SetCategoryGroup {
                category,
                group: Some(safe),
            },
        );
    }

    let o = overview(&f.portfolio).unwrap();

    assert_eq!(o.total, dec!(1000000));
    assert_eq!(o.groups.len(), 2);
    assert_eq!(o.groups[0].group, Some(safe));
    assert_eq!(o.groups[0].current, dec!(300000));
    assert_eq!(o.groups[0].current_share, dec!(0.3));
    assert_eq!(o.groups[0].target_share, dec!(0.4));
    assert_eq!(o.groups[0].difference, dec!(-0.1));
    // Stocks belong to no group.
    assert_eq!(o.groups[1].group, None);
    assert_eq!(o.groups[1].current, dec!(700000));
    assert_eq!(o.groups[1].target_share, dec!(0.6));
}

#[test]
fn overview_leaves_out_ungrouped_when_every_category_has_a_group_but_keeps_an_empty_group() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::AddGroup {
            name: "All".to_owned(),
        },
    );
    apply(
        &mut f.portfolio,
        Edit::AddGroup {
            name: "Empty".to_owned(),
        },
    );
    let all = f.portfolio.groups[0].id;
    for category in [f.stocks, f.bonds] {
        apply(
            &mut f.portfolio,
            Edit::SetCategoryGroup {
                category,
                group: Some(all),
            },
        );
    }

    let o = overview(&f.portfolio).unwrap();

    let ids: Vec<_> = o.groups.iter().map(|g| g.group).collect();
    assert_eq!(ids, [Some(all), Some(f.portfolio.groups[1].id)]);
    assert_eq!(o.groups[0].current_share, dec!(1));
    assert_eq!(o.groups[1].current, dec!(0));
}

// ---- display rounding (the structs stay exact; only the wire form is whole yen) ----

/// A USD fund worth 10.5 USD at 150.55 yen is 1,580.775 yen: a fraction below a yen.
fn fractional_fixture() -> Fixture {
    let mut f = fixture();
    let usd = product(&mut f.portfolio, "US Fund", f.stocks, "USD");
    hold(&mut f.portfolio, f.main, usd, dec!(10.5));
    apply(
        &mut f.portfolio,
        Edit::SetExchangeRate {
            usd_jpy: Some(dec!(150.55)),
        },
    );
    f
}

#[test]
fn the_overview_keeps_exact_values_and_serializes_whole_yen() {
    let f = fractional_fixture();

    let o = overview(&f.portfolio).unwrap();

    assert_eq!(o.total, dec!(901580.775));
    let json: serde_json::Value = serde_json::to_value(&o).unwrap();
    assert_eq!(json["total"], "901581");
    let stocks = json["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["category"] == f.stocks.0)
        .unwrap();
    assert_eq!(stocks["current"], "701581");
    // Ratios are not amounts and keep their own precision.
    assert_eq!(
        json["categories"][0]["current_share"],
        o.categories[0].current_share.to_string()
    );
}

#[test]
fn a_suggestion_keeps_exact_values_and_serializes_whole_yen() {
    let f = fractional_fixture();

    let s = run(
        &f.portfolio,
        f.main,
        Mode::FixedAdd {
            amount: dec!(100000),
        },
    )
    .unwrap();

    assert_eq!(category_line(&s, f.stocks).current, dec!(701580.775));
    let json: serde_json::Value = serde_json::to_value(&s).unwrap();
    let stocks = json["categories"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["category"] == f.stocks.0)
        .unwrap();
    assert_eq!(stocks["current"], "701581");
}

#[test]
fn the_account_balance_serializes_whole_yen_and_keeps_exact_values() {
    let f = fractional_fixture();

    let b = account_balance(&f.portfolio, f.main).unwrap();

    let exact = b
        .categories
        .iter()
        .find(|c| c.category == f.stocks)
        .unwrap()
        .total;
    assert_eq!(exact, dec!(701580.775));
    let json: serde_json::Value = serde_json::to_value(&b).unwrap();
    let totals: Vec<&serde_json::Value> = json["categories"]
        .as_array()
        .unwrap()
        .iter()
        .map(|c| &c["total"])
        .collect();
    assert!(totals.contains(&&serde_json::Value::from("701581")));
}

#[test]
fn held_only_never_buys_a_product_the_account_does_not_hold() {
    let f = fixture();
    // The sub account holds only bonds; stocks are under no restriction but not held there.
    let request = |held_only| Request {
        account: f.sub,
        mode: Mode::FixedAdd {
            amount: dec!(2000000),
        },
        held_only,
    };
    let all = suggest(&f.portfolio, &request(false)).unwrap();
    let held = suggest(&f.portfolio, &request(true)).unwrap();

    assert!(all.products.iter().any(|l| l.product == f.s1));
    assert!(held.products.iter().all(|l| l.product != f.s1));
    assert!(held.products.iter().any(|l| l.product == f.b1));
}

#[test]
fn held_only_also_applies_to_the_rebalancing_account() {
    let mut f = fixture();
    // A second bond fund that nobody holds: bonds are under target, so it gets money.
    let b2 = product(&mut f.portfolio, "Bond Fund 2", f.bonds, "JPY");
    apply(
        &mut f.portfolio,
        Edit::SetTargetAccount {
            account: Some(f.main),
        },
    );
    let request = |mode, held_only| Request {
        account: f.main,
        mode,
        held_only,
    };
    for mode in [
        Mode::FixedAdd {
            amount: dec!(300000),
        },
        Mode::BuyOnlyToTarget,
        Mode::FullRebalance,
    ] {
        let any = suggest(&f.portfolio, &request(mode.clone(), false)).unwrap();
        let held = suggest(&f.portfolio, &request(mode, true)).unwrap();
        assert!(any.products.iter().any(|l| l.product == b2));
        assert!(held.products.iter().all(|l| l.product != b2));
        assert!(held.products.iter().any(|l| l.product == f.b1));
    }
}

#[test]
fn held_only_defaults_to_false_in_json() {
    let request: Request =
        serde_json::from_str(r#"{"account":5,"mode":{"type":"buy_only_to_target"}}"#).unwrap();
    assert!(!request.held_only);
}

// ---- rounding keeps the total (#141) --------------------------------------

fn set_unit(f: &mut Fixture, unit: Decimal) {
    for account in [f.main, f.sub] {
        apply(
            &mut f.portfolio,
            Edit::UpdateAccount {
                id: account,
                name: None,
                sellable: None,
                rounding_unit: Some(unit),
            },
        );
    }
}

fn sum_of(s: &Suggestion, sign: Decimal) -> Decimal {
    s.products
        .iter()
        .filter(|l| l.delta * sign > Decimal::ZERO)
        .map(|l| l.delta.abs())
        .sum()
}

#[test]
fn a_fixed_sell_reaches_the_amount_when_holdings_are_not_multiples_of_the_unit() {
    let mut f = fixture();
    // Each holds 1,500 with a unit of 1,000: only 1,000 of each can be sold, so
    // 2,000 is possible but the two halves of 3,000 are not.
    hold(&mut f.portfolio, f.main, f.b1, dec!(1500));
    hold(&mut f.portfolio, f.main, f.s1, dec!(1500));

    let s = run(&f.portfolio, f.main, Mode::FixedSell { amount: dec!(2000) }).unwrap();

    assert_eq!(sum_of(&s, dec!(-1)), dec!(2000));
    assert!(s.products.iter().all(|l| l.delta == dec!(-1000)));
    // 3,000 would need more than can be sold in whole units.
    assert_eq!(
        run(&f.portfolio, f.main, Mode::FixedSell { amount: dec!(3000) }),
        Err(Error::NotEnoughToSell)
    );
}

#[test]
fn a_fixed_sell_adds_up_for_awkward_units() {
    for unit in [dec!(100), dec!(1000), dec!(10000)] {
        let mut f = fixture();
        set_unit(&mut f, unit);
        // Holdings that are not multiples of any of the units.
        hold(&mut f.portfolio, f.main, f.s1, dec!(703457));
        hold(&mut f.portfolio, f.main, f.b1, dec!(98765));
        for n in 1..=40 {
            let amount = Decimal::from(n * 20_000) + dec!(333);
            let result = run(&f.portfolio, f.main, Mode::FixedSell { amount });
            let Ok(s) = result else { continue };
            assert_eq!(sum_of(&s, dec!(-1)), s.applied, "unit {unit}, {amount}");
            assert_eq!(s.applied, floor_unit(amount, unit), "unit {unit}, {amount}");
            for line in &s.products {
                assert!((line.delta % unit).is_zero());
                let held = if line.product == f.s1 {
                    dec!(703457)
                } else {
                    dec!(98765)
                };
                assert!(-line.delta <= held);
            }
        }
    }
}

fn floor_unit(amount: Decimal, unit: Decimal) -> Decimal {
    (amount / unit).floor() * unit
}

#[test]
fn a_fixed_add_over_many_products_adds_up_for_awkward_units() {
    for unit in [dec!(100), dec!(1000), dec!(10000)] {
        let mut f = fixture();
        set_unit(&mut f, unit);
        for name in ["S2", "S3"] {
            product(&mut f.portfolio, name, f.stocks, "JPY");
        }
        for name in ["B2", "B3", "B4"] {
            product(&mut f.portfolio, name, f.bonds, "JPY");
        }
        for n in 1..=40 {
            let amount = Decimal::from(n * 17_777) + dec!(55);
            let s = run(&f.portfolio, f.main, Mode::FixedAdd { amount }).unwrap();
            assert_eq!(
                sum_of(&s, dec!(1)),
                floor_unit(amount, unit),
                "{unit} {amount}"
            );
            assert_eq!(s.applied, floor_unit(amount, unit));
            assert!(s.products.iter().all(|l| (l.delta % unit).is_zero()));
        }
    }
}

#[test]
fn a_full_rebalance_buys_and_sells_the_same_total_for_awkward_units() {
    for unit in [dec!(100), dec!(1000), dec!(10000)] {
        let mut f = fixture();
        set_unit(&mut f, unit);
        product(&mut f.portfolio, "B2", f.bonds, "JPY");
        product(&mut f.portfolio, "S2", f.stocks, "JPY");
        let s2 = f.portfolio.products.last().unwrap().id;
        hold(&mut f.portfolio, f.main, f.s1, dec!(703457));
        hold(&mut f.portfolio, f.main, s2, dec!(1500));
        hold(&mut f.portfolio, f.main, f.b1, dec!(98765));

        let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

        assert_eq!(sum_of(&s, dec!(1)), sum_of(&s, dec!(-1)), "unit {unit}");
        assert!(s.products.iter().all(|l| (l.delta % unit).is_zero()));
    }
}

#[test]
fn a_full_rebalance_cannot_sell_what_is_below_the_unit_and_says_so() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.stocks,
            name: None,
            target_weight: Some(dec!(0.1)),
            targeting: None,
        },
    );
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.bonds,
            name: None,
            target_weight: Some(dec!(0.9)),
            targeting: None,
        },
    );
    // Stocks are far overweight but are held in four pieces of 900, each below
    // the unit of 1,000, so nothing can be sold in whole units.
    hold(&mut f.portfolio, f.main, f.s1, dec!(900));
    for name in ["S2", "S3", "S4"] {
        let id = product(&mut f.portfolio, name, f.stocks, "JPY");
        hold(&mut f.portfolio, f.main, id, dec!(900));
    }
    hold(&mut f.portfolio, f.main, f.b1, dec!(0));
    hold(&mut f.portfolio, f.sub, f.b1, dec!(4000));

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    assert!(s.products.is_empty());
    assert!(!s.notes.is_empty());
}

#[test]
fn a_full_rebalance_sells_only_whole_units_when_pieces_are_not_multiples() {
    let mut f = fixture();
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.stocks,
            name: None,
            target_weight: Some(dec!(0.1)),
            targeting: None,
        },
    );
    apply(
        &mut f.portfolio,
        Edit::UpdateCategory {
            id: f.bonds,
            name: None,
            target_weight: Some(dec!(0.9)),
            targeting: None,
        },
    );
    // Pieces of 1,900 can give 1,000 each: the overweight is 2,000+ but the
    // trade must stay at what whole units can deliver.
    hold(&mut f.portfolio, f.main, f.s1, dec!(1900));
    for name in ["S2", "S3"] {
        let id = product(&mut f.portfolio, name, f.stocks, "JPY");
        hold(&mut f.portfolio, f.main, id, dec!(1900));
    }
    hold(&mut f.portfolio, f.main, f.b1, dec!(0));
    hold(&mut f.portfolio, f.sub, f.b1, dec!(2000));

    let s = run(&f.portfolio, f.main, Mode::FullRebalance).unwrap();

    assert_eq!(sum_of(&s, dec!(1)), sum_of(&s, dec!(-1)));
    assert!(sum_of(&s, dec!(-1)) <= dec!(3000));
    assert!(s.products.iter().all(|l| (l.delta % dec!(1000)).is_zero()));
}
