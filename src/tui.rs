use std::sync::mpsc::{self, Receiver, Sender};
use std::thread;
use std::time::Duration;

use anyhow::Result;
use chrono::NaiveDate;
use prs::range::{self, RangeSpec};
use prs::{Config, Pr, SourceResult, fetch_all};
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState, Wrap};
use ratatui::{DefaultTerminal, Frame};

const SPINNER: [&str; 4] = ["⠋", "⠙", "⠹", "⠸"];

#[derive(Clone, Copy, PartialEq)]
pub enum Mode {
    Today,
    Yesterday,
    Week,
}

impl Mode {
    const ALL: [(Mode, &'static str, char); 3] =
        [(Mode::Today, "Hoje", 't'), (Mode::Yesterday, "Ontem", 'y'), (Mode::Week, "7 dias", 'w')];

    fn range(self) -> (NaiveDate, NaiveDate) {
        let spec = RangeSpec { yesterday: self == Mode::Yesterday, week: self == Mode::Week, ..Default::default() };
        let (start, end, _) = range::resolve(&spec).expect("preset ranges are always valid");
        (start, end)
    }
}

type Fetched = (u64, Vec<SourceResult>);

struct App {
    cfg: Config,
    mode: Mode,
    results: Vec<SourceResult>,
    prs: Vec<Pr>,
    table: TableState,
    loading: bool,
    generation: u64,
    tick: usize,
    quit: bool,
    tx: Sender<Fetched>,
    rx: Receiver<Fetched>,
}

pub fn run(cfg: Config, mode: Mode) -> Result<()> {
    let mut terminal = ratatui::init();
    let res = event_loop(&mut terminal, App::new(cfg, mode));
    ratatui::restore();
    res
}

fn event_loop(terminal: &mut DefaultTerminal, mut app: App) -> Result<()> {
    app.load();
    while !app.quit {
        terminal.draw(|f| app.draw(f))?;
        while let Ok((generation, results)) = app.rx.try_recv() {
            app.apply(generation, results);
        }
        app.tick = app.tick.wrapping_add(1);
        if event::poll(Duration::from_millis(100))?
            && let Event::Key(key) = event::read()?
            && key.kind == KeyEventKind::Press
        {
            app.on_key(key);
        }
    }
    Ok(())
}

impl App {
    fn new(cfg: Config, mode: Mode) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            cfg,
            mode,
            results: vec![],
            prs: vec![],
            table: TableState::default(),
            loading: false,
            generation: 0,
            tick: 0,
            quit: false,
            tx,
            rx,
        }
    }

    /// Start a background fetch; results from superseded fetches are dropped in `apply`.
    fn load(&mut self) {
        self.generation += 1;
        self.loading = true;
        let (generation, cfg, tx) = (self.generation, self.cfg.clone(), self.tx.clone());
        let (start, end) = self.mode.range();
        thread::spawn(move || {
            let _ = tx.send((generation, fetch_all(&cfg, start, end)));
        });
    }

    fn apply(&mut self, generation: u64, results: Vec<SourceResult>) {
        if generation != self.generation {
            return;
        }
        let mut prs: Vec<Pr> = results.iter().flat_map(|r| r.prs.clone()).collect();
        prs.sort_by(|a, b| b.merged_at.cmp(&a.merged_at));
        self.results = results;
        self.prs = prs;
        self.loading = false;
        let sel = self.table.selected().unwrap_or(0).min(self.prs.len().saturating_sub(1));
        self.table.select(if self.prs.is_empty() { None } else { Some(sel) });
    }

    fn set_mode(&mut self, mode: Mode) {
        if self.mode != mode {
            self.mode = mode;
            self.load();
        }
    }

    fn move_by(&mut self, delta: isize) {
        if self.prs.is_empty() {
            return;
        }
        let cur = self.table.selected().unwrap_or(0) as isize;
        self.table.select(Some((cur + delta).clamp(0, self.prs.len() as isize - 1) as usize));
    }

    fn selected(&self) -> Option<&Pr> {
        self.table.selected().and_then(|i| self.prs.get(i))
    }

    fn on_key(&mut self, key: KeyEvent) {
        match key.code {
            KeyCode::Char('q') | KeyCode::Esc => self.quit = true,
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => self.quit = true,
            KeyCode::Char('j') | KeyCode::Down => self.move_by(1),
            KeyCode::Char('k') | KeyCode::Up => self.move_by(-1),
            KeyCode::PageDown => self.move_by(10),
            KeyCode::PageUp => self.move_by(-10),
            KeyCode::Char('g') | KeyCode::Home => self.move_by(isize::MIN / 2),
            KeyCode::Char('G') | KeyCode::End => self.move_by(isize::MAX / 2),
            KeyCode::Char('t') => self.set_mode(Mode::Today),
            KeyCode::Char('y') => self.set_mode(Mode::Yesterday),
            KeyCode::Char('w') => self.set_mode(Mode::Week),
            KeyCode::Char('r') => self.load(),
            KeyCode::Char('o') | KeyCode::Enter => {
                if let Some(pr) = self.selected() {
                    let _ = open::that(&pr.url);
                }
            }
            _ => {}
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let [header, body, detail, footer] = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(5),
            Constraint::Length(6),
            Constraint::Length(1),
        ])
        .areas(f.area());

        self.draw_header(f, header);
        self.draw_list(f, body);
        self.draw_detail(f, detail);
        let help = " j/k mover · t hoje · y ontem · w 7 dias · r atualizar · o abrir · q sair";
        f.render_widget(Paragraph::new(help).style(Style::new().fg(Color::DarkGray)), footer);
    }

    fn draw_header(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut spans = Vec::new();
        for (mode, name, key) in Mode::ALL {
            let style = if mode == self.mode {
                Style::new().fg(Color::Black).bg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::Gray)
            };
            spans.push(Span::styled(format!(" {name} ({key}) "), style));
            spans.push(Span::raw(" "));
        }
        let bots = self.prs.iter().filter(|p| p.bot).count();
        spans.push(Span::styled(format!("{} no total", self.prs.len()), Style::new().fg(Color::Green).bold()));
        spans.push(Span::styled(format!("   🌲 {bots} via {}", self.cfg.bot_label), Style::new().fg(Color::Green)));
        if self.loading {
            spans.push(Span::styled(format!("   {} carregando…", SPINNER[self.tick / 2 % SPINNER.len()]), Style::new().fg(Color::Yellow)));
        }
        f.render_widget(
            Paragraph::new(Line::from(spans)).block(Block::new().borders(Borders::ALL).title(" 🔀 PRs mergeados ")),
            area,
        );
    }

    fn draw_list(&mut self, f: &mut Frame, area: ratatui::layout::Rect) {
        let block = Block::new().borders(Borders::ALL);
        if self.prs.is_empty() {
            let msg = if self.loading { "carregando…" } else { "nenhum PR mergeado" };
            f.render_widget(Paragraph::new(msg).style(Style::new().fg(Color::DarkGray)).block(block), area);
            return;
        }
        let multi_day = self.mode != Mode::Today && self.mode != Mode::Yesterday;
        let rows = self.prs.iter().map(|pr| {
            let when = if multi_day { pr.merged_at.format("%a %d/%m %H:%M") } else { pr.merged_at.format("%H:%M") };
            let src = if pr.source == "GitHub" { "GH" } else { "FJ" };
            let mark = if pr.bot { "🌲 " } else { "" };
            Row::new([
                Cell::from(when.to_string()).fg(Color::DarkGray),
                Cell::from(src).fg(Color::Magenta),
                Cell::from(pr.repo.clone()).fg(Color::Cyan),
                Cell::from(format!("#{}", pr.number)).fg(Color::Yellow),
                Cell::from(format!("{mark}{}", pr.title)),
            ])
        });
        let when_w = if multi_day { 15 } else { 5 };
        let table = Table::new(
            rows,
            [
                Constraint::Length(when_w),
                Constraint::Length(2),
                Constraint::Length(28),
                Constraint::Length(8),
                Constraint::Fill(1),
            ],
        )
        .block(block)
        .column_spacing(1)
        .row_highlight_style(Style::new().add_modifier(Modifier::REVERSED))
        .highlight_symbol("▶ ");
        f.render_stateful_widget(table, area, &mut self.table);
    }

    fn draw_detail(&self, f: &mut Frame, area: ratatui::layout::Rect) {
        let mut lines = Vec::new();
        if let Some(pr) = self.selected() {
            lines.push(Line::from(Span::styled(pr.title.clone(), Style::new().bold())));
            lines.push(Line::from(Span::styled(pr.url.clone(), Style::new().fg(Color::Blue).add_modifier(Modifier::UNDERLINED))));
            let labels = if pr.labels.is_empty() { "—".to_string() } else { pr.labels.join(", ") };
            lines.push(Line::from(format!("{} · {} · labels: {labels}", pr.source, pr.merged_at.format("%d/%m/%Y %H:%M"))));
        }
        for r in self.results.iter().filter(|r| r.error.is_some()) {
            lines.push(Line::from(Span::styled(
                format!("⚠ {}: {}", r.name, r.error.as_deref().unwrap_or_default()),
                Style::new().fg(Color::Red),
            )));
        }
        f.render_widget(
            Paragraph::new(lines).wrap(Wrap { trim: true }).block(Block::new().borders(Borders::ALL).title(" Detalhes ")),
            area,
        );
    }
}
