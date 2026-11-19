//! The game library: the first screen of a port. A list of the games the
//! program plays (Ouro-Quagmire: Quagmire and Sammy the Cyclebot), each
//! with the fan datasets installed for it, then "Add from a file..." and
//! "Get packs online...". The right side shows the chosen game: a preview
//! (the saved moment, else its title), what the port says about it
//! (levels done, the saved game), and its buttons: Continue (or Play) and
//! Title and menu.
//!
//! The selection starts on the game played last, so Return there is
//! Continue. The screen draws in the style of the chosen game: the port
//! passes the theme of [`Library::game`] to [`Library::draw`].

use crate::{Ink, Key, Mark, Rect, Role, Theme, fit};

/// A game the program plays.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Game {
    /// The port's name for it (in the settings: the game played last)
    pub id: String,
    pub name: String,
    /// Lines about it under the preview ("50 levels, 31 done", "Level 12,
    /// saved today 14:32")
    pub about: Vec<String>,
    /// A game is saved: the first button is Continue, else Play
    pub saved: bool,
    /// The game has a title and menu (the second button)
    pub menu: bool,
    pub datasets: Vec<Dataset>,
}

/// A fan dataset installed for a game (rule 8 of the ports): loaded over
/// the game's pack when on.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dataset {
    /// The port's name for it (its file)
    pub id: String,
    pub name: String,
    pub on: bool,
}

/// What the player chose.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// Continue the saved game, or play when none is saved
    Play(String),
    /// The game's title and menu
    Menu(String),
    /// A dataset switched on or off: (game, dataset); the library shows
    /// the new state already
    Toggle(String, String, bool),
    /// "Add from a file...": the port opens the file browser
    Add,
    /// "Get packs online..."
    Online,
    /// Escape: leave the program
    Quit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Row {
    Game(usize),
    Dataset(usize, usize),
    Add,
    Online,
}

pub struct Library {
    pub games: Vec<Game>,
    /// Packs can be loaded from the server (else its row is dim)
    pub online: bool,
    /// A line in place of the keys at the bottom ("Added: ...")
    pub message: Option<String>,
    selected: usize,
    /// The first list row shown
    first: usize,
    /// On a game's row: the second button (Title and menu) is chosen
    second: bool,
    /// The game the screen shows (the last game row chosen)
    shown: usize,
}

/// Rows are this high; texts sit on a baseline this far down.
const ROW: i32 = 18;
const BASE: i32 = 13;
const MARGIN: i32 = 16;
const GAP: i32 = 12;
const BUTTON: i32 = 26;

/// Where things go on a screen of a given size.
struct Layout {
    screen: Rect,
    /// The list's box
    list: Rect,
    /// The rows of games and datasets inside it
    rows: Rect,
    /// The rows under them: Add, Online
    tools: Rect,
    preview: Rect,
    /// The first line about the game (baseline)
    about: i32,
    buttons: i32,
}

fn layout(screen: Rect) -> Layout {
    let body = Rect::new(screen.left + MARGIN, screen.top + MARGIN + 24, screen.right - MARGIN, screen.bottom - MARGIN - 14);
    let list_width = (body.width() - GAP) * 2 / 5;
    let list = Rect::new(body.left, body.top, body.left + list_width, body.bottom);
    let tools = Rect::new(list.left, list.bottom - 2 * ROW - 8, list.right, list.bottom - 4);
    let rows = Rect::new(list.left, list.top + 4, list.right, tools.top - 6);
    let right = Rect::new(list.right + GAP, body.top, body.right, body.bottom);
    let preview_height = (right.width() * 3 / 4).min(right.height() - BUTTON - 3 * 16 - 16);
    let preview = Rect::new(right.left, right.top, right.right, right.top + preview_height);
    Layout { screen, list, rows, tools, preview, about: preview.bottom + 18, buttons: right.bottom - BUTTON }
}

impl Library {
    /// The library with the selection on the game `last` (by id), else the
    /// first.
    pub fn new(games: Vec<Game>, last: Option<&str>) -> Library {
        let mut library = Library { games, online: false, message: None, selected: 0, first: 0, second: false, shown: 0 };
        if let Some(k) = library.rows().iter().position(|r| matches!(*r, Row::Game(g) if Some(library.games[g].id.as_str()) == last)) {
            library.selected = k;
        }
        library.follow();
        library
    }

    fn rows(&self) -> Vec<Row> {
        let mut rows = Vec::new();
        for (g, game) in self.games.iter().enumerate() {
            rows.push(Row::Game(g));
            rows.extend((0..game.datasets.len()).map(|d| Row::Dataset(g, d)));
        }
        rows.push(Row::Add);
        rows.push(Row::Online);
        rows
    }

    /// The game the screen shows and is drawn in the style of: the chosen
    /// game, or the one chosen before when Add or Online is chosen.
    pub fn game(&self) -> usize {
        self.shown
    }

    /// The games' rows scroll, Add and Online stay at the bottom.
    fn game_rows(&self) -> usize {
        self.rows().len() - 2
    }

    fn follow(&mut self) {
        match self.rows()[self.selected] {
            Row::Game(g) | Row::Dataset(g, _) => self.shown = g,
            Row::Add | Row::Online => {}
        }
        if !matches!(self.rows()[self.selected], Row::Game(_)) {
            self.second = false;
        }
    }

    /// Keep the chosen row in view, for a list of `visible` rows.
    fn scroll(&mut self, visible: usize) {
        if self.selected < self.game_rows() {
            if self.selected < self.first {
                self.first = self.selected;
            } else if self.selected >= self.first + visible {
                self.first = self.selected + 1 - visible;
            }
        }
    }

    /// A key; the screen is `screen` (for the rows that fit).
    pub fn key(&mut self, key: Key, screen: Rect) -> Option<Action> {
        let n = self.rows().len();
        let page = visible_rows(&layout(screen));
        self.message = None;
        match key {
            Key::Up => self.selected = (self.selected + n - 1) % n,
            Key::Down => self.selected = (self.selected + 1) % n,
            Key::PageUp => self.selected = self.selected.saturating_sub(page),
            Key::PageDown => self.selected = (self.selected + page).min(n - 1),
            Key::Home => self.selected = 0,
            Key::End => self.selected = n - 1,
            Key::Left | Key::Right => {
                let game = &self.games[self.shown];
                if matches!(self.rows()[self.selected], Row::Game(_)) && game.menu {
                    self.second = key == Key::Right;
                }
                return None;
            }
            Key::Accept => return self.take(),
            Key::Toggle => {
                if let Row::Dataset(..) = self.rows()[self.selected] {
                    return self.take();
                }
            }
            Key::Back => return Some(Action::Quit),
        }
        self.follow();
        self.scroll(page);
        None
    }

    /// The chosen row taken (Return, a click).
    fn take(&mut self) -> Option<Action> {
        match self.rows()[self.selected] {
            Row::Game(g) => {
                let game = &self.games[g];
                Some(if self.second && game.menu { Action::Menu(game.id.clone()) } else { Action::Play(game.id.clone()) })
            }
            Row::Dataset(g, d) => {
                let game = &mut self.games[g];
                let set = &mut game.datasets[d];
                set.on = !set.on;
                Some(Action::Toggle(game.id.clone(), set.id.clone(), set.on))
            }
            Row::Add => Some(Action::Add),
            Row::Online if self.online => Some(Action::Online),
            Row::Online => {
                self.message = Some("The pack library is not there yet".into());
                None
            }
        }
    }

    /// The mouse at (x, y) on the screen: the row or button under it is
    /// chosen; a click takes it.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, screen: Rect) -> Option<Action> {
        let l = layout(screen);
        let rows = self.rows();
        let mut hit = None;
        if l.rows.contains(x, y) {
            let k = self.first + ((y - l.rows.top) / ROW) as usize;
            if k < self.game_rows() && ((y - l.rows.top) / ROW) < visible_rows(&l) as i32 {
                hit = Some(k);
            }
        } else if l.tools.contains(x, y) {
            hit = Some(rows.len() - 2 + ((y - l.tools.top) / ROW).clamp(0, 1) as usize);
        }
        if let Some(k) = hit {
            self.selected = k;
            self.follow();
            if click {
                self.message = None;
                return self.take();
            }
            return None;
        }
        // the buttons: they belong to the game shown
        let (first, second) = self.buttons(&l);
        let row_of_game = rows.iter().position(|r| *r == Row::Game(self.shown)).unwrap_or(0);
        for (rect, is_second) in [(Some(first), false), (second, true)] {
            if let Some(rect) = rect
                && rect.contains(x, y)
            {
                self.selected = row_of_game;
                self.second = is_second;
                self.follow();
                self.scroll(visible_rows(&l));
                if click {
                    self.message = None;
                    return self.take();
                }
            }
        }
        None
    }

    /// The mouse wheel over the list.
    pub fn wheel(&mut self, dy: i32, screen: Rect) {
        let visible = visible_rows(&layout(screen));
        let max = self.game_rows().saturating_sub(visible);
        self.first = (self.first as i32 - dy).clamp(0, max as i32) as usize;
    }

    fn labels(&self) -> (&'static str, &'static str) {
        (if self.games[self.shown].saved { "Continue" } else { "Play" }, "Title and menu")
    }

    fn buttons(&self, l: &Layout) -> (Rect, Option<Rect>) {
        let (a, _) = self.labels();
        // fixed widths: the theme's font is not known here, the labels are short
        let w1 = 24 + 9 * a.len() as i32;
        let first = Rect::new(l.preview.left, l.buttons, l.preview.left + w1, l.buttons + BUTTON);
        let second = self.games[self.shown].menu.then(|| Rect::new(first.right + 8, l.buttons, first.right + 8 + 140, l.buttons + BUTTON));
        (first, second)
    }

    /// Draw the screen with `theme` (the shown game's: [`Library::game`]);
    /// `preview` fills the preview's rectangle for the shown game. Returns
    /// the texts.
    pub fn draw<T: Theme>(
        &mut self,
        theme: &T,
        surface: &mut T::Surface,
        screen: Rect,
        drop_hint: bool,
        preview: impl FnOnce(&mut T::Surface, Rect, usize),
    ) -> Vec<T::Text> {
        let l = layout(screen);
        let mut texts = Vec::new();
        theme.backdrop(surface, l.screen);
        texts.push(theme.text(l.list.left as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, "Games"));
        if drop_hint {
            let hint = "or drop a file on this window";
            let w = theme.width(Role::Small, hint);
            texts.push(theme.text(screen.right as f32 - MARGIN as f32 - w, (screen.top + MARGIN + 14) as f32, Role::Small, Ink::Dim, hint));
        }

        // the list
        theme.panel(surface, l.list);
        let rows = self.rows();
        let visible = visible_rows(&l);
        self.scroll(visible);
        let inner = l.list.inset(4);
        let room = (inner.width() - 16) as f32;
        for (k, &row) in rows.iter().enumerate().skip(self.first).take(visible.min(self.game_rows().saturating_sub(self.first))) {
            let top = l.rows.top + (k - self.first) as i32 * ROW;
            self.draw_row(theme, surface, &mut texts, row, k == self.selected, Rect::new(inner.left, top, inner.right, top + ROW), room);
        }
        if self.first > 0 {
            theme.mark(surface, l.list.right - 16, l.rows.top - 2, Mark::MoreUp, Ink::Dim);
        }
        if self.first + visible < self.game_rows() {
            theme.mark(surface, l.list.right - 16, l.rows.bottom - 4, Mark::MoreDown, Ink::Dim);
        }
        theme.rule(surface, Rect::new(inner.left + 4, l.tools.top - 4, inner.right - 4, l.tools.top - 3));
        for (k, row) in [Row::Add, Row::Online].into_iter().enumerate() {
            let top = l.tools.top + k as i32 * ROW;
            self.draw_row(theme, surface, &mut texts, row, rows[self.selected] == row, Rect::new(inner.left, top, inner.right, top + ROW), room);
        }

        // the game shown
        preview(surface, l.preview, self.shown);
        let game = &self.games[self.shown];
        let room = l.preview.width() as f32;
        for (k, line) in game.about.iter().take(3).enumerate() {
            let ink = if k == 0 { Ink::Normal } else { Ink::Dim };
            texts.push(theme.text(l.preview.left as f32, (l.about + 16 * k as i32) as f32, Role::Small, ink, &fit(theme, Role::Small, line, room)));
        }
        let on_game = matches!(rows[self.selected], Row::Game(g) if g == self.shown);
        let (a, b) = self.labels();
        let (first, second) = self.buttons(&l);
        for (rect, label, primary) in [(Some(first), a, true), (second, b, false)] {
            let Some(rect) = rect else { continue };
            let focused = on_game && (self.second != primary);
            theme.button(surface, rect, primary, focused);
            let w = theme.width(Role::Row, label);
            let x = (rect.left + rect.right) as f32 / 2.0 - w / 2.0;
            texts.push(theme.text(x, (rect.top + 17) as f32, Role::Row, theme.button_ink(primary, focused), label));
        }

        let bottom = (screen.bottom - 8) as f32;
        let hint = match &self.message {
            Some(m) => m.clone(),
            None => match rows[self.selected] {
                Row::Game(_) if game.menu => "Return: start   ← →: button   ↑ ↓: choose   Esc: quit".into(),
                Row::Game(_) => "Return: start   ↑ ↓: choose   Esc: quit".into(),
                Row::Dataset(..) => "Return or Space: on / off   ↑ ↓: choose   Esc: quit".into(),
                Row::Add => "Return: choose a file   ↑ ↓: choose   Esc: quit".into(),
                Row::Online => "Return: open   ↑ ↓: choose   Esc: quit".into(),
            },
        };
        texts.push(theme.text(l.list.left as f32, bottom, Role::Hint, if self.message.is_some() { Ink::Accent } else { Ink::Dim }, &hint));
        texts
    }

    #[allow(clippy::too_many_arguments)]
    fn draw_row<T: Theme>(&self, theme: &T, surface: &mut T::Surface, texts: &mut Vec<T::Text>, row: Row, chosen: bool, rect: Rect, room: f32) {
        if chosen {
            theme.highlight(surface, rect);
        }
        let ink = |normal: Ink| if chosen { Ink::Chosen } else { normal };
        let base = (rect.top + BASE) as f32;
        let x = (rect.left + 6) as f32;
        match row {
            Row::Game(g) => {
                let game = &self.games[g];
                texts.push(theme.text(x, base, Role::Row, ink(Ink::Normal), &fit(theme, Role::Row, &game.name, room)));
            }
            Row::Dataset(g, d) => {
                let set = &self.games[g].datasets[d];
                let state = if set.on { "on" } else { "off" };
                let w = theme.width(Role::Small, state);
                texts.push(theme.text(x + 14.0, base, Role::Row, ink(if set.on { Ink::Normal } else { Ink::Dim }), &fit(theme, Role::Row, &format!("+ {}", set.name), room - 24.0 - w)));
                texts.push(theme.text(rect.right as f32 - 6.0 - w, base, Role::Small, ink(Ink::Dim), state));
            }
            Row::Add => texts.push(theme.text(x, base, Role::Row, ink(Ink::Accent), "+ Add from a file...")),
            Row::Online => texts.push(theme.text(x, base, Role::Row, ink(if self.online { Ink::Accent } else { Ink::Dim }), "Get packs online...")),
        }
    }
}

fn visible_rows(l: &Layout) -> usize {
    (l.rows.height() / ROW).max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    fn games() -> Vec<Game> {
        let set = |id: &str, on| Dataset { id: id.into(), name: id.into(), on };
        vec![
            Game { id: "quagmire".into(), name: "Quagmire".into(), about: vec!["Level 12".into()], saved: true, menu: true, datasets: vec![set("hires", false)] },
            Game { id: "sammy".into(), name: "Sammy the Cyclebot".into(), about: vec![], saved: false, menu: false, datasets: vec![] },
        ]
    }

    /// The selection starts on the game played last: Return there plays
    /// it (Continue when saved).
    #[test]
    fn starts_on_the_last_game() {
        let mut l = Library::new(games(), Some("sammy"));
        assert_eq!(l.game(), 1);
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Play("sammy".into())));
        let mut l = Library::new(games(), Some("gone"));
        assert_eq!(l.game(), 0);
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Play("quagmire".into())));
    }

    /// Right chooses Title and menu (only for a game that has one); a
    /// dataset toggles; Add and Online; Escape quits.
    #[test]
    fn keys() {
        let mut l = Library::new(games(), None);
        l.key(Key::Right, SCREEN);
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Menu("quagmire".into())));
        l.key(Key::Down, SCREEN);
        assert_eq!(l.key(Key::Toggle, SCREEN), Some(Action::Toggle("quagmire".into(), "hires".into(), true)));
        assert!(l.games[0].datasets[0].on);
        l.key(Key::Down, SCREEN);
        assert_eq!(l.game(), 1);
        l.key(Key::Right, SCREEN);
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Play("sammy".into())), "Sammy has no menu yet");
        l.key(Key::Down, SCREEN);
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Add));
        assert_eq!(l.game(), 1, "Add keeps the game shown");
        l.key(Key::Down, SCREEN);
        assert_eq!(l.key(Key::Accept, SCREEN), None);
        assert!(l.message.is_some(), "online: not there yet");
        l.online = true;
        assert_eq!(l.key(Key::Accept, SCREEN), Some(Action::Online));
        assert_eq!(l.key(Key::Back, SCREEN), Some(Action::Quit));
    }

    /// A click on a row takes it; on a button, that button.
    #[test]
    fn mouse() {
        let mut l = Library::new(games(), None);
        let lay = layout(SCREEN);
        let y = lay.rows.top + ROW + 5;
        assert_eq!(l.mouse(lay.list.left + 20, y, true, SCREEN), Some(Action::Toggle("quagmire".into(), "hires".into(), true)));
        let (_, second) = l.buttons(&lay);
        let b = second.unwrap();
        assert_eq!(l.mouse(b.left + 3, b.top + 3, true, SCREEN), Some(Action::Menu("quagmire".into())));
        assert_eq!(l.mouse(lay.list.left + 20, lay.tools.top + 3, true, SCREEN), Some(Action::Add));
    }

    /// Many datasets scroll; Add and Online stay.
    #[test]
    fn scrolls() {
        let mut g = games();
        g[0].datasets = (0..40).map(|k| Dataset { id: k.to_string(), name: k.to_string(), on: false }).collect();
        let mut l = Library::new(g, None);
        for _ in 0..30 {
            l.key(Key::Down, SCREEN);
        }
        assert!(l.first > 0);
        let mut s = Record::default();
        let texts = l.draw(&Plain, &mut s, SCREEN, true, |_, _, _| {});
        assert!(texts.iter().any(|t| t.3 == "+ Add from a file..."));
        assert!(texts.iter().any(|t| t.3 == "+ 29"));
        assert!(!texts.iter().any(|t| t.3 == "+ 0"));
        l.key(Key::End, SCREEN);
        assert_eq!(l.key(Key::Accept, SCREEN), None, "Online, dim");
    }
}
