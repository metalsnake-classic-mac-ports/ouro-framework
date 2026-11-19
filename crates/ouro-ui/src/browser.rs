//! Choosing a file inside the game: for adding packs in full screen and
//! with a gamepad, where dragging a file onto the window does not work.
//! Places (Downloads, Desktop, Documents, Home) across the top, the
//! folder's files below; the port says for each file what it is (an Ouro
//! pack for a game, a game's Mac application) or nothing (not usable).

use std::path::{Path, PathBuf};

use crate::{Ink, Key, Mark, Rect, Role, Theme, fit};

/// A file or folder in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub name: String,
    pub path: PathBuf,
    pub folder: bool,
    /// What the port found the file to be; `None`: not usable
    pub note: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Action {
    /// A usable file chosen
    Pick(PathBuf),
    /// Escape: back without a file
    Close,
}

pub struct Browser {
    /// Name and folder of each place
    pub places: Vec<(String, PathBuf)>,
    pub place: usize,
    pub folder: PathBuf,
    pub entries: Vec<Entry>,
    selected: usize,
    first: usize,
}

const ROW: i32 = 18;
const MARGIN: i32 = 16;

struct Layout {
    places: i32,
    path: i32,
    list: Rect,
}

fn layout(screen: Rect) -> Layout {
    let top = screen.top + MARGIN + 24;
    Layout { places: top, path: top + 36, list: Rect::new(screen.left + MARGIN, top + 44, screen.right - MARGIN, screen.bottom - MARGIN - 14) }
}

/// The places found in `home`: Downloads, Desktop, Documents (when there)
/// and the home folder itself.
pub fn places(home: &Path) -> Vec<(String, PathBuf)> {
    let mut v: Vec<(String, PathBuf)> = ["Downloads", "Desktop", "Documents"].iter().map(|n| (n.to_string(), home.join(n))).filter(|(_, p)| p.is_dir()).collect();
    v.push(("Home".into(), home.to_path_buf()));
    v
}

/// The folder's entries: folders first, then files, each by name; hidden
/// ones left out; `..` first unless at the top. `what` tells what a file
/// is.
pub fn read_folder(folder: &Path, what: &dyn Fn(&Path) -> Option<String>) -> Vec<Entry> {
    let mut folders = Vec::new();
    let mut files = Vec::new();
    if let Ok(dir) = std::fs::read_dir(folder) {
        for e in dir.flatten() {
            let name = e.file_name().to_string_lossy().into_owned();
            if name.starts_with('.') {
                continue;
            }
            let path = e.path();
            if path.is_dir() {
                folders.push(Entry { name, path, folder: true, note: None });
            } else {
                let note = what(&path);
                files.push(Entry { name, path, folder: false, note });
            }
        }
    }
    let key = |e: &Entry| e.name.to_lowercase();
    folders.sort_by_key(key);
    files.sort_by_key(key);
    let mut v = Vec::new();
    if let Some(parent) = folder.parent() {
        v.push(Entry { name: "..".into(), path: parent.to_path_buf(), folder: true, note: None });
    }
    v.extend(folders);
    v.extend(files);
    v
}

impl Browser {
    /// The browser in the first place (Downloads when there).
    pub fn new(places: Vec<(String, PathBuf)>, what: &dyn Fn(&Path) -> Option<String>) -> Browser {
        let folder = places.first().map(|p| p.1.clone()).unwrap_or_else(|| PathBuf::from("/"));
        let entries = read_folder(&folder, what);
        Browser { places, place: 0, folder, entries, selected: 0, first: 0 }
    }

    fn open(&mut self, folder: PathBuf, what: &dyn Fn(&Path) -> Option<String>) {
        let from = self.folder.clone();
        self.entries = read_folder(&folder, what);
        self.folder = folder;
        // going up: the folder come from stays chosen
        self.selected = self.entries.iter().position(|e| e.path == from).unwrap_or(0);
        self.first = 0;
    }

    /// A key; `what` tells what a file is when a folder is opened.
    pub fn key(&mut self, key: Key, screen: Rect, what: &dyn Fn(&Path) -> Option<String>) -> Option<Action> {
        let n = self.entries.len().max(1);
        let page = visible(screen);
        match key {
            Key::Up => self.selected = (self.selected + n - 1) % n,
            Key::Down => self.selected = (self.selected + 1) % n,
            Key::PageUp => self.selected = self.selected.saturating_sub(page),
            Key::PageDown => self.selected = (self.selected + page).min(n - 1),
            Key::Home => self.selected = 0,
            Key::End => self.selected = n - 1,
            Key::Left | Key::Right if !self.places.is_empty() => {
                let k = self.places.len();
                self.place = if key == Key::Left { (self.place + k - 1) % k } else { (self.place + 1) % k };
                let folder = self.places[self.place].1.clone();
                self.open(folder, what);
                self.selected = 0;
            }
            Key::Accept | Key::Toggle => return self.take(what),
            Key::Back => return Some(Action::Close),
            _ => {}
        }
        self.scroll(page);
        None
    }

    fn take(&mut self, what: &dyn Fn(&Path) -> Option<String>) -> Option<Action> {
        let e = self.entries.get(self.selected)?.clone();
        if e.folder {
            self.open(e.path, what);
            None
        } else {
            e.note.is_some().then_some(Action::Pick(e.path))
        }
    }

    fn scroll(&mut self, page: usize) {
        if self.selected < self.first {
            self.first = self.selected;
        } else if self.selected >= self.first + page {
            self.first = self.selected + 1 - page;
        }
    }

    /// The mouse: a row under it is chosen, a click takes it; a click on
    /// a place opens it.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, screen: Rect, theme_width: &dyn Fn(&str) -> f32, what: &dyn Fn(&Path) -> Option<String>) -> Option<Action> {
        let l = layout(screen);
        if click {
            for (k, rect) in self.place_rects(&l, theme_width).into_iter().enumerate() {
                if rect.contains(x, y) {
                    self.place = k;
                    let folder = self.places[k].1.clone();
                    self.open(folder, what);
                    self.selected = 0;
                    return None;
                }
            }
        }
        let inner = l.list.inset(4);
        if inner.contains(x, y) {
            let k = self.first + ((y - inner.top) / ROW) as usize;
            if k < self.entries.len() && ((y - inner.top) / ROW) < visible(screen) as i32 {
                self.selected = k;
                if click {
                    return self.take(what);
                }
            }
        }
        None
    }

    pub fn wheel(&mut self, dy: i32, screen: Rect) {
        let max = self.entries.len().saturating_sub(visible(screen));
        self.first = (self.first as i32 - dy).clamp(0, max as i32) as usize;
    }

    fn place_rects(&self, l: &Layout, width: &dyn Fn(&str) -> f32) -> Vec<Rect> {
        let mut x = l.list.left;
        self.places
            .iter()
            .map(|(name, _)| {
                let w = width(name) as i32 + 16;
                let r = Rect::new(x, l.places, x + w, l.places + 20);
                x += w + 6;
                r
            })
            .collect()
    }

    /// Draw the browser with `theme`; returns the texts.
    pub fn draw<T: Theme>(&mut self, theme: &T, surface: &mut T::Surface, screen: Rect) -> Vec<T::Text> {
        let l = layout(screen);
        let mut texts = Vec::new();
        theme.backdrop(surface, screen);
        texts.push(theme.text((screen.left + MARGIN) as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, "Add from a file"));
        for (k, rect) in self.place_rects(&l, &|s| theme.width(Role::Small, s)).into_iter().enumerate() {
            let on = k == self.place && self.folder == self.places[k].1;
            theme.button(surface, rect, false, on);
            texts.push(theme.text((rect.left + 8) as f32, (rect.top + 14) as f32, Role::Small, theme.button_ink(false, on), &self.places[k].0));
        }
        let path = self.folder.display().to_string();
        texts.push(theme.text(l.list.left as f32, l.path as f32, Role::Small, Ink::Dim, &fit(theme, Role::Small, &path, l.list.width() as f32)));
        theme.panel(surface, l.list);
        let inner = l.list.inset(4);
        let page = visible(screen);
        self.scroll(page);
        for (k, e) in self.entries.iter().enumerate().skip(self.first).take(page) {
            let top = inner.top + (k - self.first) as i32 * ROW;
            let chosen = k == self.selected;
            let row = Rect::new(inner.left, top, inner.right, top + ROW);
            if chosen {
                theme.highlight(surface, row);
            }
            let ink = |normal| if chosen { Ink::Chosen } else { normal };
            let base = (top + 13) as f32;
            if e.folder {
                theme.mark(surface, row.left + 6, top + 4, Mark::Folder, ink(Ink::Normal));
            }
            let note = match (&e.note, e.folder) {
                (_, true) => String::new(),
                (Some(n), _) => n.clone(),
                (None, _) => "not usable here".into(),
            };
            let note_width = theme.width(Role::Small, &note);
            let name_room = row.width() as f32 - 40.0 - note_width;
            let usable = e.folder || e.note.is_some();
            texts.push(theme.text((row.left + 22) as f32, base, Role::Row, ink(if usable { Ink::Normal } else { Ink::Dim }), &fit(theme, Role::Row, &e.name, name_room)));
            texts.push(theme.text(row.right as f32 - 6.0 - note_width, base, Role::Small, ink(if e.note.is_some() { Ink::Accent } else { Ink::Dim }), &note));
        }
        if self.first > 0 {
            theme.mark(surface, l.list.right - 16, l.list.top + 2, Mark::MoreUp, Ink::Dim);
        }
        if self.first + page < self.entries.len() {
            theme.mark(surface, l.list.right - 16, l.list.bottom - 8, Mark::MoreDown, Ink::Dim);
        }
        texts.push(theme.text(
            (screen.left + MARGIN) as f32,
            (screen.bottom - 8) as f32,
            Role::Hint,
            Ink::Dim,
            "Return: open or add   ↑ ↓: choose   ← →: places   Esc: back",
        ));
        texts
    }
}

fn visible(screen: Rect) -> usize {
    ((layout(screen).list.height() - 8) / ROW).max(1) as usize
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    fn what(p: &Path) -> Option<String> {
        (p.extension().and_then(|e| e.to_str()) == Some("zip")).then(|| "Ouro pack".to_string())
    }

    /// A folder with a sub folder, a pack and a text file: folders first,
    /// the pack can be picked, the text cannot; into the folder and back
    /// up to it.
    #[test]
    fn browse_and_pick() {
        let dir = std::env::temp_dir().join(format!("ouro-ui-browser-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("sub")).unwrap();
        std::fs::write(dir.join("b.zip"), b"").unwrap();
        std::fs::write(dir.join("a.txt"), b"").unwrap();
        std::fs::write(dir.join(".hidden"), b"").unwrap();
        let mut b = Browser::new(vec![("Here".into(), dir.clone())], &what);
        let names: Vec<&str> = b.entries.iter().map(|e| e.name.as_str()).collect();
        assert_eq!(names, ["..", "sub", "a.txt", "b.zip"]);
        b.key(Key::Down, SCREEN, &what);
        b.key(Key::Down, SCREEN, &what);
        assert_eq!(b.key(Key::Accept, SCREEN, &what), None, "a.txt is not usable");
        b.key(Key::Down, SCREEN, &what);
        assert_eq!(b.key(Key::Accept, SCREEN, &what), Some(Action::Pick(dir.join("b.zip"))));
        b.key(Key::Home, SCREEN, &what);
        b.key(Key::Down, SCREEN, &what);
        b.key(Key::Accept, SCREEN, &what);
        assert_eq!(b.folder, dir.join("sub"));
        b.key(Key::Accept, SCREEN, &what);
        assert_eq!(b.folder, dir);
        assert_eq!(b.entries[b.selected].name, "sub", "back up: the folder come from is chosen");
        let mut s = Record::default();
        let texts = b.draw(&Plain, &mut s, SCREEN);
        assert!(texts.iter().any(|t| t.3 == "Ouro pack"));
        assert_eq!(b.key(Key::Back, SCREEN, &what), Some(Action::Close));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
