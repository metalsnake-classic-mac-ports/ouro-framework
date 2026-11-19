//! A game's main menu: a column of entries in a box (the ports' rules 3
//! and 4: Continue or Resume first, New Game, Levels, the keys page,
//! the game's own entries, Quit). The port names the entries and acts on
//! the one taken; this screen moves the selection and lays the box out
//! in the rectangle the port gives (Sammy: its title's box, Quagmire:
//! under the title's rail).

use crate::{Ink, Key, Rect, Role, Theme, fit};

/// An entry of the menu.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub label: String,
    /// Shown dim and skipped by the arrows (not available now)
    pub off: bool,
}

impl Entry {
    pub fn new(label: impl Into<String>) -> Entry {
        Entry { label: label.into(), off: false }
    }
}

/// What the player did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// The entry with this index was taken (Return, a click)
    Take(usize),
    /// Escape
    Back,
}

const ROW: i32 = 20;
const BASE: i32 = 14;
const PAD: i32 = 10;

#[derive(Debug, Default)]
pub struct Menu {
    pub selected: usize,
}

/// The rows' rectangles in `area`, with room for a heading when there
/// is one: the column centred vertically.
fn rows(area: Rect, count: usize, heading: bool) -> Vec<Rect> {
    let head = if heading { ROW + 6 } else { 0 };
    let height = head + count as i32 * ROW;
    let top = area.top + ((area.height() - height) / 2).max(PAD) + head;
    (0..count as i32).map(|k| Rect::new(area.left + PAD, top + k * ROW, area.right - PAD, top + (k + 1) * ROW)).collect()
}

impl Menu {
    /// Choose `entry` (as when the menu opens on Continue).
    pub fn select(&mut self, entry: usize) {
        self.selected = entry;
    }

    /// The next entry from the chosen one in `step`'s direction that is
    /// not off (round the end).
    fn moved(&self, entries: &[Entry], step: isize) -> usize {
        let n = entries.len() as isize;
        let mut k = self.selected as isize;
        for _ in 0..n {
            k = (k + step).rem_euclid(n);
            if !entries[k as usize].off {
                return k as usize;
            }
        }
        self.selected
    }

    pub fn key(&mut self, key: Key, entries: &[Entry]) -> Option<Choice> {
        if entries.is_empty() {
            return (key == Key::Back).then_some(Choice::Back);
        }
        self.selected = self.selected.min(entries.len() - 1);
        match key {
            Key::Up => self.selected = self.moved(entries, -1),
            Key::Down => self.selected = self.moved(entries, 1),
            Key::Home => self.selected = 0,
            Key::End => self.selected = entries.len() - 1,
            Key::Accept | Key::Toggle if !entries[self.selected].off => return Some(Choice::Take(self.selected)),
            Key::Back => return Some(Choice::Back),
            _ => {}
        }
        None
    }

    /// The mouse at (x, y): the entry under it is chosen, a click takes it.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, entries: &[Entry], area: Rect, heading: bool) -> Option<Choice> {
        let k = rows(area, entries.len(), heading).iter().position(|r| r.contains(x, y))?;
        if entries[k].off {
            return None;
        }
        self.selected = k;
        click.then_some(Choice::Take(k))
    }

    /// The box over all of `area` (the port draws what is behind it
    /// first), the heading and the entries centred.
    pub fn draw<T: Theme>(&self, theme: &T, surface: &mut T::Surface, area: Rect, heading: Option<&str>, entries: &[Entry]) -> Vec<T::Text> {
        let mut texts = Vec::new();
        theme.panel(surface, area);
        let rects = rows(area, entries.len(), heading.is_some());
        let room = (area.width() - 2 * PAD - 8) as f32;
        if let (Some(h), Some(first)) = (heading, rects.first()) {
            let w = theme.width(Role::Heading, h);
            texts.push(theme.text((area.left + area.right) as f32 / 2.0 - w / 2.0, (first.top - 10) as f32, Role::Heading, Ink::Accent, h));
        }
        for (k, (entry, rect)) in entries.iter().zip(&rects).enumerate() {
            let chosen = k == self.selected;
            if chosen {
                theme.highlight(surface, *rect);
            }
            let ink = if chosen {
                Ink::Chosen
            } else if entry.off {
                Ink::Dim
            } else {
                Ink::Normal
            };
            let label = fit(theme, Role::Row, &entry.label, room);
            let w = theme.width(Role::Row, &label);
            texts.push(theme.text((rect.left + rect.right) as f32 / 2.0 - w / 2.0, (rect.top + BASE) as f32, Role::Row, ink, &label));
        }
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const AREA: Rect = Rect::new(192, 191, 458, 443);

    fn entries() -> Vec<Entry> {
        vec![Entry::new("New Game"), Entry { label: "Continue".into(), off: true }, Entry::new("Levels"), Entry::new("Quit")]
    }

    #[test]
    fn arrows_skip_entries_that_are_off() {
        let e = entries();
        let mut m = Menu::default();
        assert_eq!(m.key(Key::Down, &e), None);
        assert_eq!(m.selected, 2);
        m.key(Key::Down, &e);
        m.key(Key::Down, &e);
        assert_eq!(m.selected, 0, "round the end");
        m.key(Key::Up, &e);
        assert_eq!(m.selected, 3);
        assert_eq!(m.key(Key::Accept, &e), Some(Choice::Take(3)));
        assert_eq!(m.key(Key::Back, &e), Some(Choice::Back));
    }

    #[test]
    fn mouse_and_drawing_agree() {
        let e = entries();
        let mut m = Menu::default();
        let mut s = Record::default();
        let texts = m.draw(&Plain, &mut s, AREA, Some("Sammy"), &e);
        // the text of "Levels" lies in the row the mouse takes it from
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Levels").unwrap().clone();
        assert_eq!(m.mouse(x as i32 + 2, base as i32 - 4, true, &e, AREA, true), Some(Choice::Take(2)));
        // an entry that is off is not taken
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Continue").unwrap().clone();
        assert_eq!(m.mouse(x as i32 + 2, base as i32 - 4, true, &e, AREA, true), None);
        assert_eq!(m.selected, 2);
    }
}
