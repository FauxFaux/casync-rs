use std;
use std::io;
use std::io::Read;

use failure::ensure;
use failure::Error;

use super::fetcher::Fetcher;
use super::read_index;
use super::Chunk;
use super::FlatReader;

/// guess the `.castr` (relative) path from the `.caidx` path, and fetch both
pub fn from_index<F: 'static + Fetcher>(idx: &str, fetcher: F) -> Result<impl Read, Error> {
    ensure!(
        idx.ends_with(".caidx"),
        "index must have a .caidx extension, not {:?}",
        idx
    );
    let prefix = format!("{}.castr", &idx[..idx.len() - ".caidx".len()]);
    from_paths(idx, prefix, fetcher)
}

/// use the explicit `caidx` and `castr` paths, and fetch both
pub fn from_paths<F: 'static + Fetcher>(
    idx: &str,
    store: impl ToString,
    mut fetcher: F,
) -> Result<impl Read, Error> {
    let (_sizes, chunks) = read_index(io::Cursor::new(fetcher.fetch(idx)?))?;
    let store = store.to_string();
    Ok(from_chunks(chunks, move |cacnk: &str| {
        fetcher.fetch(&format!("{}/{}", store, cacnk))
    }))
}

/// use a pre-fetched `index` and pre-configured `fetcher`
/// which can fetch chunks given `abcd/abcdefg012[..]30.cacnk`.
pub fn from_chunks<F: 'static + Fetcher>(chunks: Vec<Chunk>, mut fetcher: F) -> impl Read {
    FlatReader::new(chunks.into_iter().map(move |c| -> Result<_, io::Error> {
        let fetched = fetcher.fetch(&c.format_id())?;
        let fetched = zstd::stream::decode_all(io::Cursor::new(fetched))?;
        c.check(&fetched)?;
        Ok(fetched)
    }))
}
