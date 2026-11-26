//! Pattern: none (test). The calendar names blob (`dates/<language>/names.blob`) is read the way a host reads it: with the formatters of ICU4X.

mod support;

use icu_calendar::week::{WeekInformation, WeekPreferences};
use icu_calendar::{Date, Gregorian};
use icu_datetime::{fieldsets, FixedCalendarDateTimeFormatter};
use icu_locale::Locale;
use icu_provider_blob::BlobDataProvider;

fn provider(tag: &str) -> BlobDataProvider {
    let files = fs_texts_pack::pack(&support::data(), &[]).unwrap().files;
    let blob = files
        .get(&format!("dates/{tag}/names.blob"))
        .unwrap_or_else(|| panic!("no names.blob for {tag}"));
    BlobDataProvider::try_new_from_blob(blob.clone().into_boxed_slice()).expect("blob loads")
}

fn day(d: u8) -> Date<Gregorian> {
    Date::try_new_gregorian(2026, 11, d).unwrap()
}

macro_rules! say {
    ($p:expr, $loc:expr, $fs:expr, $date:expr) => {
        FixedCalendarDateTimeFormatter::<Gregorian, _>::try_new_with_buffer_provider(
            $p,
            (&$loc).into(),
            $fs,
        )
        .unwrap_or_else(|e| panic!("{:?}: {e:?}", $loc))
        .format(&$date)
        .to_string()
    };
}

/// The rule of the host: the person's full locale first, then the language alone (a region without a week of its own is not a key).
fn first(p: &BlobDataProvider, tag: &str) -> icu_calendar::types::Weekday {
    let mut loc: Locale = tag.parse().unwrap();
    let ext = loc.extensions.clone();
    let week =
        |l: &Locale| WeekInformation::try_new_with_buffer_provider(p, WeekPreferences::from(l));
    let found = week(&loc).or_else(|_| {
        loc.id.region = None;
        loc.extensions = ext;
        week(&loc)
    });
    found
        .unwrap_or_else(|e| panic!("{tag}: {e:?}"))
        .first_weekday
}

#[test]
fn de_has_the_month_the_title_and_the_weekdays() {
    let (p, de) = (provider("de"), "de".parse::<Locale>().unwrap());
    assert_eq!(say!(&p, de, fieldsets::M::long(), day(26)), "November");
    assert_eq!(
        say!(&p, de, fieldsets::YM::long(), day(26)),
        "November 2026"
    );
    let names: Vec<_> = (23..=29)
        .map(|d| say!(&p, de, fieldsets::E::long(), day(d)))
        .collect();
    assert_eq!(
        names,
        [
            "Montag",
            "Dienstag",
            "Mittwoch",
            "Donnerstag",
            "Freitag",
            "Samstag",
            "Sonntag"
        ]
    );
    assert_eq!(say!(&p, de, fieldsets::E::short(), day(23)), "Mo");
    assert_eq!(
        say!(&p, de, fieldsets::MDE::medium(), day(26)),
        "Do., 26. Nov."
    );
    assert_eq!(
        say!(&p, de, fieldsets::MDE::long(), day(26)),
        "Donnerstag, 26. November"
    );
    assert_eq!(
        say!(&p, de, fieldsets::MDE::short(), day(26)),
        "Do., 26.11."
    );
}

#[test]
fn the_week_starts_where_cldr_says() {
    use icu_calendar::types::Weekday::{Monday, Sunday};
    let (de, en) = (provider("de"), provider("en"));
    assert_eq!(first(&de, "de"), Monday);
    assert_eq!(first(&de, "de-AT"), Monday);
    assert_eq!(first(&en, "en-US"), Sunday, "United States: Sunday");
    assert_eq!(first(&en, "en-GB"), Monday);
    // The person's own choice is a BCP 47 extension and wins over the region.
    assert_eq!(first(&de, "de-u-fw-sun"), Sunday);
}

#[test]
fn en_us_reads_the_english_blob() {
    let (p, us) = (provider("en"), "en-US".parse::<Locale>().unwrap());
    assert_eq!(say!(&p, us, fieldsets::M::long(), day(26)), "November");
    assert_eq!(
        say!(&p, us, fieldsets::YM::long(), day(26)),
        "November 2026"
    );
    assert_eq!(say!(&p, us, fieldsets::E::long(), day(26)), "Thursday");
    assert_eq!(say!(&p, us, fieldsets::E::short(), day(26)), "Thu");
    assert_eq!(
        say!(&p, us, fieldsets::MDE::medium(), day(26)),
        "Thu, Nov 26"
    );
}

#[test]
fn every_length_of_every_field_set_loads() {
    for tag in ["de", "en"] {
        let (p, loc) = (provider(tag), tag.parse::<Locale>().unwrap());
        for d in [day(1), day(26)] {
            let all = [
                say!(&p, loc, fieldsets::E::long(), d),
                say!(&p, loc, fieldsets::E::medium(), d),
                say!(&p, loc, fieldsets::E::short(), d),
                say!(&p, loc, fieldsets::M::long(), d),
                say!(&p, loc, fieldsets::M::medium(), d),
                say!(&p, loc, fieldsets::M::short(), d),
                say!(&p, loc, fieldsets::YM::long(), d),
                say!(&p, loc, fieldsets::YM::medium(), d),
                say!(&p, loc, fieldsets::YM::short(), d),
                say!(&p, loc, fieldsets::MDE::long(), d),
                say!(&p, loc, fieldsets::MDE::medium(), d),
                say!(&p, loc, fieldsets::MDE::short(), d),
            ];
            assert!(all.iter().all(|s| !s.is_empty()), "{tag}: {all:?}");
        }
    }
}

#[test]
fn the_size_of_a_language_is_named_and_bounded() {
    let files = fs_texts_pack::pack(&support::data(), &[]).unwrap().files;
    for tag in ["de", "en"] {
        let n = files[&format!("dates/{tag}/names.blob")].len();
        eprintln!("names.blob {tag}: {n} bytes");
        assert!((2_000..120_000).contains(&n), "{tag}: {n} bytes");
    }
}
