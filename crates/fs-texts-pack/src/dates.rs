//! The date data blob of one language, separate from the texts (decision 9f: ONE package per language for ALL programs).
//!
//! Pattern: Builder (the ICU4X data of one language, cut to what the date functions ask for, as one postcard blob). Only Gregorian, no time zones,
//! every date and time form (`DateNeeds::all`, 108 forms, about 1.4 KB per language). Other calendars or zone names would make the blob much larger
//! (mf2-fn-datetime icu.rs: about 85 percent); a language that needs them gets a wider spec here, not another tool. The bytes come from
//! `mf2_locale_data::icu_blob::icu_blob`, which is deterministic.

use mf2_locale_data::icu_blob::{icu_blob, DateNeeds, IcuBlobSpec};

use crate::error::PackError;

/// The server-data table of `tag` (`Catalog::with_server_data` takes it as it is): the LOCALE layout of `mf2-catalog` with the one `icu.blob` entry (key 48).
/// Written instead of the raw blob so that the loader needs no `writer` feature.
pub(crate) fn table(tag: &str) -> Result<Vec<u8>, PackError> {
    let spec = IcuBlobSpec::new(false, false, DateNeeds::all());
    let bytes =
        icu_blob(tag, &spec).map_err(|e| PackError::Library(format!("{tag}: date data: {e:?}")))?;
    if bytes.is_empty() {
        return Err(PackError::Library(format!("{tag}: date data: empty blob")));
    }
    mf2_catalog::writer::server_table(&[(mf2_catalog::format::locale_key::ICU_BLOB, bytes)])
        .map_err(|e| PackError::Library(format!("{tag}: date table: {e:?}")))
}

/// Where the table of `tag` lies below the output folder.
pub(crate) fn path(tag: &str) -> String {
    format!("dates/{tag}/icu.table")
}
