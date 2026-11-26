//! The calendar names blob of one language: weekday and month names, the patterns of `E`, `M`, `YM`, `MDE`, and the week data (first weekday).
//!
//! Pattern: Builder over a Decorator (ICU4X's `ExportDriver` writes the blob; `Filled` adds, for the exporter, the root entries that carry an
//! attribute, so that a formatter finds them under the language). One blob per language and ONE for every region of it: `de` serves `de-AT`, `de-CH`;
//! `en` serves `en-US` (week starts on Sunday), `en-GB` (Monday). A locale of another language is not in it (the host asks with its own language).
//! The reader is the formatter of ICU4X: `FixedCalendarDateTimeFormatter::<Gregorian, E>::try_new_with_buffer_provider(&BlobDataProvider, ...)`
//! and `WeekInformation::try_new_with_buffer_provider`. Why the decorator: the exporter does not copy root entries with an attribute (the patterns
//! of `E` and `M`, the digits) down to a language, and a blob has no run-time fallback, so without it `E` and `M` fail with `IdentifierNotFound`.

use icu_locale::{Locale, LocaleFallbacker};
use icu_provider::export::{ExportMarker, ExportableProvider};
use icu_provider::prelude::*;
use icu_provider::{DataLocale, IterableDynamicDataProvider};
use icu_provider_export::blob_exporter::BlobExporter;
use icu_provider_export::prelude::{DataLocaleFamily, DeduplicationStrategy, ExportDriver};
use std::collections::BTreeSet;

use crate::error::PackError;
use crate::names_source::Source;

/// The source with the root entries that carry an attribute also under `locale`.
struct Filled {
    locale: DataLocale,
}

impl DynamicDataProvider<ExportMarker> for Filled {
    fn load_data(
        &self,
        marker: DataMarkerInfo,
        req: DataRequest<'_>,
    ) -> Result<DataResponse<ExportMarker>, DataError> {
        DynamicDataProvider::<ExportMarker>::load_data(&Source, marker, req)
    }
}

impl IterableDynamicDataProvider<ExportMarker> for Filled {
    fn iter_ids_for_marker(
        &self,
        marker: DataMarkerInfo,
    ) -> Result<BTreeSet<DataIdentifierCow<'_>>, DataError> {
        let mut ids = Source.iter_ids_for_marker(marker)?;
        let copies: Vec<_> = ids
            .iter()
            .filter(|id| id.locale.is_unknown() && !id.marker_attributes.is_empty())
            .map(|id| {
                let mut copy = id.clone();
                copy.locale = self.locale;
                copy
            })
            .collect();
        ids.extend(copies);
        Ok(ids)
    }
}

impl ExportableProvider for Filled {
    fn supported_markers(&self) -> BTreeSet<DataMarkerInfo> {
        Source.supported_markers()
    }
}

/// The blob of `tag` (a language, e.g. `de`): the language, its regions and the root.
pub(crate) fn blob(tag: &str) -> Result<Vec<u8>, PackError> {
    let fail = |why: String| PackError::Library(format!("{tag}: calendar names: {why}"));
    let locale: Locale = tag.parse().map_err(|e| fail(format!("{e:?}")))?;
    let data_locale = DataLocale::from(&locale);
    let mut families = vec![DataLocaleFamily::with_descendants(data_locale)];
    let regions = if locale.id.region.is_some() {
        BTreeSet::new()
    } else {
        regions().map_err(|e| fail(format!("{e:?}")))?
    };
    for region in regions {
        let mut own = locale.clone();
        own.id.region = Some(region);
        families.push(DataLocaleFamily::single(DataLocale::from(&own)));
    }
    let mut out: Vec<u8> = Vec::new();
    ExportDriver::new(
        families,
        DeduplicationStrategy::None.into(),
        LocaleFallbacker::new().static_to_owned(),
    )
    .export(
        &Filled {
            locale: data_locale,
        },
        BlobExporter::new_with_sink(Box::new(&mut out)),
    )
    .map_err(|e| fail(format!("{e:?}")))?;
    if out.is_empty() {
        return Err(fail("empty blob".to_owned()));
    }
    Ok(out)
}

/// Where the blob of `tag` lies below the output folder.
pub(crate) fn path(tag: &str) -> String {
    format!("dates/{tag}/names.blob")
}

/// Every region CLDR knows data for (any language, any of the markers). A blob has no run-time fallback, so `en-US` and `en-GB` must be keys of their
/// own to be found, and the week of `en-GB` (Monday) differs from the one of `en` (the United States: Sunday); the host asks with the full locale
/// of the person, then with the language alone (only a region that CLDR has no data for is not a key).
fn regions() -> Result<BTreeSet<icu_locale::subtags::Region>, DataError> {
    let mut out = BTreeSet::new();
    for marker in Source.supported_markers() {
        let ids = Source.iter_ids_for_marker(marker)?;
        out.extend(ids.iter().filter_map(|id| id.locale.region));
    }
    Ok(out)
}
