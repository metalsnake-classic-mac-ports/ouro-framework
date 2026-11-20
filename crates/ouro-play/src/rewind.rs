//! Rewind (the ports' rule 5): every few seconds of play a snapshot of the
//! game's state; each knows the one before it, so playing on from an
//! earlier one makes a fork and the snapshots form a tree, which ouro-ui's
//! timeline shows. A snapshot keeps the port's state (anything it can
//! serialise) as deflated JSON - a game's state is mostly zeros and
//! repeats, Harry's 367 KB of JSON become a few dozen - a picture (PNG)
//! and the line the timeline shows about it.
//!
//! Memory: at most `max` snapshots keep their state and picture; the
//! oldest are dropped first and stay as links of the tree.
//!
//! On disk (with Continue's state): a snapshot never changes once taken,
//! so saving writes only what is new - `n<k>.json` (parent, frame, time,
//! line, whether it is dropped), `s<k>.z` (the state), `p<k>.png` - and
//! then `index.json` (how many, the current one, the next snapshot's
//! frame). A snapshot dropped since is written again as dropped, and its
//! state and picture removed.

use std::marker::PhantomData;
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Snapshot {
    pub parent: Option<usize>,
    /// The game's frame it was taken at
    pub frame: u64,
    /// When (seconds since 1970)
    pub taken: u64,
    /// The port's line about it (level, time played, deaths)
    pub info: String,
    /// The state as deflated JSON; `None` when dropped (then a link of
    /// the tree only)
    pub state: Option<Vec<u8>>,
    /// The picture as PNG; empty when dropped
    pub picture: Vec<u8>,
    /// A save the player made (rule 6), not one taken by the clock
    pub manual: bool,
}

#[derive(Debug)]
pub struct Rewind<T> {
    nodes: Vec<Snapshot>,
    current: Option<usize>,
    /// Frames between two snapshots
    every: u64,
    /// Snapshots that keep their state at most
    max: usize,
    next_at: u64,
    /// Snapshots on disk; ones dropped since they were written
    written: usize,
    dropped: Vec<usize>,
    state: PhantomData<T>,
}

/// `index.json`
#[derive(Serialize, Deserialize)]
struct Index {
    count: usize,
    current: Option<usize>,
    next_at: u64,
}

/// `n<k>.json`
#[derive(Serialize, Deserialize)]
struct Stored {
    parent: Option<usize>,
    frame: u64,
    taken: u64,
    info: String,
    dropped: bool,
    #[serde(default)]
    manual: bool,
}

/// `state` as deflated JSON.
fn pack<T: Serialize>(state: &T) -> Vec<u8> {
    miniz_oxide::deflate::compress_to_vec(&serde_json::to_vec(state).expect("a state as JSON"), 6)
}

fn unpack<T: DeserializeOwned>(packed: &[u8]) -> Option<T> {
    let json = miniz_oxide::inflate::decompress_to_vec(packed).ok()?;
    serde_json::from_slice(&json).map_err(|e| eprintln!("a snapshot: {e}")).ok()
}

pub fn now() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_secs())
}

impl<T: Serialize + DeserializeOwned> Rewind<T> {
    /// A snapshot every `every` frames, at most `max` kept whole.
    pub fn new(every: u64, max: usize) -> Rewind<T> {
        Rewind { nodes: Vec::new(), current: None, every, max: max.max(1), next_at: 0, written: 0, dropped: Vec::new(), state: PhantomData }
    }

    /// No snapshots (a new game); the first is due at once.
    pub fn clear(&mut self) {
        self.nodes.clear();
        self.current = None;
        self.next_at = 0;
        self.written = 0;
        self.dropped.clear();
    }

    /// A snapshot is due at the game's frame `frame`.
    pub fn due(&self, frame: u64) -> bool {
        frame >= self.next_at
    }

    /// Keep a snapshot taken at `frame`; it follows the current one.
    pub fn take(&mut self, frame: u64, state: &T, picture: Vec<u8>, info: String) {
        self.keep(frame, state, picture, info, false);
    }

    /// Keep a save the player made (rule 6: at a fixed save point); shown
    /// apart in the timeline, the next snapshot by the clock follows it.
    pub fn save_point(&mut self, frame: u64, state: &T, picture: Vec<u8>, info: String) {
        self.keep(frame, state, picture, info, true);
    }

    fn keep(&mut self, frame: u64, state: &T, picture: Vec<u8>, info: String, manual: bool) {
        self.nodes.push(Snapshot { parent: self.current, frame, taken: now(), info, state: Some(pack(state)), picture, manual });
        self.current = Some(self.nodes.len() - 1);
        self.next_at = frame + self.every;
        self.trim();
    }

    /// Drop the oldest snapshots' states and pictures beyond `max`.
    fn trim(&mut self) {
        let mut whole: Vec<usize> = (0..self.nodes.len()).filter(|&k| self.nodes[k].state.is_some()).collect();
        while whole.len() > self.max {
            let k = whole.remove(0);
            self.nodes[k].state = None;
            self.nodes[k].picture = Vec::new();
            if k < self.written {
                self.dropped.push(k);
            }
        }
    }

    /// Play on from snapshot `k`: its state (none when dropped). The next
    /// snapshot follows it, `every` frames after it.
    pub fn restore(&mut self, k: usize) -> Option<T> {
        let n = self.nodes.get(k)?;
        let state = unpack(n.state.as_deref()?)?;
        self.next_at = n.frame + self.every;
        self.current = Some(k);
        Some(state)
    }

    /// The tree for ouro-ui's timeline.
    pub fn tree(&self) -> Vec<ouro_ui::timeline::Node> {
        self.nodes.iter().map(|n| ouro_ui::timeline::Node { parent: n.parent, gone: n.state.is_none(), manual: n.manual }).collect()
    }

    pub fn current(&self) -> Option<usize> {
        self.current
    }

    pub fn get(&self, k: usize) -> Option<&Snapshot> {
        self.nodes.get(k)
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    /// The timeline's line about snapshot `k`: the port's, and when.
    pub fn info(&self, k: usize) -> String {
        self.nodes.get(k).map_or_else(String::new, |n| format!("{}   {}", n.info, local_time(n.taken)))
    }

    /// Write what is new since the last save into `dir`.
    pub fn save(&mut self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        if self.written > self.nodes.len() {
            self.written = 0;
        }
        let dropped = std::mem::take(&mut self.dropped);
        for k in dropped.into_iter().chain(self.written..self.nodes.len()) {
            let n = &self.nodes[k];
            let stored = Stored { parent: n.parent, frame: n.frame, taken: n.taken, info: n.info.clone(), dropped: n.state.is_none(), manual: n.manual };
            crate::write_whole(&dir.join(format!("n{k}.json")), &serde_json::to_vec(&stored).map_err(std::io::Error::other)?)?;
            let (state, png) = (dir.join(format!("s{k}.z")), dir.join(format!("p{k}.png")));
            match &n.state {
                Some(s) => {
                    crate::write_whole(&state, s)?;
                    crate::write_whole(&png, &n.picture)?;
                }
                None => {
                    let _ = std::fs::remove_file(state);
                    let _ = std::fs::remove_file(png);
                }
            }
        }
        self.written = self.nodes.len();
        let index = Index { count: self.nodes.len(), current: self.current, next_at: self.next_at };
        crate::write_whole(&dir.join("index.json"), &serde_json::to_vec(&index).map_err(std::io::Error::other)?)
    }

    /// The snapshots saved in `dir` (none when it has none).
    pub fn load(dir: &Path, every: u64, max: usize) -> std::io::Result<Rewind<T>> {
        let mut r = Rewind::new(every, max);
        let Ok(bytes) = std::fs::read(dir.join("index.json")) else { return Ok(r) };
        let index: Index = serde_json::from_slice(&bytes).map_err(std::io::Error::other)?;
        for k in 0..index.count {
            let s: Stored = serde_json::from_slice(&std::fs::read(dir.join(format!("n{k}.json")))?).map_err(std::io::Error::other)?;
            // a snapshot whose state is missing is a link only
            let state = if s.dropped { None } else { std::fs::read(dir.join(format!("s{k}.z"))).ok() };
            let picture = if state.is_some() { std::fs::read(dir.join(format!("p{k}.png"))).unwrap_or_default() } else { Vec::new() };
            r.nodes.push(Snapshot { parent: s.parent.filter(|&p| p < k), frame: s.frame, taken: s.taken, info: s.info, state, picture, manual: s.manual });
        }
        r.current = index.current.filter(|&c| c < r.nodes.len());
        r.next_at = index.next_at;
        r.written = r.nodes.len();
        r.trim();
        Ok(r)
    }
}

/// Seconds since 1970 as the local "HH:MM" of today or "DD.MM. HH:MM".
fn local_time(unix: u64) -> String {
    // without a time zone library: the offset from the C library
    let offset = local_offset();
    let local = unix as i64 + offset;
    let today = (now() as i64 + offset).div_euclid(86_400);
    let day = local.div_euclid(86_400);
    let (h, m) = (local.rem_euclid(86_400) / 3600, local.rem_euclid(3600) / 60);
    if day == today {
        format!("{h:02}:{m:02}")
    } else {
        let (_, month, d) = civil(day);
        format!("{d:02}.{month:02}. {h:02}:{m:02}")
    }
}

/// The local time zone's offset from UTC in seconds.
fn local_offset() -> i64 {
    #[cfg(unix)]
    {
        #[repr(C)]
        struct Tm {
            sec: i32,
            min: i32,
            hour: i32,
            mday: i32,
            mon: i32,
            year: i32,
            wday: i32,
            yday: i32,
            isdst: i32,
            gmtoff: std::ffi::c_long,
            zone: *const std::ffi::c_char,
        }
        unsafe extern "C" {
            fn localtime_r(t: *const i64, tm: *mut Tm) -> *mut Tm;
        }
        let t = now() as i64;
        let mut tm = std::mem::MaybeUninit::<Tm>::zeroed();
        // SAFETY: localtime_r fills the struct it is given
        if unsafe { localtime_r(&t, tm.as_mut_ptr()) }.is_null() {
            return 0;
        }
        // SAFETY: filled above
        #[allow(clippy::useless_conversion)] // c_long is i32 on 32-bit systems
        i64::from(unsafe { tm.assume_init() }.gmtoff)
    }
    #[cfg(not(unix))]
    {
        0
    }
}

/// Days since 1970 as (year, month, day) (Howard Hinnant's algorithm).
fn civil(days: i64) -> (i64, u32, u32) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (yoe + era * 400 + i64::from(m <= 2), m, d)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn forks_restores_and_drops() {
        let mut r: Rewind<u32> = Rewind::new(100, 3);
        assert!(r.due(0));
        for f in [0, 100, 200] {
            r.take(f, &(f as u32), vec![1], format!("at {f}"));
        }
        assert!(!r.due(250) && r.due(300));
        // back to the second, play on: a fork
        assert_eq!(r.restore(1), Some(100));
        assert!(r.due(200));
        r.take(200, &999, vec![2], "fork".into());
        let tree = r.tree();
        assert_eq!((tree[3].parent, tree[2].parent), (Some(1), Some(1)));
        assert!(tree[0].gone, "the oldest dropped beyond 3");
        assert_eq!(r.restore(0), None);
        assert!(r.info(3).starts_with("fork   "));
    }

    #[test]
    fn saved_and_loaded() {
        let dir = std::env::temp_dir().join(format!("ouro-play-rewind-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let mut r: Rewind<String> = Rewind::new(10, 2);
        r.take(0, &"a".into(), vec![1, 2], "one".into());
        r.take(10, &"b".into(), vec![3], "two".into());
        r.save(&dir).unwrap();
        r.save_point(20, &"c".into(), vec![4], "three".into());
        r.save(&dir).unwrap();
        assert!(!dir.join("p0.png").exists() && !dir.join("s0.z").exists(), "dropped since: its state and picture removed");
        let mut l: Rewind<String> = Rewind::load(&dir, 10, 2).unwrap();
        assert_eq!(l.tree(), r.tree());
        assert_eq!(l.restore(2).as_deref(), Some("c"));
        assert_eq!(l.get(1).unwrap().picture, vec![3]);
        assert_eq!(l.current(), Some(2));
        assert!(l.due(30) && !l.due(29));
        assert_eq!(l.restore(0), None, "dropped");
        assert!(l.tree()[2].manual && !l.tree()[1].manual);
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(civil(0), (1970, 1, 1));
        assert_eq!(civil(20_412), (2025, 11, 20));
    }
}
