use chrono::{Local, NaiveDate};

use crate::cli::arguments::PeriodArgs;
use crate::config::{AppPaths, Settings, require_settings};
use crate::core::activity::{DateRange, RangeSpec, parse_date, parse_range, resolve_range};
use crate::database::Database;
use crate::database::repositories::{self, RepositoryRecord};
use crate::errors::Result;
use crate::output::colors::Theme;

pub struct Context {
    pub paths: AppPaths,
    pub settings: Settings,
    pub database: Database,
    pub theme: Theme,
}

impl Context {
    pub fn open() -> Result<Self> {
        let paths = AppPaths::resolve()?;
        let settings = require_settings(&paths)?;
        let database = Database::open(&paths.database_path(&settings))?;
        let theme = Theme::detect(settings.display.show_colors);
        Ok(Self {
            paths,
            settings,
            database,
            theme,
        })
    }

    pub fn resolve_repository(&self, selector: Option<&str>) -> Result<Option<RepositoryRecord>> {
        selector
            .map(|value| repositories::find(self.database.connection(), value))
            .transpose()
    }

    pub fn include_merges(&self) -> bool {
        self.settings.analytics.include_merges
    }
}

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn resolve_period(
    args: &PeriodArgs,
    default_range: Option<&str>,
    today: NaiveDate,
) -> Result<(Option<DateRange>, String)> {
    if args.from.is_some() || args.to.is_some() {
        let start = parse_date(args.from.as_deref().unwrap_or("1970-01-01"))?;
        let end = match &args.to {
            Some(text) => parse_date(text)?,
            None => today,
        };
        let range = DateRange::new(start, end)?;
        return Ok((Some(range), format!("{start} to {end}")));
    }
    let label = args
        .range
        .clone()
        .or_else(|| default_range.map(str::to_owned))
        .unwrap_or_else(|| "all".to_owned());
    let spec = parse_range(&label)?;
    let range = resolve_range(spec, today)?;
    Ok((range, label))
}

pub fn heatmap_period(args: &PeriodArgs, today: NaiveDate) -> Result<(DateRange, String)> {
    if args.from.is_some() || args.to.is_some() || args.range.is_some() {
        let (range, label) = resolve_period(args, None, today)?;
        let range = match range {
            Some(range) => range,
            None => DateRange::new(today, today)?,
        };
        return Ok((range, label));
    }
    let range =
        resolve_range(RangeSpec::Days(365), today)?.unwrap_or(DateRange::new(today, today)?);
    Ok((range, "365d".to_owned()))
}
