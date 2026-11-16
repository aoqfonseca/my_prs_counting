use std::collections::HashMap;

use chrono::NaiveDate;
use comfy_table::{Cell, Color, ContentArrangement, Table, presets::UTF8_FULL_CONDENSED};
use prs::SourceResult;

const BOLD: &str = "\x1b[1m";
const DIM: &str = "\x1b[2m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const CYAN: &str = "\x1b[36m";
const RESET: &str = "\x1b[0m";

pub fn render_rich(results: &[SourceResult], start: NaiveDate, end: NaiveDate, label: &str, bot_label: &str) {
    let all: Vec<_> = results.iter().flat_map(|r| &r.prs).collect();
    let multi_day = start != end;
    let bots = all.iter().filter(|p| p.bot).count();

    println!(
        "\n{BOLD}🔀 PRs mergeados{RESET}  {CYAN}{label}{RESET}   {BOLD}{GREEN}{} no total{RESET}   {GREEN}🌲 {bots} via {bot_label}{RESET}",
        all.len()
    );

    for res in results {
        println!();
        if let Some(e) = &res.error {
            println!("{BOLD}{}{RESET}  {RED}⚠ {e}{RESET}", res.name);
        }
        if res.prs.is_empty() {
            if res.error.is_none() {
                println!("{BOLD}{}{RESET}  {DIM}nenhum PR mergeado{RESET}", res.name);
            }
            continue;
        }
        println!("{BOLD}{}{RESET}  {DIM}({}){RESET}", res.name, res.prs.len());

        let mut table = Table::new();
        table.load_style(UTF8_FULL_CONDENSED).set_content_arrangement(ContentArrangement::Dynamic);
        let mut header = vec!["Hora", "Repositório", "PR", "Título", "Labels"];
        if multi_day {
            header.insert(0, "Data");
        }
        table.set_header(header.into_iter().map(|h| Cell::new(h).fg(Color::Magenta)));

        let mut prs: Vec<_> = res.prs.iter().collect();
        prs.sort_by(|a, b| b.merged_at.cmp(&a.merged_at));
        for pr in prs {
            let mark = if pr.bot { "🌲 " } else { "" };
            // OSC 8 hyperlink on the PR number.
            let link = format!("\x1b]8;;{}\x1b\\#{}\x1b]8;;\x1b\\", pr.url, pr.number);
            let mut row = vec![
                Cell::new(pr.merged_at.format("%H:%M")).fg(Color::DarkGrey),
                Cell::new(&pr.repo).fg(Color::Cyan),
                Cell::new(link).fg(Color::Yellow),
                Cell::new(format!("{mark}{}", pr.title)),
                Cell::new(pr.labels.join(", ")).fg(Color::DarkGrey),
            ];
            if multi_day {
                row.insert(0, Cell::new(pr.merged_at.format("%a %d/%m")).fg(Color::DarkGrey));
            }
            table.add_row(row);
        }
        println!("{table}");
    }

    if !all.is_empty() {
        let mut counts: HashMap<&str, usize> = HashMap::new();
        for p in &all {
            *counts.entry(p.repo.as_str()).or_default() += 1;
        }
        let mut top: Vec<_> = counts.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(b.0)));
        println!("\n{BOLD}Top repositórios{RESET}");
        for (repo, n) in top.into_iter().take(5) {
            println!("  {CYAN}{repo}{RESET}  {BOLD}{n}{RESET}  {}", "█".repeat(n));
        }
    }
    println!();
}

pub fn render_plain(results: &[SourceResult], start: NaiveDate, end: NaiveDate, label: &str, bot_label: &str) {
    let total: usize = results.iter().map(|r| r.prs.len()).sum();
    let bots = results.iter().flat_map(|r| &r.prs).filter(|p| p.bot).count();
    println!("\nPRs mergeados {label} — {total} no total ({bots} via {bot_label})\n");
    for res in results {
        println!("── {} ({}) {}", res.name, res.prs.len(), "─".repeat(40));
        if let Some(e) = &res.error {
            println!("  ! {e}");
        }
        let mut prs: Vec<_> = res.prs.iter().collect();
        prs.sort_by(|a, b| b.merged_at.cmp(&a.merged_at));
        for pr in prs {
            let fmt = if start != end { "%d/%m %H:%M" } else { "%H:%M" };
            let mark = if pr.bot { format!("[{bot_label}] ") } else { String::new() };
            let labels = if pr.labels.is_empty() { String::new() } else { format!("  <{}>", pr.labels.join(", ")) };
            println!("  #{:<6} {:<35} {}  {mark}{}{labels}", pr.number, pr.repo, pr.merged_at.format(fmt), pr.title);
            println!("          {}", pr.url);
        }
        println!();
    }
}
