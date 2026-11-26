//! The CLDR data of the calendar names, as an ICU4X export source.
//!
//! Pattern: Adapter (the baked CLDR 49 data of the `icu_*_data` crates behind ICU4X's `ExportableProvider`, which `ExportDriver` reads). The macros
//! are the ones ICU4X documents for baked data; only the Gregorian calendar, the weekday and month names, the year digits and the week data are
//! linked (no zones, no other calendars): what `E`, `M`, `YM`, `MDE` and `WeekInformation` ask for.

extern crate alloc;

use icu_provider::DataMarker as _;

/// ICU4X's compiled data, as a provider of the markers of the calendar names.
pub(crate) struct Source;

const _: () = {
    // The baked macros name these by their short paths.
    // The baked macros call each other by these names.
    use icu_calendar_data::__impl_calendar_week_v1;
    use icu_datetime_data::{
        __impl_datetime_names_month_gregorian_v1, __impl_datetime_names_weekday_v1,
        __impl_datetime_names_year_gregorian_v1, __impl_datetime_patterns_date_gregorian_v1,
    };
    use icu_decimal_data::{__impl_decimal_digits_v1, __impl_decimal_symbols_v1};
    use icu_locale::fallback as icu_locale_fallback;
    icu_datetime_data::make_provider!(Source);
    icu_datetime_data::impl_datetime_names_weekday_v1!(Source, ITER);
    icu_datetime_data::impl_datetime_names_month_gregorian_v1!(Source, ITER);
    icu_datetime_data::impl_datetime_names_year_gregorian_v1!(Source, ITER);
    icu_datetime_data::impl_datetime_patterns_date_gregorian_v1!(Source, ITER);
    icu_calendar_data::impl_calendar_week_v1!(Source, ITER);
    icu_decimal_data::impl_decimal_symbols_v1!(Source, ITER);
    icu_decimal_data::impl_decimal_digits_v1!(Source, ITER);
};

icu_provider::export::make_exportable_provider!(
    Source,
    [
        icu::datetime::provider::names::DatetimeNamesWeekdayV1,
        icu::datetime::provider::names::DatetimeNamesMonthGregorianV1,
        icu::datetime::provider::names::DatetimeNamesYearGregorianV1,
        icu::datetime::provider::semantic_skeletons::DatetimePatternsDateGregorianV1,
        icu::calendar::provider::CalendarWeekV1,
        icu::decimal::provider::DecimalSymbolsV1,
        icu::decimal::provider::DecimalDigitsV1,
    ]
);
