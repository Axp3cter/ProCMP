//! The `--pick` menu.
//!
//! Chooses among tasks the manifest already declares. Drawn on stderr, leaving stdout
//! for `--json`.

use std::io::{self, IsTerminal, Write};

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};

use procmp::error::{Error, Result};
use procmp::{AbsPath, Graph};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Tick any number, then continue.
    Many,
    /// Choosing a task ends the menu.
    One,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    SelectAll,
    ClearAll,
    Confirm,
    Cancel,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Task(usize),
    Separator,
    Act(Action),
}

enum Key {
    Up,
    Down,
    Choose,
    Quit,
}

/// Restores the terminal however the menu exits, including on an error or a panic.
struct Session;

impl Session {
    fn open() -> Result<Self> {
        terminal::enable_raw_mode().map_err(failed)?;
        io::stderr().queue(cursor::Hide).map_err(failed)?;
        Ok(Self)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let _ = terminal::disable_raw_mode();
        let _ = io::stderr()
            .queue(Clear(ClearType::FromCursorDown))
            .and_then(|out| out.queue(cursor::Show))
            .and_then(Write::flush);
    }
}

/// Chosen indices, or [`None`] when dismissed.
pub fn tasks(graph: &Graph, root: &AbsPath, title: &str, mode: Mode) -> Result<Option<Vec<usize>>> {
    if !io::stdin().is_terminal() || !io::stderr().is_terminal() {
        return Err(Error::NotATerminal);
    }

    let labels: Vec<(String, String)> = graph
        .tasks
        .iter()
        .map(|task| (task.id.clone(), task.output.relative_to(root)))
        .collect();

    let rows = layout(labels.len(), mode);
    let width = labels
        .iter()
        .map(|(id, _)| id.chars().count())
        .max()
        .unwrap_or(0);

    let mut chosen = vec![false; labels.len()];
    let mut at = 0usize;

    let _session = Session::open()?;
    let mut out = io::stderr();

    loop {
        draw(&mut out, title, &labels, &rows, &chosen, at, width)?;

        let Some(key) = keypress()? else { continue };

        match key {
            Key::Quit => return Ok(None),
            Key::Up => at = step(&rows, at, -1),
            Key::Down => at = step(&rows, at, 1),

            Key::Choose => match rows[at] {
                Row::Separator => {}
                Row::Task(index) => match mode {
                    Mode::One => return Ok(Some(vec![index])),
                    Mode::Many => chosen[index] = !chosen[index],
                },
                Row::Act(Action::Cancel) => return Ok(None),
                Row::Act(Action::SelectAll) => chosen.fill(true),
                Row::Act(Action::ClearAll) => chosen.fill(false),

                Row::Act(Action::Confirm) => {
                    let picked = ticked(&chosen);
                    if !picked.is_empty() {
                        return Ok(Some(picked));
                    }
                }
            },
        }
    }
}

/// [`Mode::One`] needs no bulk actions and no confirm.
fn layout(count: usize, mode: Mode) -> Vec<Row> {
    let mut rows: Vec<Row> = (0..count).map(Row::Task).collect();
    rows.push(Row::Separator);

    if mode == Mode::Many {
        rows.push(Row::Act(Action::SelectAll));
        rows.push(Row::Act(Action::ClearAll));
        rows.push(Row::Act(Action::Confirm));
    }
    rows.push(Row::Act(Action::Cancel));

    rows
}

fn ticked(chosen: &[bool]) -> Vec<usize> {
    chosen
        .iter()
        .enumerate()
        .filter(|(_, on)| **on)
        .map(|(index, _)| index)
        .collect()
}

/// Wraps, and skips the separator. Bounded by the row count.
fn step(rows: &[Row], from: usize, delta: isize) -> usize {
    let len = rows.len() as isize;
    let mut at = from as isize;

    for _ in 0..rows.len() {
        at = (at + delta).rem_euclid(len);
        if rows[at as usize] != Row::Separator {
            break;
        }
    }

    at as usize
}

/// Unmapped keys are ignored.
fn keypress() -> Result<Option<Key>> {
    let Event::Key(key) = event::read().map_err(failed)? else {
        return Ok(None);
    };
    if key.kind != KeyEventKind::Press {
        return Ok(None);
    }

    if key.modifiers.contains(KeyModifiers::CONTROL) && key.code == KeyCode::Char('c') {
        return Ok(Some(Key::Quit));
    }

    Ok(match key.code {
        KeyCode::Up | KeyCode::Char('k') => Some(Key::Up),
        KeyCode::Down | KeyCode::Char('j') => Some(Key::Down),
        KeyCode::Enter | KeyCode::Char(' ') => Some(Key::Choose),
        KeyCode::Esc | KeyCode::Char('q') => Some(Key::Quit),
        _ => None,
    })
}

fn draw(
    out: &mut io::Stderr,
    title: &str,
    labels: &[(String, String)],
    rows: &[Row],
    chosen: &[bool],
    at: usize,
    width: usize,
) -> Result<()> {
    let count = chosen.iter().filter(|on| **on).count();

    out.queue(Clear(ClearType::FromCursorDown))
        .map_err(failed)?;
    line(out, "")?;
    line(out, &format!("  {title}"))?;
    line(out, "")?;

    for (index, row) in rows.iter().enumerate() {
        let text = match row {
            Row::Separator => String::new(),
            Row::Task(task) => {
                let (id, output) = &labels[*task];
                let mark = if chosen[*task] { "[x]" } else { "[ ]" };
                format!("{mark} {id:width$}  {output}")
            }
            Row::Act(Action::SelectAll) => "Select all".to_owned(),
            Row::Act(Action::ClearAll) => "Select none".to_owned(),
            Row::Act(Action::Confirm) => format!("Continue with {count} selected"),
            Row::Act(Action::Cancel) => "Cancel".to_owned(),
        };

        let arrow = if index == at { ">" } else { " " };
        if index == at {
            out.queue(SetAttribute(Attribute::Bold)).map_err(failed)?;
        }
        line(out, &format!("  {arrow} {text}"))?;
        if index == at {
            out.queue(SetAttribute(Attribute::Reset)).map_err(failed)?;
        }
    }

    // Back to the top, so the next frame overwrites this one in place.
    out.queue(cursor::MoveToPreviousLine((rows.len() + 3) as u16))
        .map_err(failed)?;
    out.flush().map_err(failed)
}

/// Raw mode does not translate `\n`.
fn line(out: &mut io::Stderr, text: &str) -> Result<()> {
    write!(out, "{text}\r\n").map_err(failed)
}

fn failed(error: impl std::fmt::Display) -> Error {
    Error::Terminal(error.to_string())
}
