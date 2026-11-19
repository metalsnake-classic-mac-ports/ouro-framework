//! The level selection (the ports' rule 3): the same layout in every
//! port. Left a list of the levels - number, name, a tick for the ones
//! done; entries that are not levels (a story sequence) have no number
//! and may be locked. Right the chosen level's start view and what the
//! port says about it (best, first and latest run, deaths).
//!
//! The port gives the rows, draws the preview into the rectangle it is
//! given and supplies the lines under it; Return gives the row back.

use crate::{Ink, Key, Mark, Rect, Role, Theme, fit};

/// A row of the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Row {
    /// The level's number; `None` for other entries (a story sequence)
    pub number: Option<i32>,
    pub name: String,
    pub done: bool,
    /// Shown dim; Return still gives it back (the port says why it is
    /// locked in the lines under the preview)
    pub locked: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Take(usize),
    Back,
}

const ROW: i32 = 18;
const BASE: i32 = 13;
const MARGIN: i32 = 16;
const GAP: i32 = 12;

/// Where things go on a screen of a given size.
pub struct Layout {
    /// The list's box
    pub list: Rect,
    /// The rows inside it
    pub rows: Rect,
    /// The chosen level's start view (4 : 3)
    pub preview: Rect,
    /// The baseline of the first line under it
    pub details: i32,
}

pub fn layout(screen: Rect) -> Layout {
    let body = Rect::new(screen.left + MARGIN, screen.top + MARGIN + 24, screen.right - MARGIN, screen.bottom - MARGIN - 14);
    let list_width = (body.width() - GAP) * 2 / 5;
    let list = Rect::new(body.left, body.top, body.left + list_width, body.bottom);
    let rows = list.inset(6);
    let right = Rect::new(list.right + GAP, body.top, body.right, body.bottom);
    // a multiple of 3, so the width is exactly 4 / 3 of it
    let height = (right.width() * 3 / 4).min(right.height() - 5 * 16) / 3 * 3;
    let preview = Rect::new(right.left, right.top, right.left + height * 4 / 3, right.top + height);
    Layout { list, rows, preview, details: preview.bottom + 20 }
}

fn visible(l: &Layout) -> usize {
    (l.rows.height() / ROW).max(1) as usize
}

#[derive(Debug, Default)]
pub struct Levels {
    pub selected: usize,
    /// The first row shown
    first: usize,
}

impl Levels {
    /// Choose row `k` (the level played last), shown in the list.
    pub fn select(&mut self, k: usize, screen: Rect) {
        self.selected = k;
        self.follow(visible(&layout(screen)));
    }

    fn follow(&mut self, page: usize) {
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected >= self.first + page {
            self.first = self.selected + 1 - page;
        }
    }

    pub fn key(&mut self, key: Key, rows: usize, screen: Rect) -> Option<Choice> {
        if rows == 0 {
            return (key == Key::Back).then_some(Choice::Back);
        }
        let page = visible(&layout(screen));
        let k = self.selected;
        self.selected = match key {
            Key::Up => (k + rows - 1) % rows,
            Key::Down => (k + 1) % rows,
            Key::PageUp | Key::Left => k.saturating_sub(page),
            Key::PageDown | Key::Right => (k + page).min(rows - 1),
            Key::Home => 0,
            Key::End => rows - 1,
            Key::Accept | Key::Toggle => return Some(Choice::Take(k)),
            Key::Back => return Some(Choice::Back),
        };
        self.follow(page);
        None
    }

    pub fn mouse(&mut self, x: i32, y: i32, click: bool, rows: usize, screen: Rect) -> Option<Choice> {
        let l = layout(screen);
        if !l.rows.contains(x, y) {
            return None;
        }
        let k = self.first + ((y - l.rows.top) / ROW) as usize;
        if k >= rows || (y - l.rows.top) / ROW >= visible(&l) as i32 {
            return None;
        }
        self.selected = k;
        click.then_some(Choice::Take(k))
    }

    /// The wheel scrolls the list (positive: up).
    pub fn wheel(&mut self, dy: i32, rows: usize, screen: Rect) {
        let page = visible(&layout(screen));
        let max = rows.saturating_sub(page) as i32;
        self.first = (self.first as i32 - dy).clamp(0, max) as usize;
    }

    /// The screen: `preview` draws the chosen row's start view into its
    /// rectangle; `details` are the lines under it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<T: Theme>(
        &self,
        theme: &T,
        surface: &mut T::Surface,
        screen: Rect,
        heading: &str,
        rows: &[Row],
        details: &[(String, Ink)],
        preview: impl FnOnce(&mut T::Surface, Rect),
    ) -> Vec<T::Text> {
        let l = layout(screen);
        let mut texts = Vec::new();
        theme.backdrop(surface, screen);
        texts.push(theme.text(l.list.left as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, heading));
        theme.panel(surface, l.list);
        let page = visible(&l);
        // numbers right-aligned in a column as wide as the largest
        let widest = rows.iter().filter_map(|r| r.number).max().unwrap_or(0);
        let number_room = theme.width(Role::Row, &widest.to_string());
        let name_x = l.rows.left as f32 + 6.0 + number_room + 22.0;
        let room = l.rows.right as f32 - name_x - 14.0;
        for (k, row) in rows.iter().enumerate().skip(self.first).take(page) {
            let top = l.rows.top + (k - self.first) as i32 * ROW;
            let rect = Rect::new(l.rows.left, top, l.rows.right, top + ROW);
            let chosen = k == self.selected;
            if chosen {
                theme.highlight(surface, rect);
            }
            let ink = if chosen {
                Ink::Chosen
            } else if row.locked {
                Ink::Dim
            } else {
                Ink::Normal
            };
            let base = (top + BASE) as f32;
            if let Some(n) = row.number {
                let text = n.to_string();
                let w = theme.width(Role::Row, &text);
                texts.push(theme.text(l.rows.left as f32 + 6.0 + number_room - w, base, Role::Row, ink, &text));
            }
            if row.done {
                theme.mark(surface, (name_x - 16.0) as i32, top + 4, Mark::Tick, if chosen { Ink::Chosen } else { Ink::Accent });
            }
            texts.push(theme.text(name_x, base, Role::Row, ink, &fit(theme, Role::Row, &row.name, room)));
        }
        if self.first > 0 {
            theme.mark(surface, l.list.right - 16, l.list.top + 2, Mark::MoreUp, Ink::Dim);
        }
        if self.first + page < rows.len() {
            theme.mark(surface, l.list.right - 16, l.list.bottom - 9, Mark::MoreDown, Ink::Dim);
        }
        theme.panel(surface, Rect::new(l.preview.left - 2, l.preview.top - 2, l.preview.right + 2, l.preview.bottom + 2));
        preview(surface, l.preview);
        let room = (screen.right - MARGIN - l.preview.left) as f32;
        for (k, (line, ink)) in details.iter().enumerate() {
            texts.push(theme.text(l.preview.left as f32, (l.details + 16 * k as i32) as f32, Role::Small, *ink, &fit(theme, Role::Small, line, room)));
        }
        texts.push(theme.text(l.list.left as f32, (screen.bottom - 8) as f32, Role::Hint, Ink::Dim, "↑ ↓ choose   ← →: a page   Return: play   Esc: back"));
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    fn rows(n: i32) -> Vec<Row> {
        (1..=n).map(|k| Row { number: Some(k), name: format!("Level {k}"), done: k % 3 == 0, locked: false }).collect()
    }

    #[test]
    fn keys_scroll_the_list() {
        let r = rows(100);
        let mut s = Levels::default();
        let page = visible(&layout(SCREEN));
        for _ in 0..page {
            s.key(Key::Down, r.len(), SCREEN);
        }
        assert_eq!(s.selected, page);
        assert_eq!(s.first, 1, "the list follows the selection");
        s.key(Key::End, r.len(), SCREEN);
        assert_eq!((s.selected, s.first), (99, 100 - page));
        s.key(Key::Down, r.len(), SCREEN);
        assert_eq!((s.selected, s.first), (0, 0), "round the end");
        assert_eq!(s.key(Key::Accept, r.len(), SCREEN), Some(Choice::Take(0)));
    }

    #[test]
    fn mouse_takes_the_row_drawn_there() {
        let r = rows(100);
        let mut s = Levels::default();
        s.select(40, SCREEN);
        let mut rec = Record::default();
        let texts = s.draw(&Plain, &mut rec, SCREEN, "Levels", &r, &[], |_, _| {});
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Level 38").unwrap().clone();
        assert_eq!(s.mouse(x as i32, base as i32 - 4, true, r.len(), SCREEN), Some(Choice::Take(37)));
        // the preview keeps the shape of the screen
        let l = layout(SCREEN);
        assert_eq!(l.preview.width() * 3, l.preview.height() * 4);
    }
}
