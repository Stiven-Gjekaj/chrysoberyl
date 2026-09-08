//! The one pinned font this crate ever renders text with.
//!
//! This module holds the only `include_bytes!` call in this workspace.
//! `pinned_fontdb` builds a `fontdb::Database` that starts empty and is
//! filled with exactly this crate's own embedded bytes, so the resulting
//! database can name no font this repository did not commit.

use resvg::usvg::fontdb;

/// The embedded bytes of `NotoSans-Regular.ttf`.
///
/// The path is relative to this file's own directory
/// (`crates/chrys-source-svg/src`), not to the crate's manifest: that is
/// the one thing about `include_bytes!` that does not follow Cargo's usual
/// convention, and the one thing that breaks the build on the first try
/// when it is written the other way.
static NOTO_SANS_REGULAR: &[u8] = include_bytes!("../fonts/NotoSans-Regular.ttf");

/// Build a font database that holds exactly one face: the pinned
/// `NotoSans-Regular.ttf` this crate embeds at compile time.
///
/// `fontdb::Database::new()` is documented as starting empty, and this
/// function loads the embedded bytes into it exactly once, so the
/// database this function returns can name no font beyond the one this
/// repository committed. No path is read at run time, and this crate's
/// `resvg` dependency carries no `system-fonts` feature, so the host
/// machine's own font database is never reachable from here at all.
///
/// `usvg`'s own font query appends the database's generic "serif" family
/// as its last fallback candidate, checked only after every family a
/// `font-family` attribute named has failed to resolve. A database with
/// no generic family mapping of its own resolves that fallback to
/// nothing, so a `<text>` naming a family this one-font database does
/// not carry would render no glyph at all rather than the pinned one.
/// Every generic family this database can carry is mapped to the pinned
/// font's own name here, so every one of `usvg`'s fallback paths, not
/// only the no-`font-family`-at-all path `usvg::Options::font_family`
/// covers, still lands on this repository's one committed font.
pub fn pinned_fontdb() -> fontdb::Database {
    let mut db = fontdb::Database::new();
    db.load_font_data(NOTO_SANS_REGULAR.to_vec());
    let family = pinned_family_name(&db);
    db.set_serif_family(family.clone());
    db.set_sans_serif_family(family.clone());
    db.set_cursive_family(family.clone());
    db.set_fantasy_family(family.clone());
    db.set_monospace_family(family);
    db
}

/// Read the pinned font's own family name out of the database `db` built,
/// rather than typing that name a second time from memory as a string
/// literal.
///
/// Panics when `db` holds no face at all, which only happens if a caller
/// passes a database other than one `pinned_fontdb` built.
pub fn pinned_family_name(db: &fontdb::Database) -> String {
    let face = db
        .faces()
        .next()
        .expect("pinned_fontdb always loads exactly one face");
    face.families
        .first()
        .expect("a TrueType face always names at least one family")
        .0
        .clone()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_fontdb_holds_exactly_one_face() {
        let db = pinned_fontdb();
        assert_eq!(db.len(), 1);
    }

    #[test]
    fn pinned_family_name_reads_a_non_empty_name() {
        let db = pinned_fontdb();
        let name = pinned_family_name(&db);
        assert!(!name.is_empty());
    }
}
