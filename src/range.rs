use chrono::{Duration, Local, NaiveDate};

/// Inputs that select a date range; mirrors the CLI flags.
#[derive(Debug, Default, Clone)]
pub struct RangeSpec {
    pub yesterday: bool,
    pub week: bool,
    pub date: Option<NaiveDate>,
    pub since: Option<NaiveDate>,
    pub until: Option<NaiveDate>,
}

/// Returns `(start, end, label)`. Same precedence as the Python script:
/// since/until > week > date > yesterday > today.
pub fn resolve(spec: &RangeSpec) -> Result<(NaiveDate, NaiveDate, String), String> {
    let today = Local::now().date_naive();

    if spec.since.is_some() || spec.until.is_some() {
        let end = spec.until.unwrap_or(today);
        let start = spec.since.or(spec.until).unwrap();
        if start > end {
            return Err("--since must not be after --until".into());
        }
        return Ok((start, end, format!("{} → {}", start.format("%d/%m/%Y"), end.format("%d/%m/%Y"))));
    }
    if spec.week {
        return Ok((today - Duration::days(6), today, "nos últimos 7 dias".into()));
    }
    if let Some(d) = spec.date {
        return Ok((d, d, format!("em {}", d.format("%d/%m/%Y"))));
    }
    if spec.yesterday {
        let d = today - Duration::days(1);
        return Ok((d, d, format!("ontem ({})", d.format("%d/%m/%Y"))));
    }
    Ok((today, today, format!("hoje ({})", today.format("%d/%m/%Y"))))
}
