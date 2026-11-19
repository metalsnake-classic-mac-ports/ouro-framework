//! The timeline (the ports' rule 5): the snapshots rewind goes back to,
//! as a tree - playing on from an earlier snapshot makes a fork. The
//! chosen snapshot's picture at the top with the port's line about it
//! (level, time played, clock time, deaths), the branches below: a dot per
//! snapshot, x by its place in time, a row (lane) per branch. ← → move
//! along the time, ↑ ↓ to the branch above or below; Return plays on from
//! the chosen snapshot.
//!
//! The port keeps the snapshots (the states and pictures); this screen
//! gets their tree: each one's parent and whether its data is gone
//! (dropped for memory: then it is a link of the tree only).

use crate::{Ink, Key, Rect, Role, Theme, fit};

/// A snapshot as the tree sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Node {
    pub parent: Option<usize>,
    pub gone: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    /// Play on from this snapshot
    Play(usize),
    Back,
}

const MARGIN: i32 = 16;
/// Pixels between two snapshots in time, between two lanes.
const STEP: i32 = 8;
const GAP: i32 = 10;

struct Layout {
    preview: Rect,
    info: i32,
    branches: Rect,
}

fn layout(screen: Rect) -> Layout {
    let top = screen.top + MARGIN + 24;
    let (w, h) = (screen.width() / 2, screen.height() / 2);
    let left = screen.left + (screen.width() - w) / 2;
    let preview = Rect::new(left, top, left + w, top + h);
    let info = preview.bottom + 20;
    let branches = Rect::new(screen.left + MARGIN, info + 14, screen.right - MARGIN, screen.bottom - MARGIN - 14);
    Layout { preview, info, branches }
}

/// How deep each snapshot is in the tree (its place in time).
pub fn depths(nodes: &[Node]) -> Vec<usize> {
    let mut d = vec![0; nodes.len()];
    for k in 0..nodes.len() {
        d[k] = nodes[k].parent.map_or(0, |p| d[p] + 1);
    }
    d
}

/// A lane (row) per branch: a snapshot's first child goes on in its lane,
/// every other child opens the next free lane below.
pub fn lanes(nodes: &[Node]) -> Vec<usize> {
    let mut lane = vec![0; nodes.len()];
    let mut next = 1;
    for k in 0..nodes.len() {
        if let Some(p) = nodes[k].parent {
            let first = (0..k).all(|c| nodes[c].parent != Some(p));
            if first {
                lane[k] = lane[p];
            } else {
                lane[k] = next;
                next += 1;
            }
        } else if k > 0 {
            lane[k] = next;
            next += 1;
        }
    }
    lane
}

fn children(nodes: &[Node], k: usize) -> Vec<usize> {
    (0..nodes.len()).filter(|&c| nodes[c].parent == Some(k) && !nodes[c].gone).collect()
}

#[derive(Debug, Default)]
pub struct Timeline {
    /// The chosen snapshot (None: there is none)
    pub chosen: Option<usize>,
}

impl Timeline {
    /// Open at `current`, the snapshot the game goes on from.
    pub fn open(&mut self, nodes: &[Node], current: Option<usize>) {
        self.chosen = current.filter(|&k| k < nodes.len() && !nodes[k].gone);
    }

    pub fn key(&mut self, key: Key, nodes: &[Node]) -> Option<Choice> {
        let Some(k) = self.chosen else {
            return (key == Key::Back).then_some(Choice::Back);
        };
        match key {
            Key::Left => {
                if let Some(p) = nodes[k].parent.filter(|&p| !nodes[p].gone) {
                    self.chosen = Some(p);
                }
            }
            Key::Right => {
                // the child on the same lane, else the first
                let lane = lanes(nodes);
                let kids = children(nodes, k);
                if let Some(&c) = kids.iter().find(|&&c| lane[c] == lane[k]).or(kids.first()) {
                    self.chosen = Some(c);
                }
            }
            Key::Up | Key::Down => {
                let lane = lanes(nodes);
                let want = if key == Key::Up { lane[k].checked_sub(1) } else { Some(lane[k] + 1) };
                if let Some(want) = want {
                    let d = depths(nodes);
                    let best = (0..nodes.len()).filter(|&c| !nodes[c].gone && lane[c] == want).min_by_key(|&c| (d[c] as i64 - d[k] as i64).abs());
                    if let Some(c) = best {
                        self.chosen = Some(c);
                    }
                }
            }
            Key::Accept | Key::Toggle => return Some(Choice::Play(k)),
            Key::Back => return Some(Choice::Back),
            _ => {}
        }
        None
    }

    /// Where each snapshot's dot is (None: off the screen).
    fn places(&self, nodes: &[Node], screen: Rect) -> Vec<Option<(i32, i32)>> {
        let Some(k) = self.chosen else { return vec![None; nodes.len()] };
        let l = layout(screen);
        let (d, lane) = (depths(nodes), lanes(nodes));
        let rows = ((l.branches.height() - 16) / GAP).max(1) as usize;
        let first = lane[k].saturating_sub(rows / 2);
        let mid = (l.branches.left + l.branches.right) / 2;
        (0..nodes.len())
            .map(|j| {
                let x = mid + (d[j] as i32 - d[k] as i32) * STEP;
                let y = l.branches.top + 12 + (lane[j] as i32 - first as i32) * GAP;
                (lane[j] >= first && lane[j] < first + rows && x >= l.branches.left && x < l.branches.right).then_some((x, y))
            })
            .collect()
    }

    /// The mouse: a click on a dot chooses it, a click on the chosen one
    /// (or on the picture) plays on from it.
    pub fn mouse(&mut self, x: i32, y: i32, click: bool, nodes: &[Node], screen: Rect) -> Option<Choice> {
        if !click {
            return None;
        }
        if let Some(k) = self.chosen
            && layout(screen).preview.contains(x, y)
        {
            return Some(Choice::Play(k));
        }
        let places = self.places(nodes, screen);
        let hit = (0..nodes.len()).filter(|&j| !nodes[j].gone).find(|&j| places[j].is_some_and(|(px, py)| (px - x).abs() <= 3 && (py - y).abs() <= 4))?;
        if self.chosen == Some(hit) {
            return Some(Choice::Play(hit));
        }
        self.chosen = Some(hit);
        None
    }

    /// The screen: `preview` draws the chosen snapshot's picture into its
    /// rectangle; `info` is the line under it.
    #[allow(clippy::too_many_arguments)]
    pub fn draw<T: Theme>(
        &self,
        theme: &T,
        surface: &mut T::Surface,
        screen: Rect,
        nodes: &[Node],
        current: Option<usize>,
        info: &str,
        preview: impl FnOnce(&mut T::Surface, Rect),
    ) -> Vec<T::Text> {
        let l = layout(screen);
        let mut texts = Vec::new();
        theme.backdrop(surface, screen);
        texts.push(theme.text((screen.left + MARGIN) as f32, (screen.top + MARGIN + 14) as f32, Role::Heading, Ink::Accent, "Timeline"));
        let Some(k) = self.chosen else {
            let text = "No snapshots yet: one is kept every few seconds of play";
            let w = theme.width(Role::Row, text);
            texts.push(theme.text((screen.left + screen.right) as f32 / 2.0 - w / 2.0, (screen.top + screen.bottom) as f32 / 2.0, Role::Row, Ink::Normal, text));
            texts.push(theme.text((screen.left + MARGIN) as f32, (screen.bottom - 8) as f32, Role::Hint, Ink::Dim, "Esc: back"));
            return texts;
        };
        theme.panel(surface, l.preview.inset(-2));
        preview(surface, l.preview);
        let room = (screen.width() - 2 * MARGIN) as f32;
        let info = fit(theme, Role::Small, info, room);
        let w = theme.width(Role::Small, &info);
        texts.push(theme.text((screen.left + screen.right) as f32 / 2.0 - w / 2.0, l.info as f32, Role::Small, Ink::Normal, &info));

        // the branches: a line to each parent (along its lane, then up
        // to it), then the dots; the current snapshot stands out, the
        // chosen one is framed
        theme.panel(surface, l.branches);
        let places = self.places(nodes, screen);
        let (d, lane) = (depths(nodes), lanes(nodes));
        let mid = (l.branches.left + l.branches.right) / 2;
        let clip = l.branches.inset(3);
        let clipped = |r: Rect| Rect::new(r.left.max(clip.left), r.top.max(clip.top), r.right.min(clip.right), r.bottom.min(clip.bottom));
        for j in 0..nodes.len() {
            let (Some((x, y)), Some(p)) = (places[j], nodes[j].parent) else { continue };
            // the parent's place even when it is off the screen
            let px = mid + (d[p] as i32 - d[k] as i32) * STEP;
            let py = y + (lane[p] as i32 - lane[j] as i32) * GAP;
            let along = clipped(Rect::new(px.min(x), y, px.max(x) + 1, y + 1));
            if along.width() > 0 && along.height() > 0 {
                theme.fill(surface, along, Ink::Dim);
            }
            if py != y {
                let up = clipped(Rect::new(px, py.min(y), px + 1, py.max(y) + 1));
                if up.width() > 0 && up.height() > 0 {
                    theme.fill(surface, up, Ink::Dim);
                }
            }
        }
        for j in 0..nodes.len() {
            let Some((x, y)) = places[j].filter(|_| !nodes[j].gone) else { continue };
            theme.fill(surface, Rect::new(x - 1, y - 1, x + 2, y + 2), if Some(j) == current { Ink::Normal } else { Ink::Accent });
        }
        if let Some((x, y)) = places[k] {
            for r in [Rect::new(x - 3, y - 3, x + 4, y - 2), Rect::new(x - 3, y + 3, x + 4, y + 4), Rect::new(x - 3, y - 3, x - 2, y + 4), Rect::new(x + 3, y - 3, x + 4, y + 4)] {
                theme.fill(surface, r, Ink::Accent);
            }
        }
        texts.push(theme.text((screen.left + MARGIN) as f32, (screen.bottom - 8) as f32, Role::Hint, Ink::Dim, "← → time   ↑ ↓ branch   Return: play from here   Esc: back"));
        texts
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_theme::{Plain, Record};

    const SCREEN: Rect = Rect::new(0, 0, 640, 480);

    /// Four snapshots, then two from the second: a fork in lane 1.
    fn tree() -> Vec<Node> {
        let n = |parent| Node { parent, gone: false };
        vec![n(None), n(Some(0)), n(Some(1)), n(Some(2)), n(Some(1)), n(Some(4))]
    }

    #[test]
    fn lanes_and_keys() {
        let t = tree();
        assert_eq!(lanes(&t), vec![0, 0, 0, 0, 1, 1]);
        let mut s = Timeline::default();
        s.open(&t, Some(5));
        s.key(Key::Up, &t);
        assert_eq!(s.chosen, Some(3));
        s.key(Key::Left, &t);
        s.key(Key::Left, &t);
        assert_eq!(s.chosen, Some(1));
        s.key(Key::Right, &t);
        assert_eq!(s.chosen, Some(2), "on along its own lane");
        assert_eq!(s.key(Key::Accept, &t), Some(Choice::Play(2)));
        // a snapshot that is gone cannot be chosen
        let mut g = tree();
        g[1].gone = true;
        s.open(&g, Some(2));
        s.key(Key::Left, &g);
        assert_eq!(s.chosen, Some(2));
    }

    #[test]
    fn mouse_on_the_dots() {
        let t = tree();
        let mut s = Timeline::default();
        s.open(&t, Some(3));
        let places = s.places(&t, SCREEN);
        let (x, y) = places[4].unwrap();
        assert_eq!(s.mouse(x, y, true, &t, SCREEN), None);
        assert_eq!(s.chosen, Some(4));
        let (x, y) = s.places(&t, SCREEN)[4].unwrap();
        assert_eq!(s.mouse(x + 1, y, true, &t, SCREEN), Some(Choice::Play(4)));
        let mut rec = Record::default();
        let texts = s.draw(&Plain, &mut rec, SCREEN, &t, Some(3), "Level 1", |_, _| {});
        assert!(texts.iter().any(|t| t.3 == "Level 1"));
    }
}
