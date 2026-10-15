// Every name and amount in these tests is made up.

use portfolio::{AccountId, AccountKind, CategoryId, Edit, Portfolio, ProductId, ProductTargeting};
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

fn account(p: &mut Portfolio, name: &str, kind: AccountKind) -> AccountId {
    apply(
        p,
        Edit::AddAccount {
            name: name.to_owned(),
            kind,
        },
    );
    p.accounts.last().unwrap().id
}

fn hold(p: &mut Portfolio, account: AccountId, product: ProductId, value: Decimal) {
    apply(
        p,
        Edit::SetHolding {
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
    let main = account(&mut p, "Main", AccountKind::Taxable);
    let sub = account(&mut p, "Sub", AccountKind::Points);
    apply(
        &mut p,
        Edit::UpdateAccount {
            id: sub,
            name: None,
            kind: None,
            sellable: Some(false),
            funding: None,
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
    suggest(p, &Request { account, mode })
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
            kind: None,
            sellable: None,
            funding: None,
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
    let acc = account(&mut p, "Main", AccountKind::Taxable);
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
        Edit::SetProductWeight {
            id: a,
            weight: Some(dec!(3)),
        },
    );
    apply(
        &mut p,
        Edit::SetProductWeight {
            id: b,
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
    let acc = account(&mut p, "Main", AccountKind::Taxable);
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
    // Free: no product targets, so new money is split evenly.
    assert_eq!(
        deltas(&equal(ProductTargeting::Free)),
        vec![(a, dec!(100000)), (b, dec!(100000))]
    );
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

fn with_target(f: &mut Fixture) {
    let account = f.main;
    apply(
        &mut f.portfolio,
        Edit::SetTargetAccount {
            account: Some(account),
        },
    );
}

#[test]
fn a_full_rebalance_sells_the_excess_and_buys_the_shortfall() {
    let mut f = fixture();
    with_target(&mut f);

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
fn a_full_rebalance_needs_the_rebalancing_account() {
    let mut f = fixture();

    assert_eq!(
        run(&f.portfolio, f.main, Mode::FullRebalance),
        Err(Error::NoTargetAccount)
    );
    with_target(&mut f);
    assert_eq!(
        run(&f.portfolio, f.sub, Mode::FullRebalance),
        Err(Error::NotTargetAccount)
    );
}

#[test]
fn deviations_within_the_tolerance_are_left_alone() {
    let mut f = fixture();
    with_target(&mut f);
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
    with_target(&mut f);
    apply(
        &mut f.portfolio,
        Edit::UpdateAccount {
            id: f.main,
            name: None,
            kind: None,
            sellable: Some(false),
            funding: None,
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
    with_target(&mut f);
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
    let acc = account(&mut p, "Main", AccountKind::Taxable);

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
        Edit::SetProductWeight {
            id: f.s1,
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
    let mut f = fixture();
    with_target(&mut f);
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
            }
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
