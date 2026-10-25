//! The time of the machine the launcher runs on.
//!
//! The launcher shows the time in its log and in the names of the log files,
//! so it needs the local time of the machine. The time is read from the
//! operating system, so it also follows the daylight saving time.

use std::fmt;
use std::time::{SystemTime, UNIX_EPOCH};

/// A moment of the day, without the time zone.
///
/// The launcher shows the local time of the machine, so the time zone is not
/// part of the value.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct DateTime {
    /// The year.
    year: i32,
    /// The month, from one to twelve.
    month: u32,
    /// The day of the month, from one to thirty-one.
    day: u32,
    /// The hour, from zero to twenty-three.
    hour: u32,
    /// The minute, from zero to fifty-nine.
    minute: u32,
    /// The second, from zero to fifty-nine.
    second: u32,
}

impl DateTime {
    /// Creates a moment of the day.
    pub const fn new(year: i32, month: u32, day: u32, hour: u32, minute: u32, second: u32) -> Self {
        Self {
            year,
            month,
            day,
            hour,
            minute,
            second,
        }
    }

    /// Returns the current moment of the day, in the time zone of the
    /// machine.
    ///
    /// The time is read from the operating system; the time of the UTC zone
    /// is used when the operating system does not tell the time zone.
    pub fn now() -> Self {
        let seconds = unix_seconds();
        Self::from_unix_seconds(seconds + local_offset(seconds))
    }

    /// Returns the current moment of the day, in the time zone of the UTC.
    pub fn now_utc() -> Self {
        Self::from_unix_seconds(unix_seconds())
    }

    /// Returns the year.
    pub const fn year(self) -> i32 {
        self.year
    }

    /// Returns the month, from one to twelve.
    pub const fn month(self) -> u32 {
        self.month
    }

    /// Returns the day of the month, from one to thirty-one.
    pub const fn day(self) -> u32 {
        self.day
    }

    /// Returns the hour, from zero to twenty-three.
    pub const fn hour(self) -> u32 {
        self.hour
    }

    /// Returns the minute, from zero to fifty-nine.
    pub const fn minute(self) -> u32 {
        self.minute
    }

    /// Returns the second, from zero to fifty-nine.
    pub const fn second(self) -> u32 {
        self.second
    }

    /// Returns the name of the file of a log that was written at this
    /// moment, such as `2026-01-02T03-04-05`.
    ///
    /// The name holds no colon, so it can be used on every system.
    pub fn file_name(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}-{:02}-{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }

    /// Returns the number of the seconds since the first of January 1970.
    ///
    /// The moment is read in the zone of the UTC, so the number is the one of
    /// the moment the launcher shows as the UTC date.
    pub fn to_unix_seconds(self) -> i64 {
        let days = days_from_civil(self.year, self.month, self.day);
        days * 86_400
            + i64::from(self.hour) * 3_600
            + i64::from(self.minute) * 60
            + i64::from(self.second)
    }

    /// Creates a moment of the day from the number of seconds since the
    /// first of January 1970.
    fn from_unix_seconds(seconds: i64) -> Self {
        let days = seconds.div_euclid(86_400);
        let time_of_day = seconds.rem_euclid(86_400);

        let (year, month, day) = civil_from_days(days);

        Self {
            year,
            month,
            day,
            hour: (time_of_day / 3_600) as u32,
            minute: ((time_of_day % 3_600) / 60) as u32,
            second: (time_of_day % 60) as u32,
        }
    }
}

impl fmt::Display for DateTime {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02}",
            self.year, self.month, self.day, self.hour, self.minute, self.second
        )
    }
}

/// Returns the number of the milliseconds since the first of January 1970.
///
/// The launcher keeps the moments of the files it caches in milliseconds,
/// like the Java launcher, so the two launchers read the same index.
pub fn unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default()
}

/// Reads the date of the headers of the servers, such as
/// `Sun, 06 Nov 1994 08:49:37 GMT`.
///
/// The servers write the dates in the format of the RFC 1123; the other two
/// formats the protocol allows, the one of the RFC 850 and the one of the
/// asctime of C, are read as well, because the servers of the mirrors write
/// them.
pub fn parse_http_date(text: &str) -> Option<i64> {
    let text = text.trim();
    if text.is_empty() {
        return None;
    }

    // The name of the day of the week may be there, followed by a comma, and
    // the date of the RFC 850 names the day, the month and the year with
    // dashes; both are split apart, so the parts are read the same way.
    let tokens: Vec<&str> = text
        .split(|character: char| character.is_whitespace() || character == ',' || character == '-')
        .filter(|token| !token.is_empty())
        .collect();

    match tokens.as_slice() {
        // The format of the RFC 1123: the day of the week, the day, the month,
        // the year, the time and the zone.
        [_, day, month, year, time, zone] => parse_parts(day, month, year, time, zone),
        // The format of the RFC 1123 without the name of the day of the week.
        [day, month, year, time, zone] if is_zone(zone) => {
            parse_parts(day, month, year, time, zone)
        }
        // The format of the asctime of C: `Sun Nov  6 08:49:37 1994`; it has
        // no zone, because every date of the protocol is in the zone of the
        // UTC.
        [_day_name, month, day, time, year] => parse_parts(day, month, year, time, "GMT"),
        _ => None,
    }
}

/// Returns whether the text names one of the zones of the dates of the
/// protocol.
fn is_zone(text: &str) -> bool {
    text.eq_ignore_ascii_case("GMT") || text.eq_ignore_ascii_case("UTC")
}

/// Reads the parts of a date of the headers.
fn parse_parts(day: &str, month: &str, year: &str, time: &str, zone: &str) -> Option<i64> {
    // The time is read in the zone of the UTC; the date of the other zones is
    // rewritten, because the servers send their dates in the zone of the UTC.
    if !zone.eq_ignore_ascii_case("GMT") && !zone.eq_ignore_ascii_case("UTC") {
        return None;
    }

    let day = day.parse::<u32>().ok()?;
    let month = month_number(month)?;
    let year = year_number(year)?;

    let mut parts = time.split(':');
    let hour = parts.next()?.parse::<u32>().ok()?;
    let minute = parts.next()?.parse::<u32>().ok()?;
    let second = parts.next()?.parse::<u32>().ok()?;
    if parts.next().is_some() {
        return None;
    }
    if !(1..=31).contains(&day) || hour > 23 || minute > 59 || second > 60 {
        return None;
    }

    Some(DateTime::new(year, month, day, hour, minute, second).to_unix_seconds())
}

/// Returns the number of the year, with the year of two digits of the RFC 850
/// made a year of the twenty-first century.
fn year_number(text: &str) -> Option<i32> {
    match text.len() {
        4 => text.parse().ok(),
        // The format of the RFC 850 writes the year with two digits; the dates
        // of the servers are recent, so the year is taken to be of this
        // century.
        2 => text
            .parse::<i32>()
            .ok()
            .map(|year| if year < 70 { 2000 + year } else { 1900 + year }),
        _ => None,
    }
}

/// Returns the number of the month of its name.
fn month_number(name: &str) -> Option<u32> {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];

    MONTHS
        .iter()
        .position(|month| month.eq_ignore_ascii_case(name))
        .map(|index| index as u32 + 1)
}

/// Returns the number of the seconds since the first of January 1970.
fn unix_seconds() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs() as i64)
        .unwrap_or_default()
}

/// Returns the year, the month and the day of a number of days since the
/// first of January 1970.
///
/// The calculation is the one of Howard Hinnant, which works for every day
/// of the calendar of the Gregorian.
fn civil_from_days(days: i64) -> (i32, u32, u32) {
    let days = days + 719_468;
    let era = days.div_euclid(146_097);
    let day_of_era = days.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = (day_of_year - (153 * month_prime + 2) / 5 + 1) as u32;
    let month = if month_prime < 10 {
        month_prime + 3
    } else {
        month_prime - 9
    } as u32;

    let year = if month <= 2 { year + 1 } else { year };
    (year as i32, month, day)
}

/// Returns the number of days since the first of January 1970 of a date.
///
/// The calculation is the reverse of [`civil_from_days`], so the two convert
/// the dates the same way.
fn days_from_civil(year: i32, month: u32, day: u32) -> i64 {
    let year = i64::from(year) - i64::from(month <= 2);
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month = i64::from(month);
    let day_of_year =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + i64::from(day) - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

    era * 146_097 + day_of_era - 719_468
}

/// Returns the difference between the local time and the time of the UTC
/// zone, in seconds.
#[cfg(unix)]
fn local_offset(seconds: i64) -> i64 {
    use std::mem::MaybeUninit;

    let mut result: libc::tm = unsafe { MaybeUninit::zeroed().assume_init() };
    let time = seconds as libc::time_t;

    // The call is safe, because the result is a structure of a known size
    // and the time is a valid number of seconds.
    let filled = unsafe { libc::localtime_r(&time, &mut result) };
    if filled.is_null() {
        return 0;
    }

    let local = result.tm_hour as i64 * 3_600 + result.tm_min as i64 * 60 + result.tm_sec as i64;
    let utc = seconds.rem_euclid(86_400);

    local - utc
}

/// Returns the difference between the local time and the time of the UTC
/// zone, in seconds.
#[cfg(windows)]
fn local_offset(seconds: i64) -> i64 {
    use std::mem::MaybeUninit;

    #[repr(C)]
    struct SystemTime {
        year: u16,
        month: u16,
        day_of_week: u16,
        day: u16,
        hour: u16,
        minute: u16,
        second: u16,
        milliseconds: u16,
    }

    #[link(name = "kernel32")]
    extern "system" {
        fn SystemTimeToTzSpecificLocalTime(
            time_zone: *const std::ffi::c_void,
            utc: *const SystemTime,
            local: *mut SystemTime,
        ) -> i32;
    }

    let utc = DateTime::from_unix_seconds(seconds);

    let utc_time = SystemTime {
        year: utc.year() as u16,
        month: utc.month() as u16,
        day_of_week: 0,
        day: utc.day() as u16,
        hour: utc.hour() as u16,
        minute: utc.minute() as u16,
        second: utc.second() as u16,
        milliseconds: 0,
    };
    let mut local: MaybeUninit<SystemTime> = MaybeUninit::zeroed();

    // The call is safe, because the structures are filled in and the time
    // zone is the one of the machine.
    let filled =
        unsafe { SystemTimeToTzSpecificLocalTime(std::ptr::null(), &utc_time, local.as_mut_ptr()) };
    if filled == 0 {
        return 0;
    }

    let local = unsafe { local.assume_init() };
    let local_seconds = local.hour as i64 * 3_600 + local.minute as i64 * 60 + local.second as i64;
    let utc_seconds = utc.hour() as i64 * 3_600 + utc.minute() as i64 * 60 + utc.second() as i64;

    local_seconds - utc_seconds
}

/// Returns the difference between the local time and the time of the UTC
/// zone, in seconds.
#[cfg(not(any(unix, windows)))]
fn local_offset(_seconds: i64) -> i64 {
    0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_moments_of_the_day_are_read_from_the_number_of_seconds() {
        // The first of January 1970, at midnight.
        let moment = DateTime::from_unix_seconds(0);
        assert_eq!(moment, DateTime::new(1970, 1, 1, 0, 0, 0));

        // The first of January 2026, at midnight.
        let moment = DateTime::from_unix_seconds(1_767_225_600);
        assert_eq!(moment, DateTime::new(2026, 1, 1, 0, 0, 0));

        // The first of March 2026, at midnight.
        let moment = DateTime::from_unix_seconds(1_772_323_200);
        assert_eq!(moment, DateTime::new(2026, 3, 1, 0, 0, 0));

        // The first of March 2026, at nine twenty in the morning.
        let moment = DateTime::from_unix_seconds(1_772_356_800);
        assert_eq!(moment, DateTime::new(2026, 3, 1, 9, 20, 0));
    }

    #[test]
    fn the_moments_of_the_day_are_written() {
        let moment = DateTime::new(2026, 1, 2, 3, 4, 5);
        assert_eq!(moment.to_string(), "2026-01-02 03:04:05");
        assert_eq!(moment.file_name(), "2026-01-02T03-04-05");
    }

    #[test]
    fn the_current_moment_of_the_day_is_read() {
        let moment = DateTime::now();
        assert!(moment.year() >= 2024);
        assert!((1..=12).contains(&moment.month()));
        assert!((1..=31).contains(&moment.day()));
        assert!(moment.hour() < 24);
        assert!(moment.minute() < 60);
        assert!(moment.second() < 60);

        let utc = DateTime::now_utc();
        assert!(utc.year() >= 2024);
    }

    #[test]
    fn the_moments_of_the_day_are_counted_from_the_first_of_january_1970() {
        assert_eq!(DateTime::new(1970, 1, 1, 0, 0, 0).to_unix_seconds(), 0);
        assert_eq!(
            DateTime::new(2026, 1, 1, 0, 0, 0).to_unix_seconds(),
            1_767_225_600
        );
        assert_eq!(
            DateTime::new(2026, 3, 1, 9, 20, 0).to_unix_seconds(),
            1_772_356_800
        );

        // A date the calendar of the Gregorian has a leap day in.
        assert_eq!(
            DateTime::new(2024, 2, 29, 12, 0, 0).to_unix_seconds(),
            1_709_208_000
        );

        // The two conversions are the reverse of each other.
        for seconds in [0, 951_782_400, 1_767_225_600, 1_772_356_800] {
            assert_eq!(
                DateTime::from_unix_seconds(seconds).to_unix_seconds(),
                seconds
            );
        }
    }

    #[test]
    fn the_dates_of_the_headers_are_read() {
        let expected = 784_111_777;
        assert_eq!(
            parse_http_date("Sun, 06 Nov 1994 08:49:37 GMT"),
            Some(expected)
        );
        // The format of the RFC 850, with a year of two digits.
        assert_eq!(
            parse_http_date("Sunday, 06-Nov-94 08:49:37 GMT"),
            Some(expected)
        );
        // The format of the asctime of C.
        assert_eq!(parse_http_date("Sun Nov  6 08:49:37 1994"), Some(expected));
        // The date is the same in every case.
        assert_eq!(
            parse_http_date("Sun, 06 nov 1994 08:49:37 gmt"),
            Some(expected)
        );

        assert_eq!(parse_http_date(""), None);
        assert_eq!(parse_http_date("not a date"), None);
        assert_eq!(parse_http_date("Sun, 06 Nov 1994 08:49:37 PST"), None);
        assert_eq!(parse_http_date("Sun, 32 Nov 1994 08:49:37 GMT"), None);
        assert_eq!(parse_http_date("Sun, 06 Foo 1994 08:49:37 GMT"), None);
    }

    #[test]
    fn the_time_zone_of_the_machine_is_used() {
        let seconds = unix_seconds();
        let offset = local_offset(seconds);

        // The difference between the local time and the time of the UTC zone
        // is less than a day.
        assert!(offset.abs() < 86_400);
        // The difference is a whole number of minutes, because the time
        // zones of the world are a whole number of minutes apart.
        assert_eq!(offset % 60, 0);
    }
}
