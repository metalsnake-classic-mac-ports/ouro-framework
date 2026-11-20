//! The options page (the ports' rule 9: the original's settings dialogs
//! become a modern page, the same in every port): a list of the game's
//! settings with their values, then the port's other rows (the keys page,
//! Defaults), then Back. Left and Right change the chosen setting, Return,
//! Space and a click step it on; a row without a value is taken. The port
//! keeps the settings, applies a change at once and saves it. Under the list
//! a line about the chosen row and the port's notes, as on the keys page.

use crate::{Ink, Key, Rect, Role, Theme, fit};

/// A row of the page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    pub label: String,
    /// The setting's value now ("On", "Whole multiples"); `None`: a row
    /// that is taken (opens a page, resets)
    pub value: Option<String>,
    /// What the row does: shown under the list when chosen
    pub about: String,
}

impl Row {
    /// A setting with its value.
    pub fn setting(label: impl Into<String>, value: impl Into<String>, about: impl Into<String>) -> Row {
        Row { label: label.into(), value: Some(value.into()), about: about.into() }
    }

    /// A row that is taken.
    pub fn action(label: impl Into<String>, about: impl Into<String>) -> Row {
        Row { label: label.into(), value: None, about: about.into() }
    }
}

/// "On" or "Off".
pub fn on_off(on: bool) -> &'static str {
    if on { "On" } else { "Off" }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Step the setting of this row: -1 back, 1 on
    Change(usize, i32),
    /// A row without a value was taken
    Take(usize),
    /// Escape or the Back row
    Back,
}

const ROW: i32 = 20;
const BASE: i32 = 14;
const MARGIN: i32 = 16;

#[derive(Debug, Default)]
pub struct Options {
    pub selected: usize,
}

/// The box as on the keys page: as wide as needed, at most the screen less
/// the margins, centred; the rows, then room for the lines under them.
fn panel(screen: Rect, rows: usize, notes: usize) -> Rect {
    let width = (screen.width() - 2 * MARGIN).min(520);
    let height = 16 + rows as i32 * ROW + 20 + (1 + notes as i32) * 16;
    let left = screen.left + (screen.width() - width) / 2;
    let top = screen.top + MARGIN + 30;
    Rect::new(left, top, left + width, (top + height).min(screen.bottom - MARGIN - 14))
}

fn row_rect(p: Rect, k: usize) -> Rect {
    let top = p.top + 8 + k as i32 * ROW;
    Rect::new(p.left + 4, top, p.right - 4, top + ROW)
}

impl Options {
    /// Return, Space or a click on row `k` (`rows.len()`: Back).
    fn take(&self, rows: &[Row]) -> Choice {
        match rows.get(self.selected) {
            Some(Row { value: Some(_), .. }) => Choice::Change(self.selected, 1),
            Some(_) => Choice::Take(self.selected),
            None => Choice::Back,
        }
    }

    pub fn key(&mut self, key: Key, rows: &[Row]) -> Option<Choice> {
        let count = rows.len() + 1;
        self.selected = self.selected.min(count - 1);
        let setting = rows.get(self.selected).is_some_and(|r| r.value.is_some());
        match key {
            Key::Up => self.selected = (self.selected + count - 1) % count,
            Key::Down => self.selected = (self.selected + 1) % count,
            Key::Home => self.selected = 0,
            Key::End => self.selected = count - 1,
            Key::Left if setting => return Some(Choice::Change(self.selected, -1)),
            Key::Right if setting => return Some(Choice::Change(self.selected, 1)),
            Key::Accept | Key::Toggle => return Some(self.take(rows)),
            Key::Back => return Some(Choice::Back),
            _ => {}
        }
        None
    }

    /// The mouse at (x, y): the row under it is chosen, a click takes it.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, rows: &[Row], notes: usize, screen: Rect) -> Option<Choice> {
        let p = panel(screen, rows.len() + 1, notes);
        let k = (0..=rows.len()).find(|&k| row_rect(p, k).contains(x, y))?;
        self.selected = k;
        click.then(|| self.take(rows))
    }

    /// The page: the values in a column after the longest label; the
    /// chosen setting's value between arrows.
    pub fn draw<T: Theme>(&self, theme: &T, surface: &mut T::Surface, screen: Rect, heading: &str, rows: &[Row], notes: &[&str]) -> Vec<T::Text> {
        let mut texts = Vec::new();
        theme.backdrop(surface, screen);
        let count = rows.len() + 1;
        let p = panel(screen, count, notes.len());
        texts.push(theme.text(p.left as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, heading));
        theme.panel(surface, p);
        let label_w = rows.iter().filter(|r| r.value.is_some()).map(|r| theme.width(Role::Row, &r.label)).fold(0.0, f32::max);
        let value_x = p.left as f32 + 16.0 + label_w + 24.0;
        let room = p.right as f32 - 16.0 - value_x;
        for k in 0..count {
            let r = row_rect(p, k);
            let chosen = k == self.selected;
            if chosen {
                theme.highlight(surface, r);
            }
            let ink = if chosen { Ink::Chosen } else { Ink::Normal };
            let base = (r.top + BASE) as f32;
            let (label, value) = match rows.get(k) {
                Some(row) => (row.label.as_str(), row.value.as_deref()),
                None => ("Back", None),
            };
            texts.push(theme.text(p.left as f32 + 16.0, base, Role::Row, ink, label));
            if let Some(v) = value {
                let shown = if chosen { format!("<  {v}  >") } else { v.to_string() };
                texts.push(theme.text(value_x, base, Role::Row, ink, &fit(theme, Role::Row, &shown, room)));
            }
        }
        let below = row_rect(p, count).top + 4;
        theme.rule(surface, Rect::new(p.left + 12, below, p.right - 12, below + 1));
        let about = rows.get(self.selected).map_or("Back to the menu", |r| r.about.as_str());
        let room = (p.width() - 32) as f32;
        for (k, (line, ink)) in std::iter::once((about, Ink::Normal)).chain(notes.iter().map(|n| (*n, Ink::Dim))).enumerate() {
            texts.push(theme.text(p.left as f32 + 16.0, (below + 18 + 16 * k as i32) as f32, Role::Small, ink, &fit(theme, Role::Small, line, room)));
        }
        texts.push(theme.text(p.left as f32, (screen.bottom - 8) as f32, Role::Hint, Ink::Dim, "↑ ↓ choose   ← → change   Return: take   Esc: back"));
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    fn rows() -> Vec<Row> {
        vec![
            Row::setting("Sound", "On", "The game's sounds"),
            Row::setting("Scaling", "Whole multiples", "Sharp pixels, or fill the window"),
            Row::action("Keys...", "The game's keys and pad buttons"),
        ]
    }

    #[test]
    fn keys_change_settings_and_take_rows() {
        let r = rows();
        let mut o = Options::default();
        assert_eq!(o.key(Key::Right, &r), Some(Choice::Change(0, 1)));
        assert_eq!(o.key(Key::Left, &r), Some(Choice::Change(0, -1)));
        assert_eq!(o.key(Key::Accept, &r), Some(Choice::Change(0, 1)), "Return steps a setting on");
        o.key(Key::Down, &r);
        o.key(Key::Down, &r);
        assert_eq!(o.key(Key::Right, &r), None, "a row without a value has nothing to change");
        assert_eq!(o.key(Key::Accept, &r), Some(Choice::Take(2)));
        o.key(Key::Down, &r);
        assert_eq!(o.key(Key::Accept, &r), Some(Choice::Back), "the Back row");
        o.key(Key::Down, &r);
        assert_eq!(o.selected, 0, "round the end");
        assert_eq!(o.key(Key::Back, &r), Some(Choice::Back));
    }

    #[test]
    fn mouse_takes_the_row_drawn_there() {
        let r = rows();
        let mut o = Options::default();
        let mut rec = Record::default();
        let texts = o.draw(&Plain, &mut rec, SCREEN, "Options", &r, &["note"]);
        assert!(texts.iter().any(|t| t.3 == "<  On  >"), "the chosen value between arrows");
        assert!(texts.iter().any(|t| t.3 == "The game's sounds"), "the chosen row's line");
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Keys...").unwrap().clone();
        assert_eq!(o.mouse(x as i32, base as i32 - 4, true, &r, 1, SCREEN), Some(Choice::Take(2)));
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Whole multiples").unwrap().clone();
        assert_eq!(o.mouse(x as i32, base as i32 - 4, true, &r, 1, SCREEN), Some(Choice::Change(1, 1)));
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Back").unwrap().clone();
        assert_eq!(o.mouse(x as i32, base as i32 - 4, false, &r, 1, SCREEN), None);
        assert_eq!(o.selected, 3);
    }
}
