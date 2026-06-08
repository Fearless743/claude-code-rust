use crate::api::message::{ContentBlock, Message};
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode},
};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;
use tokio::sync::mpsc;

enum InputMode {
    Normal,
    Insert,
}

struct App {
    input: String,
    cursor_pos: usize,
    messages: Vec<String>,
    mode: InputMode,
    scroll: u16,
    engine_tx: mpsc::UnboundedSender<String>,
    engine_rx: mpsc::UnboundedReceiver<String>,
}

impl App {
    fn new() -> (Self, mpsc::UnboundedSender<String>) {
        let (tx, rx) = mpsc::unbounded_channel();
        (
            Self {
                input: String::new(),
                cursor_pos: 0,
                messages: vec![
                    "Claude Code v2.1.888 — Type /help for commands, /quit to exit".into(),
                ],
                mode: InputMode::Insert,
                scroll: 0,
                engine_tx: tx.clone(),
                engine_rx: rx,
            },
            tx,
        )
    }
}

pub fn run_repl(
    settings: crate::config::Settings,
    initial_prompt: Option<String>,
) -> eyre::Result<()> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let (mut app, prompt_tx) = App::new();
    let res = run_app(&mut terminal, &mut app, settings, initial_prompt, prompt_tx);

    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    res
}

fn run_app<B: Backend>(
    terminal: &mut Terminal<B>,
    app: &mut App,
    settings: crate::config::Settings,
    initial_prompt: Option<String>,
    prompt_tx: mpsc::UnboundedSender<String>,
) -> eyre::Result<()> {
    let rt = tokio::runtime::Runtime::new()?;

    // If there's an initial prompt, send it immediately
    if let Some(prompt) = initial_prompt {
        app.add_message(format!("> {prompt}"));
        let tx = prompt_tx.clone();
        let s = settings.clone();
        rt.spawn(async move {
            match crate::engine::QueryEngine::new(s, None).await {
                Ok(engine) => match engine.run(Some(prompt)).await {
                    Ok(msgs) => {
                        for msg in msgs {
                            if let Message::Assistant { content, .. } = &msg {
                                for block in content {
                                    if let ContentBlock::Text { text } = block {
                                        let _ = tx.send(format!("Assistant: {text}"));
                                    }
                                }
                            }
                        }
                    }
                    Err(e) => {
                        let _ = tx.send(format!("Error: {e}"));
                    }
                },
                Err(e) => {
                    let _ = tx.send(format!("Error: {e}"));
                }
            }
        });
    }

    loop {
        // Drain engine messages
        while let Ok(msg) = app.engine_rx.try_recv() {
            app.add_message(msg);
        }

        terminal.draw(|f| ui(f, app))?;

        if !event::poll(std::time::Duration::from_millis(100))? {
            continue;
        }

        if let Event::Key(key) = event::read()? {
            if key.kind == KeyEventKind::Release {
                continue;
            }
            match app.mode {
                InputMode::Normal => match key.code {
                    KeyCode::Char('i') => app.mode = InputMode::Insert,
                    KeyCode::Char('q') => break,
                    KeyCode::Char('j') => app.scroll = app.scroll.saturating_add(1),
                    KeyCode::Char('k') => app.scroll = app.scroll.saturating_sub(1),
                    _ => {}
                },
                InputMode::Insert => match key.code {
                    KeyCode::Esc => app.mode = InputMode::Normal,
                    KeyCode::Enter => {
                        let input = std::mem::take(&mut app.input);
                        app.cursor_pos = 0;
                        if input == "/quit" {
                            break;
                        }
                        if input.is_empty() {
                            continue;
                        }
                        app.add_message(format!("> {input}"));
                        let tx = prompt_tx.clone();
                        let s = settings.clone();
                        rt.spawn(async move {
                            match crate::engine::QueryEngine::new(s, None).await {
                                Ok(engine) => match engine.run(Some(input)).await {
                                    Ok(msgs) => {
                                        for msg in msgs {
                                            if let Message::Assistant { content, .. } = &msg {
                                                for block in content {
                                                    if let ContentBlock::Text { text } = block {
                                                        let _ = tx.send(text.clone());
                                                    }
                                                }
                                            }
                                        }
                                    }
                                    Err(e) => {
                                        let _ = tx.send(format!("Error: {e:#}"));
                                    }
                                },
                                Err(e) => {
                                    let _ = tx.send(format!("Error: {e:#}"));
                                }
                            }
                        });
                    }
                    KeyCode::Char(c) => {
                        app.input.insert(app.cursor_pos, c);
                        app.cursor_pos += 1;
                    }
                    KeyCode::Backspace => {
                        if app.cursor_pos > 0 {
                            app.cursor_pos -= 1;
                            app.input.remove(app.cursor_pos);
                        }
                    }
                    KeyCode::Left => app.cursor_pos = app.cursor_pos.saturating_sub(1),
                    KeyCode::Right => {
                        if app.cursor_pos < app.input.len() {
                            app.cursor_pos += 1;
                        }
                    }
                    KeyCode::Up => {
                        app.scroll = app.scroll.saturating_sub(1);
                    }
                    KeyCode::Down => {
                        app.scroll += 1;
                    }
                    KeyCode::Home => app.cursor_pos = 0,
                    KeyCode::End => app.cursor_pos = app.input.len(),
                    _ => {}
                },
            }
        }
    }
    Ok(())
}

impl App {
    fn add_message(&mut self, msg: String) {
        self.messages.push(msg);
        // Auto-scroll to bottom
        if self.messages.len() > 50 {
            self.scroll = (self.messages.len() - 50) as u16;
        }
    }
}

fn ui(f: &mut ratatui::Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),
            Constraint::Length(3),
            Constraint::Length(1),
        ])
        .split(f.area());

    // Messages
    let visible: Vec<ListItem> = app
        .messages
        .iter()
        .skip(app.scroll as usize)
        .take(chunks[0].height as usize)
        .map(|m| ListItem::new(m.as_str()))
        .collect();
    let messages =
        List::new(visible).block(Block::default().borders(Borders::ALL).title("Claude Code"));
    f.render_widget(messages, chunks[0]);

    // Input
    let mut input_display = app.input.clone();
    if app.cursor_pos <= input_display.len() {
        input_display.insert(app.cursor_pos, '|');
    }
    let style = match app.mode {
        InputMode::Insert => Style::default().fg(Color::Yellow),
        InputMode::Normal => Style::default(),
    };
    let input = Paragraph::new(format!("> {input_display}"))
        .style(style)
        .block(Block::default().borders(Borders::ALL).title("Input"));
    f.render_widget(input, chunks[1]);

    // Status
    let mode_str = match app.mode {
        InputMode::Normal => "NORMAL",
        InputMode::Insert => "INSERT",
    };
    let status = Line::from(vec![
        Span::styled(
            format!(" {mode_str} "),
            Style::default().fg(Color::Black).bg(Color::Green),
        ),
        Span::raw(" | "),
        Span::raw(format!("msgs: {}", app.messages.len())),
        Span::raw(" | /quit exit | /help commands"),
    ]);
    f.render_widget(Paragraph::new(status), chunks[2]);
}
