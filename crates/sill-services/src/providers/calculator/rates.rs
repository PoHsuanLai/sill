//! Currency rates for the calculator (M9 lane c, FINDINGS "M9 freeze" F650). Off by default
//! (`launcher.currency = "off"`): nothing is fetched and currency codes are not units. With
//! `ecb`, the European Central Bank's daily reference rates (`eurofxref-daily.xml`, EUR based)
//! are fetched at most once a day by `curl` into `$XDG_CACHE_HOME/sill/ecb-rates.xml`, off the
//! keystroke path, and handed to fend through its `ExchangeRateFnV2` (base EUR, which is 1).
//! Units need no network: fend converts them already.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use sill_launcher::UnixTime;

/// The ECB's daily file.
pub const ECB_DAILY_URL: &str = "https://www.ecb.europa.eu/stats/eurofxref/eurofxref-daily.xml";

/// Units of a currency per euro, by ISO 4217 code (upper case), and the day they are for.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Rates {
    pub per_euro: BTreeMap<String, f64>,
    /// The file's `time` attribute, as a day's start (UTC); `None` before any fetch.
    pub day: Option<UnixTime>,
}

/// Why a rates file could not be read. Carries no file content.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum RatesFault {
    NotEcb,
    NoRates,
}

/// The rates the provider converts with, shared with the fetcher.
pub type SharedRates = Arc<Mutex<Rates>>;

/// The rates in an ECB `eurofxref-daily.xml`: every `<Cube currency="USD" rate="1.0876"/>`,
/// plus `EUR` at 1. A small attribute scan, not an XML parser (the file is three levels of
/// one element). Lane c fills it.
pub fn parse_ecb(xml: &str) -> Result<Rates, RatesFault> {
    let _ = xml;
    todo!("M9 fill: c-emoji-web-units")
}

/// `code`'s value relative to the base (EUR): `1 / per_euro[code]`, the shape fend's
/// `relative_to_base_currency` answers. `None` for an unknown code.
pub fn relative_to_euro(rates: &Rates, code: &str) -> Option<f64> {
    let _ = (rates, code);
    todo!("M9 fill: c-emoji-web-units")
}

#[cfg(test)]
mod tests {
    use super::{RatesFault, parse_ecb, relative_to_euro};

    const SAMPLE: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<gesmes:Envelope xmlns:gesmes="http://www.gesmes.org/xml/2002-08-01" xmlns="http://www.ecb.int/vocabulary/2002-08-01/eurofxref">
  <gesmes:subject>Reference rates</gesmes:subject>
  <Cube>
    <Cube time='2026-09-25'>
      <Cube currency='USD' rate='1.1000'/>
      <Cube currency='JPY' rate='160.00'/>
      <Cube currency='TWD' rate='35.500'/>
    </Cube>
  </Cube>
</gesmes:Envelope>"#;

    #[test]
    #[ignore = "M9 fill: c-emoji-web-units"]
    fn the_daily_file_becomes_rates_per_euro() {
        let rates = parse_ecb(SAMPLE).expect("rates");
        assert_eq!(rates.per_euro.get("USD"), Some(&1.1));
        assert_eq!(rates.per_euro.get("EUR"), Some(&1.0));
        assert_eq!(rates.per_euro.len(), 4);
        let usd = relative_to_euro(&rates, "USD").expect("usd");
        assert!((usd - 1.0 / 1.1).abs() < 1e-12);
        assert_eq!(relative_to_euro(&rates, "XXX"), None);
        assert_eq!(parse_ecb("<html/>"), Err(RatesFault::NotEcb));
    }
}
