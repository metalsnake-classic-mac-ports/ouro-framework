//! Continue's state (the ports' rule 4): the game as it was left, as JSON
//! in a wrapper that names its format, `{"format": N, "game": ...}`. A
//! port raises its format when its state changes in a way an older file
//! cannot be read as, and upgrades what it can. A file that cannot be read
//! is put aside (`.unreadable`), so the next save does not overwrite it
//! and the menu offers no Continue that fails.

use std::path::Path;

use serde::Serialize;
use serde::de::DeserializeOwned;

#[derive(Serialize)]
struct Out<'a, T> {
    format: u32,
    game: &'a T,
}

/// Save `game` in `format` to `path`.
pub fn save<T: Serialize>(path: &Path, format: u32, game: &T) -> std::io::Result<()> {
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir)?;
    }
    let bytes = serde_json::to_vec(&Out { format, game }).map_err(std::io::Error::other)?;
    crate::write_whole(path, &bytes)
}

/// The state in `path`: `upgrade` gets its format and JSON and returns the
/// game in the current format, or an error (then the file is put aside).
/// `None` when there is no file.
pub fn load<T: DeserializeOwned>(path: &Path, upgrade: impl FnOnce(u32, serde_json::Value) -> Result<serde_json::Value, String>) -> Option<T> {
    let bytes = std::fs::read(path).ok()?;
    let read = || -> Result<T, String> {
        let mut v: serde_json::Value = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        let format = v.get("format").and_then(serde_json::Value::as_u64).ok_or("no format")? as u32;
        let game = v.get_mut("game").map(serde_json::Value::take).ok_or("no game")?;
        serde_json::from_value(upgrade(format, game)?).map_err(|e| e.to_string())
    };
    match read() {
        Ok(game) => Some(game),
        Err(e) => {
            keep_aside(path);
            eprintln!("{}: {e}; kept as .unreadable", path.display());
            None
        }
    }
}

/// Put `path` aside as `<path>.unreadable`.
pub fn keep_aside(path: &Path) {
    let mut aside = path.as_os_str().to_owned();
    aside.push(".unreadable");
    let _ = std::fs::rename(path, aside);
}

/// Only the current format is read.
pub fn only(current: u32) -> impl FnOnce(u32, serde_json::Value) -> Result<serde_json::Value, String> {
    move |format, game| if format == current { Ok(game) } else { Err(format!("format {format}, not {current}")) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_and_unreadable() {
        let dir = std::env::temp_dir().join(format!("ouro-play-saved-{}", std::process::id()));
        let path = dir.join("continue.json");
        save(&path, 2, &vec![1, 2, 3]).unwrap();
        assert_eq!(load::<Vec<i32>>(&path, only(2)), Some(vec![1, 2, 3]));
        assert_eq!(load::<Vec<i32>>(&path, only(3)), None);
        assert!(!path.exists() && dir.join("continue.json.unreadable").exists());
        assert_eq!(load::<Vec<i32>>(&path, only(3)), None, "no file");
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
