//! What the Ouro ports keep of play, with their files (ouro-ui holds the
//! screens and does no I/O; this crate keeps what they show):
//!
//! - [`records`]: the level selection's records (rule 3): per level the
//!   first, the best and the latest done run with its deaths;
//! - [`rewind`]: rewind's snapshots as a tree (rule 5), kept on disk with
//!   Continue's state;
//! - [`saved`]: Continue's state in a file that names its format
//!   (rule 4);
//! - [`picture`]: small pictures as PNG (the snapshots' and the level
//!   selection's previews).
//!
//! The port gives the paths (SDL's pref folder) and its own state type;
//! what counts as a run, a death, a snapshot's state is the game's.

pub mod picture;
pub mod records;
pub mod rewind;
pub mod saved;

/// Write `bytes` to `path` through a file beside it, so a crash while
/// writing leaves the old file whole.
pub fn write_whole(path: &std::path::Path, bytes: &[u8]) -> std::io::Result<()> {
    let part = path.with_extension("part");
    std::fs::write(&part, bytes)?;
    std::fs::rename(&part, path)
}
