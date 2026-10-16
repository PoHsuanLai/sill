//! Currency rates for the calculator. Off by default (`launcher.currency = "off"`): nothing is
//! fetched and currency codes are not units. With `ecb`, the European Central Bank's daily
//! reference rates (`eurofxref-daily.xml`, EUR based) are fetched at most once a day by `curl` into
//! `$XDG_CACHE_HOME/sill/ecb-rates.xml`, off the keystroke path, and handed to fend through its
//! `ExchangeRateFnV2` (base EUR, which is 1). Units need no network: fend converts them already.

use std::collections::BTreeMap;
use std::sync::{Arc, Mutex};

use fend_core::{ExchangeRateFnV2, ExchangeRateFnV2Options};
use jiff::civil::Date;
use jiff::tz::TimeZone;
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

/// A namespace fragment only the ECB's own files carry, checked before anything else is
/// parsed (`parse_ecb("<html/>")` is [`RatesFault::NotEcb`], not an empty table).
const ECB_MARKER: &str = "ecb.int/vocabulary";

/// The rates in an ECB `eurofxref-daily.xml`: every `<Cube currency="USD" rate="1.0876"/>`,
/// plus `EUR` at 1. A small attribute scan, not an XML parser (the file is three levels of
/// one element).
pub fn parse_ecb(xml: &str) -> Result<Rates, RatesFault> {
    if !xml.contains(ECB_MARKER) {
        return Err(RatesFault::NotEcb);
    }
    let day = xml
        .split("<Cube")
        .find_map(|tag| attribute(tag, "time"))
        .and_then(day_start);
    let mut per_euro = BTreeMap::new();
    per_euro.insert("EUR".to_owned(), 1.0);
    for tag in xml.split("<Cube").skip(1) {
        let (Some(code), Some(rate)) = (attribute(tag, "currency"), attribute(tag, "rate")) else {
            continue;
        };
        if let Ok(rate) = rate.parse::<f64>()
            && rate.is_finite()
            && rate > 0.0
        {
            per_euro.insert(code.to_uppercase(), rate);
        }
    }
    if per_euro.len() <= 1 {
        return Err(RatesFault::NoRates);
    }
    Ok(Rates { per_euro, day })
}

/// `name`'s value in `tag` (`<Cube currency="USD" .../`), single or double quoted.
fn attribute<'a>(tag: &'a str, name: &str) -> Option<&'a str> {
    let double = format!("{name}=\"");
    let single = format!("{name}='");
    let (after, quote) = if let Some(pos) = tag.find(&double) {
        (pos + double.len(), '"')
    } else {
        let pos = tag.find(&single)?;
        (pos + single.len(), '\'')
    };
    let rest = &tag[after..];
    let end = rest.find(quote)?;
    Some(&rest[..end])
}

/// `text` (`2026-09-25`) as the start of that day, UTC.
fn day_start(text: &str) -> Option<UnixTime> {
    let date: Date = text.parse().ok()?;
    let seconds = date.to_zoned(TimeZone::UTC).ok()?.timestamp().as_second();
    u64::try_from(seconds).ok().map(UnixTime)
}

/// `code`'s value relative to the base (EUR): `1 / per_euro[code]`, the shape fend's
/// `relative_to_base_currency` answers. `None` for an unknown code.
pub fn relative_to_euro(rates: &Rates, code: &str) -> Option<f64> {
    let per_euro = *rates.per_euro.get(&code.to_uppercase())?;
    (per_euro != 0.0).then_some(1.0 / per_euro)
}

/// Feeds `fend` a snapshot of `Rates` as its exchange-rate handler
/// (`Context::set_exchange_rate_handler_v2`): `evaluate` builds one from the shared rates it
/// was given for the one query, never touching the network itself.
#[derive(Debug, Clone)]
pub struct ExchangeRates(Rates);

impl ExchangeRates {
    /// Wraps `rates` for fend to query while this evaluation lasts.
    pub fn new(rates: Rates) -> ExchangeRates {
        ExchangeRates(rates)
    }
}

impl ExchangeRateFnV2 for ExchangeRates {
    fn relative_to_base_currency(
        &self,
        currency: &str,
        _options: &ExchangeRateFnV2Options,
    ) -> Result<f64, Box<dyn std::error::Error + Send + Sync + 'static>> {
        relative_to_euro(&self.0, currency)
            .ok_or_else(|| format!("no ECB rate for {currency}").into())
    }
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

    #[test]
    fn a_rate_that_is_not_a_finite_positive_number_is_no_rate() {
        let file = |rate: &str| {
            format!(
                "<Cube xmlns='http://www.ecb.int/vocabulary/2002-08-01/eurofxref'><Cube currency='USD' rate='1.1'/><Cube currency='XYZ' rate='{rate}'/></Cube>"
            )
        };
        for rate in ["NaN", "inf", "-inf", "0", "-2.5", "abc"] {
            let rates = parse_ecb(&file(rate)).expect("the good rate remains");
            assert_eq!(rates.per_euro.get("XYZ"), None, "{rate}");
            assert_eq!(rates.per_euro.get("USD"), Some(&1.1), "{rate}");
        }
        let rates = parse_ecb(&file("2.5")).expect("rates");
        assert_eq!(rates.per_euro.get("XYZ"), Some(&2.5));
    }
}
