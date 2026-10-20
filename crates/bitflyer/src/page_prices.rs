//! Prices read from the public bitFlyer price page (a table the person opened in the browser).
//!
//! The page lists one price per currency pair ("BTC/JPY", "12,345 円"), so one price basis
//! covers every currency, including those with no Lightning market. The table is only read
//! here; fetching it is the job of the browser path, so this crate opens no new connection.

use std::collections::BTreeMap;
use std::str::FromStr;

use rust_decimal::Decimal;

use crate::Error;
use crate::model::validate_currency_code;

const PAIR_HEADING: &str = "通貨ペア";
const PRICE_HEADING: &str = "価格";
const JPY_SUFFIX: &str = "/JPY";

/// Yen prices by currency code.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PagePrices(BTreeMap<String, Decimal>);

impl PagePrices {
    pub fn get(&self, currency: &str) -> Option<Decimal> {
        self.0.get(currency).copied()
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Reads the table of the price page: a heading row ("通貨ペア", "価格", ...), then one row
    /// per pair. Rows that are not a `<CODE>/JPY` pair with a positive price are left out; a
    /// table with another heading is refused, so a wrong table is never read as prices.
    pub fn from_table(rows: &[Vec<String>]) -> Result<Self, Error> {
        let heading_ok = rows.first().is_some_and(|row| {
            row.first().is_some_and(|c| c.trim() == PAIR_HEADING)
                && row.get(1).is_some_and(|c| c.trim() == PRICE_HEADING)
        });
        if !heading_ok {
            return Err(Error::Format(
                "the table is not the bitFlyer price table".to_owned(),
            ));
        }
        let mut prices = BTreeMap::new();
        for row in &rows[1..] {
            let (Some(pair), Some(price)) = (row.first(), row.get(1)) else {
                continue;
            };
            let Some(code) = pair.trim().strip_suffix(JPY_SUFFIX) else {
                continue;
            };
            if validate_currency_code(code).is_err() {
                continue;
            }
            let Some(price) = parse_price(price) else {
                continue;
            };
            if prices.insert(code.to_owned(), price).is_some() {
                return Err(Error::Format(format!("{code} appears twice in the table")));
            }
        }
        if prices.is_empty() {
            return Err(Error::Format("the table has no prices".to_owned()));
        }
        Ok(Self(prices))
    }
}

/// "12,986,322 円" or "0.0007897 円" as a positive number.
fn parse_price(cell: &str) -> Option<Decimal> {
    let number: String = cell
        .trim()
        .trim_end_matches('円')
        .trim()
        .chars()
        .filter(|c| *c != ',')
        .collect();
    let price = Decimal::from_str(&number).ok()?;
    (price > Decimal::ZERO).then_some(price)
}

#[cfg(test)]
mod tests {
    use rust_decimal::dec;

    use super::*;

    fn table(rows: &[[&str; 3]]) -> Vec<Vec<String>> {
        rows.iter()
            .map(|r| r.iter().map(|c| (*c).to_owned()).collect())
            .collect()
    }

    #[test]
    fn reads_pairs_with_commas_and_small_decimals() {
        let rows = table(&[
            ["通貨ペア", "価格", "前日比"],
            ["AAA/JPY", "1,234,567 円", "-0.5%"],
            ["BBB/JPY", "0.0007897 円", "+0.2%"],
        ]);

        let prices = PagePrices::from_table(&rows).unwrap();

        assert_eq!(prices.get("AAA"), Some(dec!(1234567)));
        assert_eq!(prices.get("BBB"), Some(dec!(0.0007897)));
        assert_eq!(prices.get("CCC"), None);
        assert_eq!(prices.len(), 2);
    }

    #[test]
    fn leaves_out_rows_that_are_not_a_yen_pair_with_a_positive_price() {
        let rows = table(&[
            ["通貨ペア", "価格", "前日比"],
            ["AAA/USD", "10 円", ""],
            ["bbb/JPY", "10 円", ""],
            ["CCC/JPY", "-", ""],
            ["DDD/JPY", "0 円", ""],
            ["EEE/JPY", "5 円", ""],
        ]);

        let prices = PagePrices::from_table(&rows).unwrap();

        assert_eq!(prices.len(), 1);
        assert_eq!(prices.get("EEE"), Some(dec!(5)));
    }

    #[test]
    fn refuses_another_table_and_a_table_without_prices() {
        let other = table(&[["銘柄", "評価額", ""], ["AAA/JPY", "1 円", ""]]);
        let empty = table(&[["通貨ペア", "価格", "前日比"], ["AAA/USD", "1 円", ""]]);

        assert!(PagePrices::from_table(&other).is_err());
        assert!(PagePrices::from_table(&empty).is_err());
        assert!(PagePrices::from_table(&[]).is_err());
    }

    #[test]
    fn refuses_a_currency_listed_twice() {
        let rows = table(&[
            ["通貨ペア", "価格", "前日比"],
            ["AAA/JPY", "1 円", ""],
            ["AAA/JPY", "2 円", ""],
        ]);

        assert!(PagePrices::from_table(&rows).is_err());
    }
}
