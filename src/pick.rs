//! An opt-in menu for choosing tasks.
//!
//! Reachable only through `--pick`, so a script or a CI job that never passes the flag
//! behaves exactly as it did before. A build that blocks on a prompt cannot run
//! unattended, and one that asks for a version at build time produces different bytes
//! from the same commit, which is what makes this a menu rather than a wizard: it
//! chooses among tasks the manifest already declares and changes nothing else.
//!
//! Every action is a row. There is no legend to read and no key to know beyond the
//! arrows and enter, so selecting all is something you can see rather than something
//! you have to be told.
//!
//! Drawn on stderr, so `pcmp build --pick --json | jq` still works.

use std::io::{self, IsTerminal, Write};

use crossterm::{
    QueueableCommand, cursor,
    event::{self, Event, KeyCode, KeyEventKind, KeyModifiers},
    style::{Attribute, SetAttribute},
    terminal::{self, Clear, ClearType},
};

use procmp::error::{Error, Result};
use procmp::{AbsPath, Graph};

/// Chosen task indices, or [`None`] when the menu was dismissed.
pub type Picked = Option<Vec<usize>>;

/// How many tasks a menu may return.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// `build` and `watch`: tick any number, then continue.
    Many,
    /// `explain`: choosing a task ends the menu.
    One,
}

/// A row that does something when chosen, as opposed to a task that toggles.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Action {
    SelectAll,
    ClearAll,
    Confirm,
    Cancel,
}

/// One line of the menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Row {
    Task(usize),
    Separator,
    Act(Action),
}

/// Restores the terminal however the menu exits, including on an error or a panic.
struct Session;

impl Session {
    fn open() -> Result<Self> {
        terminal::enable_raw_mode().map_err(terminal_error)?;
        io::stderr().queue(cursor::Hide).map_err(terminal_error)?;
        Ok(Self)
    }
}

impl Drop for Session {
    fn drop(&mut self) {
        let mut out = io::stderr();
        let _ = terminal::disable_raw_mode();
        let _ = out
            .queue(Clear(ClearType::FromCursorDown))
            .and_then(|out| out.queue(cursor::Show))
            .and_then(Write::flush);
    }
}

/// Presents `graph` as a menu titled `title` and returns the chosen indices.
///
/// # Errors
///
/// When either end of the terminal is redirected, because a menu with nothing to draw
/// on and nobody to read it is a hang rather than a question.
pub fn tasks(graph: &Graph, root: &AbsPath, title: &str, mode: Mode) -> Result<Picked> {
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

        let Some(key) = keypress()? else {
            continue;
        };

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

                // Confirming nothing would build nothing, which reads as a hang rather
                // than a choice, so the row simply does not fire.
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

/// The rows a menu of `count` tasks is built from.
///
/// [`Mode::One`] needs no bulk actions and no confirm, because choosing a task is the
/// confirmation, so it gets one action rather than four.
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

/// Moves the cursor by `delta`, wrapping and skipping the separator.
fn step(rows: &[Row], from: usize, delta: isize) -> usize {
    let len = rows.len() as isize;
    let mut at = from as isize;

    // Bounded by the row count: a menu always holds at least one action, so a full lap
    // cannot run out of landing places.
    for _ in 0..rows.len() {
        at = (at + delta).rem_euclid(len);
        if rows[at as usize] != Row::Separator {
            break;
        }
    }

    at as usize
}

/// The four things a menu reacts to. Everything else is ignored rather than guessed at.
enum Key {
    Up,
    Down,
    Choose,
    Quit,
}

fn keypress() -> Result<Option<Key>> {
    let Event::Key(key) = event::read().map_err(terminal_error)? else {
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
        .map_err(terminal_error)?;
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
            Row::Act(action) => match action {
                Action::SelectAll => "Select all".to_owned(),
                Action::ClearAll => "Select none".to_owned(),
                Action::Confirm => format!("Continue with {count} selected"),
                Action::Cancel => "Cancel".to_owned(),
            },
        };

        let arrow = if index == at { ">" } else { " " };
        if index == at {
            out.queue(SetAttribute(Attribute::Bold))
                .map_err(terminal_error)?;
        }
        line(out, &format!("  {arrow} {text}"))?;
        if index == at {
            out.queue(SetAttribute(Attribute::Reset))
                .map_err(terminal_error)?;
        }
    }

    // Back to the top so the next frame overwrites this one in place.
    out.queue(cursor::MoveToPreviousLine((rows.len() + 3) as u16))
        .map_err(terminal_error)?;
    out.flush().map_err(terminal_error)
}

/// Raw mode does not translate `\n`, so every line needs an explicit return.
fn line(out: &mut io::Stderr, text: &str) -> Result<()> {
    write!(out, "{text}\r\n").map_err(terminal_error)
}

fn terminal_error(error: impl std::fmt::Display) -> Error {
    Error::Terminal(error.to_string())
}
