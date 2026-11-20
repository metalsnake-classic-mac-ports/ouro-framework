//! The screens every Ouro port shares (the rules in the ports' CLAUDE.md:
//! the game library, adding packs, a game's menu, the level selection,
//! the options page, the keys page, the timeline). This crate holds what they do and where
//! things go - the state, the keys, the mouse, the layout - and nothing of
//! how they look: each game draws them through its [`Theme`] (its colours,
//! font, boxes), so a screen looks like the game it belongs to and works
//! the same in every port.
//!
//! No SDL and no I/O here except reading folders for the file browser: the
//! port maps its keys onto [`Key`], its mouse onto screen pixels, and
//! shows what the screens draw.

pub mod browser;
pub mod keys;
pub mod levels;
pub mod library;
pub mod menu;
pub mod options;
pub mod timeline;

/// A rectangle in screen pixels: `left..right` x `top..bottom`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Rect {
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}

impl Rect {
    pub const fn new(left: i32, top: i32, right: i32, bottom: i32) -> Rect {
        Rect { left, top, right, bottom }
    }

    pub fn width(&self) -> i32 {
        self.right - self.left
    }

    pub fn height(&self) -> i32 {
        self.bottom - self.top
    }

    pub fn contains(&self, x: i32, y: i32) -> bool {
        x >= self.left && x < self.right && y >= self.top && y < self.bottom
    }

    /// Smaller by `by` on every side.
    pub fn inset(&self, by: i32) -> Rect {
        Rect::new(self.left + by, self.top + by, self.right - by, self.bottom - by)
    }
}

/// What the player pressed, whatever the device: the port maps the
/// keyboard (arrows, Return, Escape, Space ...) and the gamepad (D-pad, A,
/// B ...) onto these.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Key {
    Up,
    Down,
    Left,
    Right,
    PageUp,
    PageDown,
    Home,
    End,
    /// Return, Enter, the pad's A
    Accept,
    /// Escape, the pad's B
    Back,
    /// Space, the pad's X: switch the chosen thing on or off
    Toggle,
}

/// The kinds of text a screen writes; the theme picks font and size.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    /// A screen's title
    Heading,
    /// A list row, a button
    Row,
    /// Details beside or under a list
    Small,
    /// The line of keys at the bottom
    Hint,
}

/// How a text or mark stands out; the theme picks the colour.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ink {
    Normal,
    /// On the highlight of the chosen row or button
    Chosen,
    /// Not available, off, or less important
    Dim,
    /// What the player should notice (the game's own colour)
    Accent,
}

/// Small pictures the screens use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mark {
    /// Done, on
    Tick,
    /// More above
    MoreUp,
    /// More below
    MoreDown,
    /// A folder in the file browser
    Folder,
    /// Something to watch (a story sequence in the level selection)
    Play,
}

/// How a game draws the shared screens. The screens call these with
/// rectangles they have laid out; the theme fills them in its style.
/// `Surface` is the port's picture, `Text` its line of text (drawn by the
/// port at the window's resolution, on top of the picture).
pub trait Theme {
    type Surface;
    type Text;

    /// The width of `text` in screen pixels.
    fn width(&self, role: Role, text: &str) -> f32;

    /// Everything behind the screen (a colour, the game's title picture).
    fn backdrop(&self, surface: &mut Self::Surface, screen: Rect);

    /// A box that holds a list or a page.
    fn panel(&self, surface: &mut Self::Surface, rect: Rect);

    /// The bar behind the chosen row.
    fn highlight(&self, surface: &mut Self::Surface, rect: Rect);

    /// A button; `primary` is the one Return takes, `focused` the one
    /// chosen now.
    fn button(&self, surface: &mut Self::Surface, rect: Rect, primary: bool, focused: bool);

    /// A thin line between parts of a box.
    fn rule(&self, surface: &mut Self::Surface, rect: Rect);

    /// A rectangle in the colour of `ink` (the timeline's lines and dots).
    fn fill(&self, surface: &mut Self::Surface, rect: Rect, ink: Ink);

    /// A small picture, about 9 x 9 pixels, at (x, y); by default drawn
    /// with [`Theme::fill`] ([`plain_mark`]).
    fn mark(&self, surface: &mut Self::Surface, x: i32, y: i32, mark: Mark, ink: Ink) {
        plain_mark(self, surface, x, y, mark, ink);
    }

    /// A line of text at (x, baseline).
    fn text(&self, x: f32, baseline: f32, role: Role, ink: Ink, text: &str) -> Self::Text;

    /// The ink of text on a button.
    fn button_ink(&self, primary: bool, focused: bool) -> Ink {
        let _ = primary;
        if focused { Ink::Chosen } else { Ink::Normal }
    }
}

/// The marks in plain rectangles of `ink`: arrows, a tick, a play
/// triangle, a folder.
pub fn plain_mark<T: Theme + ?Sized>(theme: &T, surface: &mut T::Surface, x: i32, y: i32, mark: Mark, ink: Ink) {
    let mut fill = |left: i32, top: i32, right: i32, bottom: i32| theme.fill(surface, Rect::new(left, top, right, bottom), ink);
    match mark {
        Mark::MoreUp | Mark::MoreDown => {
            for k in 0..5 {
                let row = if mark == Mark::MoreDown { y + k } else { y + 4 - k };
                fill(x + k, row, x + 9 - k, row + 1);
            }
        }
        Mark::Tick => {
            for k in 0..4 {
                fill(x + k, y + 4 + k, x + k + 2, y + 6 + k);
            }
            for k in 0..5 {
                fill(x + 4 + k, y + 6 - k, x + 6 + k, y + 8 - k);
            }
        }
        Mark::Play => {
            // a triangle pointing right, 7 x 9
            for k in 0..5 {
                fill(x, y + k, x + 2 + k * 3 / 2, y + 1 + k);
                fill(x, y + 8 - k, x + 2 + k * 3 / 2, y + 9 - k);
            }
        }
        Mark::Folder => {
            fill(x, y + 2, x + 11, y + 10);
            fill(x, y, x + 5, y + 2);
        }
    }
}

/// Text that fits in `room` pixels: cut with "..." when it is too wide.
pub fn fit<T: Theme + ?Sized>(theme: &T, role: Role, text: &str, room: f32) -> String {
    if theme.width(role, text) <= room {
        return text.to_string();
    }
    let mut cut: String = text.to_string();
    while !cut.is_empty() {
        cut.pop();
        let tried = format!("{}...", cut.trim_end());
        if theme.width(role, &tried) <= room {
            return tried;
        }
    }
    String::new()
}

#[cfg(test)]
pub(crate) mod test_theme {
    //! A theme that records what it is asked to draw.
    use super::*;

    #[derive(Default)]
    pub struct Record {
        pub calls: Vec<String>,
    }

    pub struct Plain;

    impl Theme for Plain {
        type Surface = Record;
        type Text = (f32, f32, Ink, String);

        fn width(&self, _role: Role, text: &str) -> f32 {
            7.0 * text.chars().count() as f32
        }
        fn backdrop(&self, s: &mut Record, r: Rect) {
            s.calls.push(format!("backdrop {r:?}"));
        }
        fn panel(&self, s: &mut Record, r: Rect) {
            s.calls.push(format!("panel {r:?}"));
        }
        fn highlight(&self, s: &mut Record, r: Rect) {
            s.calls.push(format!("highlight {r:?}"));
        }
        fn button(&self, s: &mut Record, r: Rect, primary: bool, focused: bool) {
            s.calls.push(format!("button {r:?} {primary} {focused}"));
        }
        fn rule(&self, s: &mut Record, r: Rect) {
            s.calls.push(format!("rule {r:?}"));
        }
        fn fill(&self, s: &mut Record, r: Rect, ink: Ink) {
            s.calls.push(format!("fill {r:?} {ink:?}"));
        }
        fn mark(&self, s: &mut Record, x: i32, y: i32, m: Mark, _ink: Ink) {
            s.calls.push(format!("mark {x} {y} {m:?}"));
        }
        fn text(&self, x: f32, baseline: f32, _role: Role, ink: Ink, text: &str) -> Self::Text {
            (x, baseline, ink, text.to_string())
        }
    }

    #[test]
    fn fit_cuts_long_text() {
        assert_eq!(fit(&Plain, Role::Row, "short", 100.0), "short");
        // 7 px a letter: 10 letters fit, "abcdefg..." is 10
        assert_eq!(fit(&Plain, Role::Row, "abcdefghijklmnop", 70.0), "abcdefg...");
    }
}
