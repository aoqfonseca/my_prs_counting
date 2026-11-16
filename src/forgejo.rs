//! Forgejo source. Forgejo exposes a Gitea-compatible `/api/v1`, and credentials
//! are read from `FORGEJO_TOKEN` (or `GITEA_TOKEN`) / the `tea` login config.

use std::collections::HashMap;
use std::path::PathBuf;
use std::time::Duration as StdDuration;

use chrono::{Duration, Local, NaiveDate, TimeZone};
use serde::Deserialize;

use crate::{Pr, SourceResult, in_range, parse_ts};

fn tea_configs() -> Vec<PathBuf> {
    let Some(home) = std::env::var_os("HOME").map(PathBuf::from) else {
        return vec![];
    };
    vec![
        home.join("Library/Application Support/tea/config.yml"),
        home.join(".config/tea/config.yml"),
    ]
}

/// Minimal parse of tea's config.yml: returns the login block whose url matches `host`.
pub fn tea_login(host: &str) -> HashMap<String, String> {
    for path in tea_configs() {
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let mut blocks: Vec<HashMap<String, String>> = Vec::new();
        for raw in text.lines() {
            let mut line = raw.trim();
            if line.starts_with("- name:") {
                blocks.push(HashMap::new());
                line = &line[2..];
            }
            if let (Some(cur), Some((k, v))) = (blocks.last_mut(), line.split_once(": ")) {
                cur.insert(k.trim().to_string(), v.trim().trim_matches(['\'', '"']).to_string());
            }
        }
        if let Some(b) = blocks.into_iter().find(|b| b.get("url").is_some_and(|u| u.contains(host))) {
            return b;
        }
    }
    HashMap::new()
}

#[derive(Deserialize)]
struct Issue {
    number: u64,
    title: String,
    html_url: Option<String>,
    user: Option<Login>,
    repository: Option<Repo>,
    pull_request: Option<PullInfo>,
    labels: Option<Vec<Label>>,
}

#[derive(Deserialize)]
struct Login {
    login: Option<String>,
}

#[derive(Deserialize)]
struct Repo {
    full_name: Option<String>,
}

#[derive(Deserialize)]
struct Label {
    name: String,
}

#[derive(Deserialize)]
struct PullInfo {
    merged: Option<bool>,
    merged_at: Option<String>,
    html_url: Option<String>,
}

pub fn fetch(host: &str, user: Option<&str>, start: NaiveDate, end: NaiveDate) -> SourceResult {
    let name = host;
    let login = tea_login(host);
    let token = ["FORGEJO_TOKEN", "GITEA_TOKEN"].iter().find_map(|k| std::env::var(k).ok().filter(|t| !t.is_empty())).or_else(|| login.get("token").cloned());
    let user = user.map(str::to_string).or_else(|| login.get("user").cloned());
    let (Some(token), Some(user)) = (token, user) else {
        return SourceResult::err(name, vec![], "no token/user (set FORGEJO_TOKEN/--forgejo-user or run `tea login add`)");
    };

    let since = Local
        .from_local_datetime(&(start - Duration::days(1)).and_hms_opt(0, 0, 0).unwrap())
        .earliest()
        .map(|d| d.to_rfc3339())
        .unwrap_or_default();

    let agent: ureq::Agent = ureq::Agent::config_builder()
        .timeout_global(Some(StdDuration::from_secs(30)))
        .build()
        .into();

    // The server ignores `created_by` here, but `created=true` filters to the token owner.
    let owner = agent
        .get(&format!("https://{host}/api/v1/user"))
        .header("Authorization", &format!("token {token}"))
        .call()
        .ok()
        .and_then(|mut r| r.body_mut().read_json::<Login>().ok())
        .and_then(|l| l.login);
    let own_prs = owner.as_deref() == Some(user.as_str());

    let mut prs = Vec::new();
    for page in 1.. {
        let req = agent
            .get(&format!("https://{host}/api/v1/repos/issues/search"))
            .header("Authorization", &format!("token {token}"))
            .query("type", "pulls")
            .query("state", "closed")
            .query("since", &since)
            .query("limit", "50")
            .query("page", page.to_string());
        let req = if own_prs { req.query("created", "true") } else { req.query("created_by", &user) };
        let resp = req.call();
        let data: Vec<Issue> = match resp.map_err(|e| e.to_string()).and_then(|mut r| r.body_mut().read_json().map_err(|e| e.to_string())) {
            Ok(d) => d,
            // network/auth errors shouldn't kill the GitHub results
            Err(e) => return SourceResult::err(name, prs, e),
        };
        if data.is_empty() {
            break;
        }
        for item in data {
            let Some(info) = item.pull_request else { continue };
            let Some(merged_at) = info.merged_at.filter(|_| info.merged == Some(true)) else { continue };
            if item.user.and_then(|u| u.login).as_deref() != Some(user.as_str()) {
                continue;
            }
            let Ok(merged) = parse_ts(&merged_at) else { continue };
            if in_range(merged, start, end) {
                prs.push(Pr {
                    source: name.into(),
                    repo: item.repository.and_then(|r| r.full_name).unwrap_or_default(),
                    number: item.number,
                    title: item.title,
                    merged_at: merged,
                    url: info.html_url.or(item.html_url).unwrap_or_default(),
                    labels: item.labels.unwrap_or_default().into_iter().map(|l| l.name).collect(),
                    bot: false,
                });
            }
        }
    }
    SourceResult::ok(name, prs)
}
