//! Lightweight TUI prompts for CLI flows.

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode, KeyEventKind},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{
    prelude::*,
    widgets::{Block, BorderType, Borders, Clear, List, ListItem, Paragraph},
};
use std::io;

pub fn ask_choice(question: &str, options: &[&str]) -> Result<Option<usize>> {
    if options.is_empty() {
        return Ok(None);
    }

    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let result = (|| -> Result<Option<usize>> {
        let mut selected: usize = 0;
        loop {
            terminal.draw(|frame| {
                let area = centered_rect(70, 30, frame.area());
                frame.render_widget(Clear, area);

                let block = Block::default()
                    .title(" Maestro Prompt ")
                    .title_alignment(Alignment::Center)
                    .borders(Borders::ALL)
                    .border_type(BorderType::Double)
                    .style(Style::default().bg(Color::Rgb(10, 10, 15)));
                frame.render_widget(block.clone(), area);

                let inner = block.inner(area);
                let chunks = Layout::default()
                    .direction(Direction::Vertical)
                    .margin(1)
                    .constraints([
                        Constraint::Length(3),
                        Constraint::Min(0),
                        Constraint::Length(2),
                    ])
                    .split(inner);

                let q = Paragraph::new(question)
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::Yellow).bold());
                frame.render_widget(q, chunks[0]);

                let items: Vec<ListItem> = options
                    .iter()
                    .enumerate()
                    .map(|(idx, text)| {
                        let style = if idx == selected {
                            Style::default()
                                .fg(Color::Black)
                                .bg(Color::Cyan)
                                .add_modifier(Modifier::BOLD)
                        } else {
                            Style::default().fg(Color::White)
                        };
                        ListItem::new(Line::from(Span::styled((*text).to_string(), style)))
                    })
                    .collect();

                let list = List::new(items).block(
                    Block::default()
                        .borders(Borders::ALL)
                        .border_type(BorderType::Rounded)
                        .title(" Select "),
                );
                frame.render_widget(list, chunks[1]);

                let help = Paragraph::new("↑/↓ select • Enter confirm • Esc cancel")
                    .alignment(Alignment::Center)
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(help, chunks[2]);
            })?;

            if event::poll(std::time::Duration::from_millis(50))? {
                if let Event::Key(key) = event::read()? {
                    if key.kind != KeyEventKind::Press {
                        continue;
                    }
                    match key.code {
                        KeyCode::Up | KeyCode::Left => {
                            selected = if selected == 0 {
                                options.len().saturating_sub(1)
                            } else {
                                selected - 1
                            };
                        }
                        KeyCode::Down | KeyCode::Right => {
                            selected = if selected + 1 >= options.len() {
                                0
                            } else {
                                selected + 1
                            };
                        }
                        KeyCode::Enter => return Ok(Some(selected)),
                        KeyCode::Esc => return Ok(None),
                        _ => {}
                    }
                }
            }
        }
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
        ])
        .split(r);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
