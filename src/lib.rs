//! Core logic shared by the CLI (and, later, the TUI): data model, date ranges
//! and the GitHub / Forgejo fetchers.

pub mod forgejo;
pub mod github;
pub mod range;

use chrono::{DateTime, Local, NaiveDate};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct Pr {
    pub source: String,
    pub repo: String,
    pub number: u64,
    pub title: String,
    /// Timezone-aware, local tz.
    pub merged_at: DateTime<Local>,
    pub url: String,
    pub labels: Vec<String>,
    /// Carries the bot label (e.g. "evergreen").
    pub bot: bool,
}

#[derive(Debug, Clone)]
pub struct SourceResult {
    pub name: String,
    pub prs: Vec<Pr>,
    pub error: Option<String>,
}

impl SourceResult {
    pub fn ok(name: impl Into<String>, prs: Vec<Pr>) -> Self {
        Self { name: name.into(), prs, error: None }
    }

    pub fn err(name: impl Into<String>, prs: Vec<Pr>, error: impl Into<String>) -> Self {
        Self { name: name.into(), prs, error: Some(error.into()) }
    }
}

/// Parse an RFC 3339 timestamp (`Z` or offset) into the local timezone.
pub fn parse_ts(value: &str) -> anyhow::Result<DateTime<Local>> {
    Ok(DateTime::parse_from_rfc3339(value)?.with_timezone(&Local))
}

pub fn in_range(t: DateTime<Local>, start: NaiveDate, end: NaiveDate) -> bool {
    let d = t.date_naive();
    start <= d && d <= end
}

/// Mark PRs carrying `bot_label` (case-insensitive).
pub fn mark_bots(results: &mut [SourceResult], bot_label: &str) {
    let needle = bot_label.to_lowercase();
    for pr in results.iter_mut().flat_map(|r| r.prs.iter_mut()) {
        pr.bot = pr.labels.iter().any(|l| l.to_lowercase() == needle);
    }
}

/// Which sources to query and as whom.
#[derive(Debug, Clone)]
pub struct Config {
    pub github: bool,
    pub forgejo: bool,
    pub github_user: String,
    pub forgejo_host: String,
    pub forgejo_user: Option<String>,
    pub bot_label: String,
}

/// Fetch all enabled sources in parallel and flag bot PRs.
pub fn fetch_all(cfg: &Config, start: NaiveDate, end: NaiveDate) -> Vec<SourceResult> {
    let mut handles = Vec::new();
    if cfg.github {
        let user = cfg.github_user.clone();
        handles.push(std::thread::spawn(move || github::fetch(&user, start, end)));
    }
    if cfg.forgejo {
        let (host, user) = (cfg.forgejo_host.clone(), cfg.forgejo_user.clone());
        handles.push(std::thread::spawn(move || forgejo::fetch(&host, user.as_deref(), start, end)));
    }
    let mut results: Vec<SourceResult> = handles
        .into_iter()
        .map(|h| h.join().unwrap_or_else(|_| SourceResult::err("?", vec![], "fetch thread panicked")))
        .collect();
    mark_bots(&mut results, &cfg.bot_label);
    results
}
