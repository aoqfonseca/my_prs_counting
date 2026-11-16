mod render;
mod tui;

use std::io::IsTerminal;

use chrono::NaiveDate;
use clap::{ArgGroup, Parser, Subcommand, ValueEnum};
use prs::{Config, fetch_all, range};
use serde_json::json;

#[derive(Clone, Copy, ValueEnum, PartialEq)]
enum Source {
    All,
    Github,
    Forgejo,
}

#[derive(Subcommand)]
enum Command {
    /// Interactive terminal UI (starts on today; use -y / -w to change)
    Tui,
}

/// List PRs you merged in a date range (GitHub + Forgejo).
#[derive(Parser)]
#[command(version, group(ArgGroup::new("when").args(["today", "yesterday", "week", "date"])))]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// today only (the default)
    #[arg(short, long, global = true)]
    today: bool,
    /// yesterday only
    #[arg(short, long, global = true)]
    yesterday: bool,
    /// last 7 days (including today)
    #[arg(short, long, global = true)]
    week: bool,
    /// a specific day
    #[arg(short, long, value_name = "YYYY-MM-DD")]
    date: Option<NaiveDate>,
    /// range start
    #[arg(long, value_name = "YYYY-MM-DD")]
    since: Option<NaiveDate>,
    /// range end (default: today)
    #[arg(long, value_name = "YYYY-MM-DD")]
    until: Option<NaiveDate>,
    #[arg(long, value_enum, default_value = "all", global = true)]
    source: Source,
    #[arg(long, env = "GITHUB_USER", default_value = "@me", global = true)]
    github_user: String,
    #[arg(long, env = "FORGEJO_HOST", default_value = "git.carta.rocks", global = true)]
    forgejo_host: String,
    #[arg(long, env = "FORGEJO_USER", global = true)]
    forgejo_user: Option<String>,
    /// label that marks bot-made PRs (case-insensitive)
    #[arg(long, env = "BOT_LABEL", default_value = "evergreen", global = true)]
    bot_label: String,
    /// machine-readable output
    #[arg(long)]
    json: bool,
    /// disable rich formatting
    #[arg(long)]
    plain: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let cfg = Config {
        github: cli.source != Source::Forgejo,
        forgejo: cli.source != Source::Github,
        github_user: cli.github_user.clone(),
        forgejo_host: cli.forgejo_host.clone(),
        forgejo_user: cli.forgejo_user.clone(),
        bot_label: cli.bot_label.clone(),
    };

    if let Some(Command::Tui) = cli.command {
        let mode = if cli.week {
            tui::Mode::Week
        } else if cli.yesterday {
            tui::Mode::Yesterday
        } else {
            tui::Mode::Today
        };
        return tui::run(cfg, mode);
    }

    let spec = range::RangeSpec {
        yesterday: cli.yesterday,
        week: cli.week,
        date: cli.date,
        since: cli.since,
        until: cli.until,
    };
    let (start, end, label) = match range::resolve(&spec) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("error: {e}");
            std::process::exit(2);
        }
    };

    let results = fetch_all(&cfg, start, end);

    if cli.json {
        let out: Vec<_> = results
            .iter()
            .map(|r| json!({ "source": r.name, "error": r.error, "prs": r.prs }))
            .collect();
        println!("{}", serde_json::to_string_pretty(&out)?);
    } else if !cli.plain && std::io::stdout().is_terminal() {
        render::render_rich(&results, start, end, &label, &cli.bot_label);
    } else {
        render::render_plain(&results, start, end, &label, &cli.bot_label);
    }
    Ok(())
}
