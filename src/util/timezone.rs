use chrono::Datelike;
use chrono_tz::Tz;
use sqlx::types::chrono::{DateTime, TimeZone, Utc};
use time::{Date, Month, OffsetDateTime, UtcOffset};

pub fn localized_datetime(time: OffsetDateTime, tz: Tz) -> DateTime<Tz> {
    let utc = time.to_offset(UtcOffset::UTC);
    let seconds = utc.unix_timestamp();
    let nanos: u32 = utc.nanosecond();
    let datetime_utc = DateTime::<Utc>::from_timestamp(seconds, nanos).unwrap_or_else(|| {
        DateTime::<Utc>::from_timestamp(seconds, 0).expect("valid UTC timestamp")
    });
    tz.from_utc_datetime(&datetime_utc.naive_utc())
}

pub fn localized_date(time: OffsetDateTime, tz: Tz) -> Date {
    let localized = localized_datetime(time, tz);
    let month = Month::try_from(localized.month() as u8)
        .expect("valid month value from chrono to time conversion");
    let day =
        u8::try_from(localized.day()).expect("valid day value from chrono to time conversion");
    Date::from_calendar_date(localized.year(), month, day).expect("valid calendar date")
}
