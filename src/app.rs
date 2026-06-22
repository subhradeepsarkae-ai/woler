use crate::cache;
use crate::db::{self, Category, Package};
use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyModifiers};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen};
use crossterm::ExecutableCommand;
use ratatui::layout::{Constraint, Layout, Margin};
use ratatui::style::{Color, Modifier, Style, Stylize};
use ratatui::text::{Line, Span};
use ratatui::widgets::{
    Block, BorderType, Borders, Clear, List, ListItem, ListState, Paragraph, Tabs,
};
use ratatui::{Frame, Terminal};
use std::io::stdout;
use std::time::Duration;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Tab {
    All,
    Apps,
    Clis,
    Libraries,
}

impl Tab {
    fn all() -> &'static [Tab] {
        &[Tab::All, Tab::Apps, Tab::Clis, Tab::Libraries]
    }

    fn name(&self) -> &'static str {
        match self {
            Tab::All => "All",
            Tab::Apps => "Apps",
            Tab::Clis => "CLIs",
            Tab::Libraries => "Libs",
        }
    }

    fn index(&self) -> usize {
        Self::all().iter().position(|t| t == self).unwrap()
    }

    fn from_index(i: usize) -> Self {
        Self::all()[i.min(3)]
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Browse,
    Search,
    ConfirmDelete,
    Deleting,
}

pub struct App {
    packages: Vec<Package>,
    filtered: Vec<usize>,
    selected: usize,
    scroll: usize,
    tab: Tab,
    mode: Mode,
    search: String,
    show_detail: bool,
    detail_pkg: Option<usize>,
    confirm_pkg: Option<usize>,
    status_message: String,
    total_apps: usize,
    total_clis: usize,
    total_libs: usize,
    quit: bool,
}

impl App {
    fn new(packages: Vec<Package>, initial_tab: Option<Tab>) -> Self {
        let (apps, clis, libs) = db::packages_by_category(&packages);
        let filtered: Vec<usize> = (0..packages.len()).collect();
        let tab = initial_tab.unwrap_or(Tab::All);
        let mut app = App {
            packages,
            filtered,
            selected: 0,
            scroll: 0,
            tab,
            mode: Mode::Browse,
            search: String::new(),
            show_detail: false,
            detail_pkg: None,
            confirm_pkg: None,
            status_message: String::new(),
            total_apps: apps,
            total_clis: clis,
            total_libs: libs,
            quit: false,
        };
        app.filter();
        app
    }

    fn filter(&mut self) {
        let tab = self.tab;
        let search = self.search.to_lowercase();

        self.filtered = self
            .packages
            .iter()
            .enumerate()
            .filter(|&(_, p)| match tab {
                Tab::All => true,
                Tab::Apps => p.has_desktop,
                Tab::Clis => !p.bins.is_empty(),
                Tab::Libraries => !p.has_desktop && p.bins.is_empty(),
            })
            .filter(|&(_, p)| {
                search.is_empty()
                    || p.name.to_lowercase().contains(&search)
                    || p.description.to_lowercase().contains(&search)
            })
            .map(|(i, _)| i)
            .collect();

        self.selected = self.selected.min(self.filtered.len().saturating_sub(1));
    }

    fn remove_and_refresh(&mut self) {
        disable_raw_mode().ok();
        let _ = stdout().execute(LeaveAlternateScreen);

        let pkg_name = self.confirm_pkg.and_then(|i| {
            self.filtered.get(i).map(|&idx| self.packages[idx].name.clone())
        });

        if let Some(ref name) = pkg_name {
            let status = std::process::Command::new("sudo")
                .args(["pacman", "-Rns", name])
                .status();

            match status {
                Ok(s) if s.success() => {
                    self.status_message = format!("Removed {}", name);
                }
                Ok(s) => {
                    self.status_message =
                        format!("Failed to remove {} (exit code: {:?})", name, s.code());
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }

        enable_raw_mode().ok();
        let _ = stdout().execute(EnterAlternateScreen);

        let packages = load_packages_internal();
        match packages {
            Ok(pkgs) => {
                let (apps, clis, libs) = db::packages_by_category(&pkgs);
                self.packages = pkgs;
                self.total_apps = apps;
                self.total_clis = clis;
                self.total_libs = libs;
            }
            Err(e) => {
                self.status_message = format!("Refresh error: {}", e);
            }
        }

        self.mode = Mode::Browse;
        self.confirm_pkg = None;
        self.filter();
    }

    fn remove_with_orphans_and_refresh(&mut self) {
        disable_raw_mode().ok();
        let _ = stdout().execute(LeaveAlternateScreen);

        let pkg_name = self.confirm_pkg.and_then(|i| {
            self.filtered.get(i).map(|&idx| self.packages[idx].name.clone())
        });

        if let Some(ref name) = pkg_name {
            let status = std::process::Command::new("sudo")
                .args([
                    "pacman",
                    "-Rns",
                    name,
                    "$(pacman -Qdtq)",
                ])
                .status();

            match status {
                Ok(s) if s.success() => {
                    self.status_message = format!("Removed {} with orphans", name);
                }
                Ok(s) => {
                    self.status_message =
                        format!("Failed to remove {} (exit code: {:?})", name, s.code());
                }
                Err(e) => {
                    self.status_message = format!("Error: {}", e);
                }
            }
        }

        enable_raw_mode().ok();
        let _ = stdout().execute(EnterAlternateScreen);

        let packages = load_packages_internal();
        match packages {
            Ok(pkgs) => {
                let (apps, clis, libs) = db::packages_by_category(&pkgs);
                self.packages = pkgs;
                self.total_apps = apps;
                self.total_clis = clis;
                self.total_libs = libs;
            }
            Err(e) => {
                self.status_message = format!("Refresh error: {}", e);
            }
        }

        self.mode = Mode::Browse;
        self.confirm_pkg = None;
        self.filter();
    }
}

fn load_packages_internal() -> Result<Vec<Package>> {
    if let Some(cached) = cache::load()? {
        return Ok(cached);
    }
    let packages = db::scan()?;
    cache::save(&packages)?;
    Ok(packages)
}

pub fn run_tui() -> Result<()> {
    run_tui_with_tab(None)
}

pub fn run_tui_with_tab(initial_tab: Option<Tab>) -> Result<()> {
    let packages = load_packages_internal()?;
    let (apps, clis, libs) = db::packages_by_category(&packages);

    enable_raw_mode()?;
    let mut stdout = stdout();
    stdout.execute(EnterAlternateScreen)?;
    let mut terminal = ratatui::Terminal::new(ratatui::backend::CrosstermBackend::new(stdout))?;
    terminal.clear()?;

    let mut app = App::new(packages, initial_tab);
    app.status_message = format!(
        "Packages: {}  Apps: {}  CLIs: {}  Libs: {}  |  Cache saved",
        app.packages.len(),
        apps,
        clis,
        libs
    );

    let res = run_loop(&mut terminal, &mut app);

    disable_raw_mode()?;
    terminal.backend_mut().execute(LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    if let Err(e) = &res {
        eprintln!("Error: {}", e);
    }

    res
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
) -> Result<()> {
    loop {
        terminal.draw(|f| render(f, app))?;

        if !event::poll(Duration::from_millis(50))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            match app.mode {
                Mode::Browse => handle_browse(terminal, app, key),
                Mode::Search => handle_search(app, key),
                Mode::ConfirmDelete => handle_confirm(app, key),
                Mode::Deleting => {}
            }
        }

        if app.quit {
            break Ok(());
        }
    }
}

fn handle_browse<B: ratatui::backend::Backend>(
    _terminal: &mut Terminal<B>,
    app: &mut App,
    key: event::KeyEvent,
) {
    match key.code {
        KeyCode::Char('q') => {
            app.quit = true;
        }
        KeyCode::Char('?') => {
            app.status_message = String::from(
                "j/k/↑/↓ navigate | Tab/1-4: tabs | /: search | Enter: detail | d: delete | D: delete+orphans | r: refresh | q: quit",
            );
        }
        KeyCode::Char('/') => {
            app.search.clear();
            app.mode = Mode::Search;
        }
        KeyCode::Char('r') => {
            app.status_message = "Refreshing...".into();
            let packages = load_packages_internal();
            match packages {
                Ok(pkgs) => {
                    let (apps, clis, libs) = db::packages_by_category(&pkgs);
                    app.packages = pkgs;
                    app.total_apps = apps;
                    app.total_clis = clis;
                    app.total_libs = libs;
                    app.filter();
                    app.status_message = format!("Refreshed: {} packages", app.packages.len());
                }
                Err(e) => {
                    app.status_message = format!("Refresh failed: {}", e);
                }
            }
        }
        KeyCode::Char('d') => {
            if !app.filtered.is_empty() {
                app.mode = Mode::ConfirmDelete;
                app.confirm_pkg = Some(app.selected);
            }
        }
        KeyCode::Char('D') => {
            if !app.filtered.is_empty() {
                app.mode = Mode::ConfirmDelete;
                app.confirm_pkg = Some(app.selected);
            }
        }
        KeyCode::Enter => {
            if !app.filtered.is_empty() {
                app.show_detail = true;
                app.detail_pkg = Some(app.selected);
            }
        }
        KeyCode::Esc => {
            if app.show_detail {
                app.show_detail = false;
                app.detail_pkg = None;
            }
        }
        KeyCode::Tab | KeyCode::Char('\t') => {
            let idx = (app.tab.index() + 1) % 4;
            app.tab = Tab::from_index(idx);
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::BackTab => {
            let idx = if app.tab.index() == 0 {
                3
            } else {
                app.tab.index() - 1
            };
            app.tab = Tab::from_index(idx);
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::Char('1') => {
            app.tab = Tab::All;
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::Char('2') => {
            app.tab = Tab::Apps;
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::Char('3') => {
            app.tab = Tab::Clis;
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::Char('4') => {
            app.tab = Tab::Libraries;
            app.selected = 0;
            app.scroll = 0;
            app.filter();
        }
        KeyCode::Down | KeyCode::Char('j') => {
            if !app.filtered.is_empty() {
                app.selected = (app.selected + 1).min(app.filtered.len() - 1);
            }
        }
        KeyCode::Up | KeyCode::Char('k') => {
            app.selected = app.selected.saturating_sub(1);
        }
        KeyCode::PageDown => {
            if !app.filtered.is_empty() {
                app.selected = (app.selected + 10).min(app.filtered.len() - 1);
            }
        }
        KeyCode::PageUp => {
            app.selected = app.selected.saturating_sub(10);
        }
        KeyCode::Home => {
            app.selected = 0;
        }
        KeyCode::End => {
            if !app.filtered.is_empty() {
                app.selected = app.filtered.len() - 1;
            }
        }
        _ => {}
    }
}

fn handle_search(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Esc => {
            app.search.clear();
            app.mode = Mode::Browse;
            app.filter();
        }
        KeyCode::Enter => {
            app.mode = Mode::Browse;
        }
        KeyCode::Backspace => {
            app.search.pop();
            app.filter();
        }
        KeyCode::Char(c) => {
            if key.modifiers.contains(KeyModifiers::CONTROL) {
                return;
            }
            app.search.push(c);
            app.filter();
            app.selected = 0;
        }
        _ => {}
    }
}

fn handle_confirm(app: &mut App, key: event::KeyEvent) {
    match key.code {
        KeyCode::Char('y') | KeyCode::Char('Y') => {
            app.mode = Mode::Deleting;
            app.remove_and_refresh();
        }
        KeyCode::Char('D') => {
            app.mode = Mode::Deleting;
            app.remove_with_orphans_and_refresh();
        }
        KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => {
            app.mode = Mode::Browse;
            app.confirm_pkg = None;
        }
        _ => {}
    }
}

fn render(f: &mut Frame, app: &App) {
    let area = f.area();
    let vertical = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Min(1),
        Constraint::Length(1),
    ]);
    let chunks = vertical.split(area);
    let top_bar = chunks[0];
    let tab_bar = chunks[1];
    let main_area = chunks[2];
    let status_bar = chunks[3];

    render_top_bar(f, top_bar, app);
    render_tab_bar(f, tab_bar, app);

    if app.show_detail && app.detail_pkg.is_some() && app.filtered.len() > 1 {
        let horizontal = Layout::horizontal([Constraint::Percentage(55), Constraint::Percentage(45)]);
        let [list_area, detail_area] = horizontal.areas(main_area);
        render_package_list(f, list_area, app);
        render_detail(f, detail_area, app);
    } else if app.show_detail && app.detail_pkg.is_some() {
        render_detail(f, main_area, app);
    } else {
        render_package_list(f, main_area, app);
    }

    render_status_bar(f, status_bar, app);

    if app.mode == Mode::ConfirmDelete {
        render_delete_modal(f, f.area(), app);
    }
}

fn render_top_bar(f: &mut Frame, area: ratatui::layout::Rect, _app: &App) {
    let help = Line::from(vec![
        Span::styled(" woler ", Style::new().bold().fg(Color::Cyan)),
        Span::styled("0.1.0", Style::new().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[?] help", Style::new().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[/] search", Style::new().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[d] delete", Style::new().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::styled("[q] quit", Style::new().fg(Color::DarkGray)),
    ]);
    f.render_widget(
        Paragraph::new(help).style(Style::new().bg(Color::Reset)),
        area,
    );
}

fn render_tab_bar(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let titles: Vec<Line> = Tab::all()
        .iter()
        .map(|t| {
            let name = t.name();
            if *t == app.tab {
                Line::from(Span::styled(
                    format!(" {} ", name),
                    Style::new()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ))
            } else {
                Line::from(Span::styled(
                    format!(" {} ", name),
                    Style::new().fg(Color::Gray),
                ))
            }
        })
        .collect();

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::BOTTOM)
                .border_type(BorderType::Plain),
        )
        .highlight_style(Style::new().add_modifier(Modifier::BOLD))
        .select(app.tab.index());
    f.render_widget(tabs, area);
}

fn render_package_list(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let _ = Layout::horizontal([
        Constraint::Fill(1),
        Constraint::Length(if area.width > 60 { 0 } else { 0 }),
    ])
    .split(area);

    let items: Vec<ListItem> = app
        .filtered
        .iter()
        .map(|&idx| {
            let pkg = &app.packages[idx];
            let cat = pkg.category();
            let cat_str = match cat {
                Category::App => "APP",
                Category::Cli => "CLI",
                Category::Library => "LIB",
            };
            let cat_color = match cat {
                Category::App => Color::Green,
                Category::Cli => Color::Yellow,
                Category::Library => Color::Blue,
            };

            let name_style = if cat == Category::App {
                Style::new().fg(Color::White).add_modifier(Modifier::BOLD)
            } else {
                Style::new().fg(Color::White)
            };

            let line = Line::from(vec![
                Span::styled(
                    format!(" {:<24}", pkg.name),
                    name_style,
                ),
                Span::styled(
                    format!(" {:<14}", pkg.version),
                    Style::new().fg(Color::DarkGray),
                ),
                Span::styled(
                    format!(" {:>8}", pkg.size_human()),
                    Style::new().fg(Color::Gray),
                ),
                Span::raw(" "),
                Span::styled(
                    format!(" {}", cat_str),
                    Style::new().fg(cat_color).add_modifier(Modifier::BOLD),
                ),
            ]);
            ListItem::new(line)
        })
        .collect();

    let selected = if app.filtered.is_empty() {
        None
    } else {
        Some(app.selected)
    };

    let mut list_state = ListState::default();
    list_state.select(selected);

    let list = List::new(items)
        .highlight_style(
            Style::new()
                .bg(Color::Rgb(30, 40, 60))
                .add_modifier(Modifier::BOLD),
        )
        .highlight_symbol("▸")
        .block(
            Block::default()
                .borders(Borders::LEFT | Borders::RIGHT)
                .style(Style::new().bg(Color::Rgb(18, 18, 28))),
        );

    f.render_stateful_widget(list, area, &mut list_state);
}

fn render_detail(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pkg = match app
        .detail_pkg
        .and_then(|i| app.filtered.get(i))
        .map(|&idx| &app.packages[idx])
    {
        Some(p) => p,
        None => return,
    };

    let lines = vec![
        Line::from(vec![
            Span::styled(" ", Style::new()),
            Span::styled(pkg.name.clone(), Style::new().bold().fg(Color::Cyan)),
        ]),
        Line::from(vec![
            Span::styled(" Version:   ", Style::new().fg(Color::DarkGray)),
            Span::styled(&pkg.version, Style::new().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Size:      ", Style::new().fg(Color::DarkGray)),
            Span::styled(pkg.size_human(), Style::new().fg(Color::White)),
        ]),
        Line::from(vec![
            Span::styled(" Type:      ", Style::new().fg(Color::DarkGray)),
            Span::styled(pkg.type_label(), Style::new().fg(Color::Green)),
        ]),
        Line::from(vec![
            Span::styled(" Installed: ", Style::new().fg(Color::DarkGray)),
            Span::styled(pkg.date_formatted(), Style::new().fg(Color::White)),
        ]),
        Line::from(""),
        Line::from(vec![
            Span::styled(" Description:", Style::new().fg(Color::DarkGray)),
        ]),
        Line::from(vec![
            Span::styled(
                textwrap(&pkg.description, area.width.saturating_sub(3) as usize),
                Style::new().fg(Color::White),
            ),
        ]),
    ];

    let mut detail_lines = lines;

    if !pkg.bins.is_empty() {
        detail_lines.push(Line::from(""));
        detail_lines.push(Line::from(vec![
            Span::styled(" Binaries:", Style::new().fg(Color::DarkGray)),
        ]));
        for bin in &pkg.bins {
            detail_lines.push(Line::from(vec![
                Span::styled(format!("   {}", bin), Style::new().fg(Color::Yellow)),
            ]));
        }
    }

    detail_lines.push(Line::from(""));
    detail_lines.push(Line::from(vec![
        Span::styled(" [d] Remove package", Style::new().fg(Color::Red)),
    ]));
    detail_lines.push(Line::from(vec![
        Span::styled(" [D] Remove + orphans", Style::new().fg(Color::Red)),
    ]));

    let detail = Paragraph::new(detail_lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .border_type(BorderType::Rounded)
                .border_style(Style::new().fg(Color::Cyan))
                .style(Style::new().bg(Color::Rgb(18, 18, 28))),
        )
        .style(Style::new().bg(Color::Rgb(18, 18, 28)));

    f.render_widget(detail, area);
}

fn render_status_bar(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let chunks = Layout::horizontal([Constraint::Percentage(60), Constraint::Percentage(40)]).split(area);
    let left_area = chunks[0];
    let right_area = chunks[1];

    let total = app.filtered.len();
    let selected = if app.filtered.is_empty() {
        0
    } else {
        app.selected + 1
    };

    let left_text = format!(
        " {} / {}  |  GUI: {}  CLI: {}  Lib: {}  |  {} total",
        selected,
        total,
        app.total_apps,
        app.total_clis,
        app.total_libs,
        app.packages.len(),
    );

    f.render_widget(
        Paragraph::new(left_text).style(Style::new().fg(Color::DarkGray)),
        left_area,
    );

    match app.mode {
        Mode::Search => {
            let search_text = format!(" Search: {}", app.search);
            f.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    search_text,
                    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD),
                )))
                .style(Style::new().bg(Color::Rgb(20, 30, 40))),
                right_area,
            );
        }
        _ => {
            if !app.status_message.is_empty() {
                f.render_widget(
                    Paragraph::new(Line::from(Span::styled(
                        &app.status_message,
                        Style::new().fg(Color::DarkGray),
                    ))),
                    right_area,
                );
            }
        }
    }
}

fn render_delete_modal(f: &mut Frame, area: ratatui::layout::Rect, app: &App) {
    let pkg_name = app
        .confirm_pkg
        .and_then(|i| app.filtered.get(i))
        .map(|&idx| &app.packages[idx].name)
        .cloned()
        .unwrap_or_default();

    let block = Block::default()
        .title(" Confirm Removal ")
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(Style::new().fg(Color::Red))
        .style(Style::new().bg(Color::Rgb(30, 10, 10)));

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            format!(" Remove {}?", pkg_name),
            Style::new().fg(Color::White).bold(),
        )),
        Line::from(""),
        Line::from(vec![
            Span::styled(" [y] Remove  ", Style::new().fg(Color::Red)),
            Span::styled("[n] Cancel  ", Style::new().fg(Color::DarkGray)),
            Span::styled("[D] + orphans", Style::new().fg(Color::Red)),
        ]),
        Line::from(""),
    ];

    let (modal_w, modal_h) = (44u16, 7u16);
    let x = area.x.saturating_add(area.width.saturating_sub(modal_w) / 2);
    let y = area.y.saturating_add(area.height.saturating_sub(modal_h) / 2);
    let modal_area = ratatui::layout::Rect::new(x, y, modal_w, modal_h);

    let clearing = Clear;
    f.render_widget(clearing, modal_area);
    f.render_widget(block, modal_area);

    let inner = modal_area.inner(Margin {
        horizontal: 2,
        vertical: 1,
    });
    f.render_widget(Paragraph::new(text).centered(), inner);
}

fn textwrap(text: &str, max_width: usize) -> String {
    if max_width < 10 {
        return text.chars().take(max_width).collect();
    }
    let mut result = String::new();
    let mut line_len = 0;
    for word in text.split_whitespace() {
        if line_len + word.len() + 1 > max_width {
            result.push('\n');
            line_len = 0;
        } else if line_len > 0 {
            result.push(' ');
            line_len += 1;
        }
        result.push_str(word);
        line_len += word.len();
    }
    result
}
