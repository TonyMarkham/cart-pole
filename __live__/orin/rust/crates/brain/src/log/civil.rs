use std::time::{SystemTime, UNIX_EPOCH};

/// The clock's divisions.
const SECONDS_PER_MINUTE: u64 = 60;
const SECONDS_PER_HOUR: u64 = 60 * SECONDS_PER_MINUTE;
const SECONDS_PER_DAY: u64 = 24 * SECONDS_PER_HOUR;

/// The calendar's divisions, for Howard Hinnant's `civil_from_days`. It counts
/// in **eras** of 400 years, after which the Gregorian calendar repeats
/// exactly: 400 × 365 days plus 97 leap days, 146,097 days. Inside an era it
/// shifts each year to start on 1 March, so that a leap day is the last day of
/// its year and never shifts the months after it.
const DAYS_PER_ERA: i64 = 146_097;
const YEARS_PER_ERA: i64 = 400;
/// From 0000-03-01, the start of the shifted calendar's era 0, to 1970-01-01.
const DAYS_FROM_ERA_0_TO_EPOCH: i64 = 719_468;
/// Four years are 1,461 days with their leap day; dividing by 1,460 and
/// subtracting takes the leap days out of a day count, as the algorithm does.
const DAYS_PER_FOUR_YEARS_LESS_ONE: i64 = 1_460;
/// A century inside an era: 100 years with 24 leap days.
const DAYS_PER_CENTURY: i64 = 36_524;
/// The last day of an era, which the century rule would otherwise count as the
/// first day of the next century.
const LAST_DAY_OF_ERA: i64 = DAYS_PER_ERA - 1;
const DAYS_PER_YEAR: i64 = 365;
const YEARS_PER_LEAP_CYCLE: i64 = 4;
const YEARS_PER_CENTURY: i64 = 100;
/// From 1 March, every five months hold 153 days (31 + 30 + 31 + 30 + 31), which
/// is what lets a month be found from a day of the year by one division:
/// `(5 × day + 2) / 153`.
const DAYS_PER_FIVE_MONTHS: i64 = 153;
const MONTHS_PER_FIVE_MONTHS: i64 = 5;
const MONTH_ROUNDING: i64 = 2;
/// Shifted months 0–9 are March to December; 10 and 11 are January and
/// February of the next calendar year.
const SHIFTED_MONTHS_IN_YEAR: i64 = 10;
const MARCH: i64 = 3;
const SHIFTED_TO_JANUARY: i64 = 9;
const FEBRUARY: u32 = 2;

/// A run folder's name from the moment of the Run press: `YYYYMMDD_HHMMSS`, the
/// trainer's form (`runs/20261101_163458`), in UTC. The Orin's clock keeps
/// UTC, and no library is needed for it: the days since 1970 are turned into a
/// calendar date by Howard Hinnant's `civil_from_days`, the arithmetic the C++
/// standard library uses.
pub(crate) fn folder_name(now: SystemTime) -> String {
    let seconds = now
        .duration_since(UNIX_EPOCH)
        .map_or(0, |since| since.as_secs());
    let days = seconds / SECONDS_PER_DAY;
    let in_day = seconds % SECONDS_PER_DAY;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}{month:02}{day:02}_{:02}{:02}{:02}",
        in_day / SECONDS_PER_HOUR,
        in_day % SECONDS_PER_HOUR / SECONDS_PER_MINUTE,
        in_day % SECONDS_PER_MINUTE
    )
}

/// The calendar date `days` after 1970-01-01 (Howard Hinnant, "chrono-Compatible
/// Low-Level Date Algorithms", `civil_from_days`), step by step: which era the
/// day is in and which day of that era; which year of the era, with the leap
/// days taken out; which day of that shifted year; which shifted month, by the
/// five-month rule; then the day of the month, and the shift undone.
fn civil_from_days(days: i64) -> (i64, u32, u32) {
    let z = days + DAYS_FROM_ERA_0_TO_EPOCH;
    let era = z.div_euclid(DAYS_PER_ERA);
    let day_of_era = z.rem_euclid(DAYS_PER_ERA);
    let year_of_era = (day_of_era - day_of_era / DAYS_PER_FOUR_YEARS_LESS_ONE
        + day_of_era / DAYS_PER_CENTURY
        - day_of_era / LAST_DAY_OF_ERA)
        / DAYS_PER_YEAR;
    let day_of_year = day_of_era
        - (DAYS_PER_YEAR * year_of_era + year_of_era / YEARS_PER_LEAP_CYCLE
            - year_of_era / YEARS_PER_CENTURY);
    let shifted_month =
        (MONTHS_PER_FIVE_MONTHS * day_of_year + MONTH_ROUNDING) / DAYS_PER_FIVE_MONTHS;
    let day = (day_of_year
        - (DAYS_PER_FIVE_MONTHS * shifted_month + MONTH_ROUNDING) / MONTHS_PER_FIVE_MONTHS
        + 1) as u32;
    let month = if shifted_month < SHIFTED_MONTHS_IN_YEAR {
        shifted_month + MARCH
    } else {
        shifted_month - SHIFTED_TO_JANUARY
    } as u32;
    let year = year_of_era + era * YEARS_PER_ERA + i64::from(month <= FEBRUARY);
    (year, month, day)
}
