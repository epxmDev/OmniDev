use chrono::{DateTime, Days, Months, NaiveDate, TimeDelta, Utc};

use crate::errors::{OmniDevError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateRange {
    pub start: NaiveDate,
    pub end: NaiveDate,
}

impl DateRange {
    pub fn new(start: NaiveDate, end: NaiveDate) -> Result<Self> {
        if start > end {
            return Err(OmniDevError::InvalidInput(format!(
                "start date {start} is after end date {end}"
            )));
        }
        Ok(Self { start, end })
    }

    pub fn contains(&self, day: NaiveDate) -> bool {
        day >= self.start && day <= self.end
    }

    pub fn day_count(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RangeSpec {
    Days(u32),
    Weeks(u32),
    Months(u32),
    Years(u32),
    All,
}

pub fn parse_range(input: &str) -> Result<RangeSpec> {
    let normalized = input.trim().to_ascii_lowercase();
    if normalized == "all" {
        return Ok(RangeSpec::All);
    }
    let split = normalized
        .find(|character: char| !character.is_ascii_digit())
        .unwrap_or(normalized.len());
    let (digits, unit) = normalized.split_at(split);
    let amount: u32 = digits
        .parse()
        .map_err(|_| OmniDevError::InvalidInput(format!("invalid range '{input}'")))?;
    if amount == 0 {
        return Err(OmniDevError::InvalidInput(format!(
            "range '{input}' must be greater than zero"
        )));
    }
    match unit {
        "d" => Ok(RangeSpec::Days(amount)),
        "w" => Ok(RangeSpec::Weeks(amount)),
        "m" => Ok(RangeSpec::Months(amount)),
        "y" => Ok(RangeSpec::Years(amount)),
        _ => Err(OmniDevError::InvalidInput(format!(
            "invalid range '{input}'; use a number followed by d, w, m or y, or 'all'"
        ))),
    }
}

pub fn resolve_range(spec: RangeSpec, today: NaiveDate) -> Result<Option<DateRange>> {
    let overflow = || OmniDevError::InvalidInput("range is too large".to_owned());
    let start = match spec {
        RangeSpec::All => return Ok(None),
        RangeSpec::Days(days) => today
            .checked_sub_days(Days::new(u64::from(days) - 1))
            .ok_or_else(overflow)?,
        RangeSpec::Weeks(weeks) => today
            .checked_sub_days(Days::new(u64::from(weeks) * 7 - 1))
            .ok_or_else(overflow)?,
        RangeSpec::Months(months) => today
            .checked_sub_months(Months::new(months))
            .and_then(|date| date.checked_add_days(Days::new(1)))
            .ok_or_else(overflow)?,
        RangeSpec::Years(years) => today
            .checked_sub_months(Months::new(years * 12))
            .and_then(|date| date.checked_add_days(Days::new(1)))
            .ok_or_else(overflow)?,
    };
    DateRange::new(start, today).map(Some)
}

pub fn parse_date(input: &str) -> Result<NaiveDate> {
    NaiveDate::parse_from_str(input.trim(), "%Y-%m-%d").map_err(|_| {
        OmniDevError::InvalidInput(format!("invalid date '{input}'; expected YYYY-MM-DD"))
    })
}

pub fn local_day(committed_at: i64, utc_offset_minutes: i32) -> Result<NaiveDate> {
    let instant = DateTime::<Utc>::from_timestamp(committed_at, 0).ok_or_else(|| {
        OmniDevError::InvalidInput(format!("invalid commit timestamp {committed_at}"))
    })?;
    let shifted = instant
        .checked_add_signed(TimeDelta::minutes(i64::from(utc_offset_minutes)))
        .ok_or_else(|| {
            OmniDevError::InvalidInput(format!("commit timestamp {committed_at} is out of range"))
        })?;
    Ok(shifted.date_naive())
}

pub fn format_timestamp(instant: DateTime<Utc>) -> String {
    instant.format("%Y-%m-%dT%H:%M:%SZ").to_string()
}

pub fn format_day(day: NaiveDate) -> String {
    day.format("%Y-%m-%d").to_string()
}
