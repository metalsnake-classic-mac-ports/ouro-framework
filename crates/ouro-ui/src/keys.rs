//! The keys page (the ports' rule 9; S.C.Out's page is the model): a
//! list of the game's actions with their key and gamepad button, then
//! Defaults and Back. Return on an action waits for the new key or
//! button; the port takes it (a key in use is swapped, saved at once) and
//! ends the wait with [`Keys::done`]. Under the list a line about the
//! chosen action and the port's notes (how the mouse or the sticks work).

use crate::{Ink, Key, Rect, Role, Theme, fit};

/// An action and what it is bound to now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binding {
    pub action: String,
    /// The key's name ("A", "Space"; "Mouse button" for one that has none)
    pub key: String,
    /// The pad's input ("A", "RT")
    pub pad: String,
    /// What the action does: shown under the list when chosen
    pub about: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Wait for the new key or button of this action
    Change(usize),
    Defaults,
    Back,
}

const ROW: i32 = 20;
const BASE: i32 = 14;
const MARGIN: i32 = 16;

#[derive(Debug, Default)]
pub struct Keys {
    pub selected: usize,
    /// Waiting for the new key or button of the chosen action
    pub waiting: bool,
}

/// The box: as wide as needed, at most the screen less the margins,
/// centred; the rows, then room for the lines under them.
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

impl Keys {
    /// The rows: the actions, Defaults, Back.
    fn take(&mut self, actions: usize) -> Option<Choice> {
        match self.selected {
            k if k < actions => {
                self.waiting = true;
                Some(Choice::Change(k))
            }
            k if k == actions => Some(Choice::Defaults),
            _ => Some(Choice::Back),
        }
    }

    /// A key while not waiting (while waiting the port reads the keys).
    pub fn key(&mut self, key: Key, actions: usize) -> Option<Choice> {
        let rows = actions + 2;
        match key {
            Key::Up => self.selected = (self.selected + rows - 1) % rows,
            Key::Down => self.selected = (self.selected + 1) % rows,
            Key::Home => self.selected = 0,
            Key::End => self.selected = rows - 1,
            Key::Accept | Key::Toggle => return self.take(actions),
            Key::Back => return Some(Choice::Back),
            _ => {}
        }
        None
    }

    /// The new key or button was taken, or the wait cancelled.
    pub fn done(&mut self) {
        self.waiting = false;
    }

    pub fn mouse(&mut self, x: i32, y: i32, click: bool, actions: usize, notes: usize, screen: Rect) -> Option<Choice> {
        if self.waiting {
            return None;
        }
        let p = panel(screen, actions + 2, notes);
        let k = (0..actions + 2).find(|&k| row_rect(p, k).contains(x, y))?;
        self.selected = k;
        if click { self.take(actions) } else { None }
    }

    /// The page: the key and pad columns placed after the longest names.
    pub fn draw<T: Theme>(&self, theme: &T, surface: &mut T::Surface, screen: Rect, bindings: &[Binding], notes: &[&str]) -> Vec<T::Text> {
        let mut texts = Vec::new();
        theme.backdrop(surface, screen);
        let rows = bindings.len() + 2;
        let p = panel(screen, rows, notes.len());
        texts.push(theme.text(p.left as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, "Keys"));
        theme.panel(surface, p);
        // the columns: after the longest action, then after the longest key
        let action_w = bindings.iter().map(|b| theme.width(Role::Row, &b.action)).fold(theme.width(Role::Row, "Defaults"), f32::max);
        let key_x = p.left as f32 + 16.0 + action_w + 24.0;
        let key_w = bindings.iter().map(|b| theme.width(Role::Row, &b.key)).fold(theme.width(Role::Row, "Key"), f32::max);
        let pad_x = key_x + key_w + 24.0;
        let head = (p.top - 6) as f32;
        texts.push(theme.text(key_x, head, Role::Small, Ink::Dim, "Key"));
        // a game without gamepad bindings (yet) shows no column for them
        if bindings.iter().any(|b| !b.pad.is_empty()) {
            texts.push(theme.text(pad_x, head, Role::Small, Ink::Dim, "Gamepad"));
        }
        for k in 0..rows {
            let r = row_rect(p, k);
            let chosen = k == self.selected;
            if chosen {
                theme.highlight(surface, r);
            }
            let ink = if chosen { Ink::Chosen } else { Ink::Normal };
            let base = (r.top + BASE) as f32;
            match bindings.get(k) {
                Some(b) => {
                    texts.push(theme.text(p.left as f32 + 16.0, base, Role::Row, ink, &b.action));
                    if self.waiting && chosen {
                        texts.push(theme.text(key_x, base, Role::Row, ink, "press a key or a pad button..."));
                    } else {
                        texts.push(theme.text(key_x, base, Role::Row, ink, &b.key));
                        texts.push(theme.text(pad_x, base, Role::Row, ink, &b.pad));
                    }
                }
                None => texts.push(theme.text(p.left as f32 + 16.0, base, Role::Row, ink, if k == bindings.len() { "Defaults" } else { "Back" })),
            }
        }
        let below = row_rect(p, rows).top + 4;
        theme.rule(surface, Rect::new(p.left + 12, below, p.right - 12, below + 1));
        let about = match bindings.get(self.selected) {
            Some(b) => b.about.as_str(),
            None if self.selected == bindings.len() => "The original's keys and the usual pad buttons",
            None => "Back to the menu",
        };
        let room = (p.width() - 32) as f32;
        for (k, (line, ink)) in std::iter::once((about, Ink::Normal)).chain(notes.iter().map(|n| (*n, Ink::Dim))).enumerate() {
            texts.push(theme.text(p.left as f32 + 16.0, (below + 18 + 16 * k as i32) as f32, Role::Small, ink, &fit(theme, Role::Small, line, room)));
        }
        let hint = if self.waiting { "Press the new key or button   Esc / Start: keep the old one" } else { "↑ ↓ choose   Return: change   Esc: back" };
        texts.push(theme.text(p.left as f32, (screen.bottom - 8) as f32, Role::Hint, Ink::Dim, hint));
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    fn bindings() -> Vec<Binding> {
        ["Left", "Right", "Jump"].iter().map(|a| Binding { action: a.to_string(), key: "A".into(), pad: "B".into(), about: format!("{a}!") }).collect()
    }

    #[test]
    fn rows_and_waiting() {
        let b = bindings();
        let mut k = Keys::default();
        k.key(Key::Down, b.len());
        assert_eq!(k.key(Key::Accept, b.len()), Some(Choice::Change(1)));
        assert!(k.waiting);
        assert_eq!(k.mouse(100, 100, true, b.len(), 1, SCREEN), None, "no clicks while waiting");
        k.done();
        k.key(Key::End, b.len());
        assert_eq!(k.key(Key::Accept, b.len()), Some(Choice::Back));
        k.key(Key::Up, b.len());
        assert_eq!(k.key(Key::Accept, b.len()), Some(Choice::Defaults));
    }

    #[test]
    fn mouse_takes_the_row_drawn_there() {
        let b = bindings();
        let mut k = Keys::default();
        let mut rec = Record::default();
        let texts = k.draw(&Plain, &mut rec, SCREEN, &b, &["note"]);
        let (x, base, _, _) = texts.iter().find(|t| t.3 == "Jump").unwrap().clone();
        assert_eq!(k.mouse(x as i32, base as i32 - 4, true, b.len(), 1, SCREEN), Some(Choice::Change(2)));
        assert!(texts.iter().any(|t| t.3 == "Left!"), "the chosen action's line");
    }
}
