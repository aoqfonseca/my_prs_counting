use std::io::ErrorKind;
use std::process::{Command, Stdio};

use chrono::{Duration, NaiveDate};
use serde::Deserialize;

use crate::{Pr, SourceResult, in_range, parse_ts};

const NAME: &str = "GitHub";

#[derive(Deserialize)]
struct Item {
    number: u64,
    title: String,
    repo: String,
    merged_at: String,
    url: String,
    labels: Vec<String>,
}

/// Fetch merged PRs through the `gh` CLI (reuses the user's existing auth).
pub fn fetch(user: &str, start: NaiveDate, end: NaiveDate) -> SourceResult {
    // `merged:` qualifier is UTC-based; pad by a day and filter on local date afterwards.
    let lo = start - Duration::days(1);
    let hi = end + Duration::days(1);
    let query = format!("is:pr is:merged author:{user} merged:{lo}..{hi}");
    let jq = r#".items[] | {number, title, repo: (.repository_url | split("/")[-2:] | join("/")), merged_at: .pull_request.merged_at, url: .html_url, labels: [.labels[].name]}"#;

    let output = match Command::new("gh").stdin(Stdio::null())
        .args(["api", "--paginate", "-X", "GET", "search/issues"])
        .args(["-f", &format!("q={query}"), "-f", "per_page=100", "--jq", jq])
        .output()
    {
        Ok(o) => o,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return SourceResult::err(NAME, vec![], "`gh` CLI not found");
        }
        Err(e) => return SourceResult::err(NAME, vec![], e.to_string()),
    };

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
        return SourceResult::err(NAME, vec![], if stderr.is_empty() { "gh failed".into() } else { stderr });
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut prs = Vec::new();
    for line in stdout.lines().filter(|l| !l.trim().is_empty()) {
        let item: Item = match serde_json::from_str(line) {
            Ok(i) => i,
            Err(e) => return SourceResult::err(NAME, prs, format!("bad gh output: {e}")),
        };
        let Ok(merged) = parse_ts(&item.merged_at) else { continue };
        if in_range(merged, start, end) {
            prs.push(Pr {
                source: NAME.into(),
                repo: item.repo,
                number: item.number,
                title: item.title,
                merged_at: merged,
                url: item.url,
                labels: item.labels,
                bot: false,
            });
        }
    }
    SourceResult::ok(NAME, prs)
}
