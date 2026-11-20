//! The level selection's records (the ports' rule 3, as Quagmire keeps
//! them): per level the first, the best and the latest done run, each with
//! its time (in the game's frames) and deaths. What a run is - from when to
//! when, what counts as done - is the game's to say; this keeps and shows
//! them. Kept as JSON in a file the port names; an unreadable file is put
//! aside (`.unreadable`), not overwritten.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use ouro_ui::Ink;
use serde::{Deserialize, Serialize};

/// A done run: its length in the game's frames, its deaths.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Time {
    pub frames: u32,
    pub deaths: u32,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Record {
    pub first: Time,
    pub best: Time,
    pub latest: Time,
}

#[derive(Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Records {
    /// By level number
    pub levels: BTreeMap<i32, Record>,
    /// The game's frames a second (for the times shown)
    #[serde(skip)]
    rate: u32,
    #[serde(skip)]
    path: Option<PathBuf>,
}

impl Records {
    /// The records in `path` (none yet when it is missing), for a game of
    /// `rate` frames a second.
    pub fn load(path: Option<PathBuf>, rate: u32) -> Records {
        let mut r = match path.as_deref().map(std::fs::read) {
            Some(Ok(bytes)) => serde_json::from_slice(&bytes).unwrap_or_else(|e| {
                let p = path.as_deref().expect("read from it");
                let aside = p.with_extension("json.unreadable");
                let _ = std::fs::rename(p, &aside);
                eprintln!("{}: {e}; kept as {}", p.display(), aside.display());
                Records::default()
            }),
            _ => Records::default(),
        };
        r.rate = rate.max(1);
        r.path = path;
        r
    }

    fn save(&self) {
        let Some(p) = &self.path else { return };
        let bytes = serde_json::to_vec_pretty(self).expect("records to JSON");
        if let Err(e) = crate::write_whole(p, &bytes) {
            eprintln!("{}: {e}", p.display());
        }
    }

    /// A done run of `level`: recorded and saved.
    pub fn add(&mut self, level: i32, frames: u32, deaths: u32) {
        self.record(level, Time { frames, deaths });
        self.save();
    }

    fn record(&mut self, level: i32, t: Time) {
        let r = self.levels.entry(level).or_insert(Record { first: t, best: t, latest: t });
        if (t.frames, t.deaths) < (r.best.frames, r.best.deaths) {
            r.best = t;
        }
        r.latest = t;
    }

    /// Level `level` was done at least once.
    pub fn done(&self, level: i32) -> bool {
        self.levels.contains_key(&level)
    }

    /// The level selection's lines about `level` (under its preview).
    pub fn lines(&self, level: i32) -> Vec<(String, Ink)> {
        match self.levels.get(&level) {
            Some(r) => {
                let t = |t: Time| format!("{}  ({} {})", clock(t.frames, self.rate), t.deaths, if t.deaths == 1 { "death" } else { "deaths" });
                vec![(format!("Best:    {}", t(r.best)), Ink::Accent), (format!("First:   {}", t(r.first)), Ink::Normal), (format!("Latest:  {}", t(r.latest)), Ink::Normal)]
            }
            None => vec![("Not done yet".into(), Ink::Dim)],
        }
    }

    pub fn path(&self) -> Option<&Path> {
        self.path.as_deref()
    }
}

/// m:ss of `frames` at `rate` a second.
pub fn clock(frames: u32, rate: u32) -> String {
    let s = frames / rate.max(1);
    format!("{}:{:02}", s / 60, s % 60)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn first_best_latest_and_the_file() {
        let dir = std::env::temp_dir().join(format!("ouro-play-records-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("records.json");
        let mut r = Records::load(Some(path.clone()), 60);
        for (frames, deaths) in [(900, 2), (600, 0), (750, 1)] {
            r.add(3, frames, deaths);
        }
        let e = Records::load(Some(path.clone()), 60).levels[&3];
        assert_eq!((e.first.frames, e.best.frames, e.latest.frames), (900, 600, 750));
        assert!(r.done(3) && !r.done(4));
        assert_eq!(r.lines(3)[0].0, "Best:    0:10  (0 deaths)");
        std::fs::write(&path, b"not json").unwrap();
        assert!(Records::load(Some(path.clone()), 60).levels.is_empty());
        assert!(dir.join("records.json.unreadable").exists(), "kept aside");
        std::fs::remove_dir_all(&dir).unwrap();
        assert_eq!(clock(3660, 60), "1:01");
    }
}
