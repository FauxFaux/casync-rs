use std;
use std::convert::TryFrom;
use std::fmt;
use std::io;
use std::io::Read;

use failure::ensure;
use failure::err_msg;
use failure::Error;

use super::fetcher::Fetcher;
use super::format::StreamMagic;
use super::Chunk;
use crate::{read_index, FlatReader};

const HEADER_TAG_LEN: u64 = 16;
const RECORD_SIZE_LIMIT: u64 = 64 * 1024;

pub struct Stream<R: Read> {
    inner: R,
    path: Path,
}

#[derive(Debug, Clone)]
pub struct Path {
    inner: Vec<Item>,
}

#[derive(Clone)]
pub struct Item {
    pub name: Box<[u8]>,
    pub entry: Option<Entry>,
}

#[derive(Clone)]
pub struct Entry {
    pub mode: u64,
    pub uid: u64,
    pub gid: u64,
    pub mtime: u64,
    pub feature_flags: u64,
    pub flags: u64,
    pub user_name: Option<Box<[u8]>>,
    pub group_name: Option<Box<[u8]>>,
}

impl Item {
    fn new(name: &str) -> Item {
        Item {
            name: name.to_string().into_bytes().into_boxed_slice(),
            entry: None,
        }
    }
}

impl fmt::Debug for Item {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Item {{ {:?}, {:?} }}",
            String::from_utf8_lossy(self.name.as_ref()),
            self.entry,
        )
    }
}

impl fmt::Debug for Entry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "Entry {{ 0o{:o}{} u:{} g:{} user: {:?} group: {:?} }}",
            self.mode & 0o7777,
            if self.is_dir() {
                "d"
            } else if self.is_reg() {
                "r"
            } else {
                " XXX"
            },
            self.uid,
            self.gid,
            self.user_name,
            self.group_name
        )
    }
}

impl Entry {
    pub fn is_dir(&self) -> bool {
        0o040000 == (self.mode & 0o170000)
    }

    pub fn is_reg(&self) -> bool {
        0o100000 == (self.mode & 0o170000)
    }
}

#[derive(Clone, Debug)]
enum ItemType {
    File(u64),
    Directory,
}

#[derive(Debug)]
pub enum Content<'r, R: 'r> {
    File(io::Take<&'r mut R>),
    Directory,
}

impl ItemType {
    fn into_content<'r, R: 'r + Read, F>(self, take: F) -> Content<'r, R>
    where
        F: FnOnce(u64) -> io::Take<&'r mut R>,
    {
        match self {
            ItemType::File(len) => Content::File(take(len)),
            ItemType::Directory => Content::Directory,
        }
    }
}

impl Stream<Box<dyn Read>> {
    pub fn from_index<F: 'static + Fetcher>(
        idx: &str,
        fetcher: F,
    ) -> Result<Stream<Box<dyn Read>>, Error> {
        ensure!(
            idx.ends_with(".caidx"),
            "index must have a .caidx extension, not {:?}",
            idx
        );
        let prefix = format!("{}.castr", &idx[..idx.len() - ".caidx".len()]);
        Self::from_paths(idx, prefix, fetcher)
    }

    pub fn from_paths<F: 'static + Fetcher>(
        idx: &str,
        store: impl ToString,
        mut fetcher: F,
    ) -> Result<Stream<Box<dyn Read>>, Error> {
        let (_sizes, chunks) = read_index(io::Cursor::new(fetcher.fetch(idx)?))?;
        let store = store.to_string();
        Ok(Self::from_chunks(chunks, move |cacnk: &str| {
            fetcher.fetch(&format!("{}/{}", store, cacnk))
        }))
    }

    /// `fetcher` fetches chunks, given `abcd/efg012[..]30.cacnk`.
    pub fn from_chunks<F: 'static + Fetcher>(
        chunks: Vec<Chunk>,
        mut fetcher: F,
    ) -> Stream<Box<dyn Read>> {
        Stream::new(Box::new(FlatReader::new(chunks.into_iter().map(
            move |c| -> Result<_, io::Error> {
                let fetched = fetcher.fetch(&c.format_id())?;
                let fetched = zstd::stream::decode_all(io::Cursor::new(fetched))?;
                c.check(&fetched)?;
                Ok(fetched)
            },
        ))))
    }
}

impl<R: Read> Stream<R> {
    pub fn new(inner: R) -> Stream<R> {
        Stream {
            inner,
            path: Path::at_dot(),
        }
    }

    pub fn into_inner(self) -> R {
        self.inner
    }

    pub fn next(&mut self) -> Result<Option<(Path, Content<R>)>, Error> {
        if self.path.is_empty() {
            return Ok(None);
        }

        process_item(&mut self.inner, &mut self.path).map(move |item| {
            let copy = self.path.clone();
            self.path.pop();
            Some((
                copy,
                item.into_content(move |limit| (&mut self.inner).take(limit)),
            ))
        })
    }
}

impl<R: Read> AsRef<R> for Stream<R> {
    fn as_ref(&self) -> &R {
        &self.inner
    }
}

impl<R: Read> AsMut<R> for Stream<R> {
    fn as_mut(&mut self) -> &mut R {
        &mut self.inner
    }
}

impl Path {
    fn at_dot() -> Path {
        Path {
            inner: vec![Item::new(".")],
        }
    }

    fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    fn push(&mut self, item: Item) {
        self.inner.push(item);
    }

    fn pop(&mut self) {
        self.inner.pop();
    }

    pub fn end(&self) -> &Item {
        let end = self.inner.len() - 1;
        &self.inner[end]
    }

    fn end_entry(&mut self) -> &mut Option<Entry> {
        let end = self.inner.len() - 1;
        &mut self.inner[end].entry
    }

    pub fn into_iter(self) -> ::std::vec::IntoIter<Item> {
        self.inner.into_iter()
    }
}

fn process_item<R: Read>(mut from: &mut R, path: &mut Path) -> Result<ItemType, Error> {
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;

        match header_format {
            StreamMagic::Entry => {
                ensure!(
                    (8 * 6) + HEADER_TAG_LEN == header_size,
                    "incorrect ENTRY length; not supported by us: 48 != {}",
                    header_size
                );

                let end = path.end_entry();

                ensure!(end.is_none(), "entry found without data");
                *end = Some(load_entry(&mut from)?);
            }
            StreamMagic::User => {
                path.end_entry()
                    .as_mut()
                    .ok_or_else(|| err_msg("user without entry"))?
                    .user_name =
                    Some(read_string_record(header_size, &mut from)?.into_boxed_slice());
            }
            StreamMagic::Group => {
                path.end_entry()
                    .as_mut()
                    .ok_or_else(|| err_msg("group without entry"))?
                    .group_name =
                    Some(read_string_record(header_size, &mut from)?.into_boxed_slice());
            }
            StreamMagic::Name => {
                let new_name = read_string_record(header_size, &mut from)?;

                ensure!(!new_name.is_empty(), "filename must be non-empty");

                path.push(Item {
                    name: new_name.into_boxed_slice(),
                    entry: None,
                });
            }
            StreamMagic::Data => {
                ensure!(
                    header_size >= HEADER_TAG_LEN,
                    "data <0 bytes long: {}",
                    header_size
                );
                return Ok(ItemType::File(header_size - HEADER_TAG_LEN));
            }
            StreamMagic::Bye => {
                // TODO: all kinds of tailing records
                read_data_record(header_size, &mut from)?;
                return Ok(ItemType::Directory);
            }
        }
    }
}

fn load_entry<R: Read>(mut from: R) -> Result<Entry, Error> {
    Ok(Entry {
        feature_flags: leu64(&mut from)?,
        mode: leu64(&mut from)?,
        flags: leu64(&mut from)?,
        uid: leu64(&mut from)?,
        gid: leu64(&mut from)?,
        mtime: leu64(&mut from)?,
        // these are filled in by following packets
        user_name: None,
        group_name: None,
    })
}

pub fn dump_packets<R: Read>(mut from: R) -> Result<(), Error> {
    let mut depth = 0usize;
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;

        let payload_len = header_size - 16;
        let mut payload = vec![0; usize::try_from(payload_len)?];
        from.read_exact(&mut payload)?;
        print!(
            "{} * {:5} | {:3} | ",
            String::from_utf8(vec![b' '; depth * 2]).unwrap(),
            format!("{:?}", header_format),
            payload_len
        );

        match header_format {
            StreamMagic::Entry => {
                let entry = load_entry(io::Cursor::new(&payload))?;
                println!("dir: {}", entry.is_dir());
            }
            StreamMagic::Data => {
                println!();

                depth -= 1;
            }
            StreamMagic::Name => {
                println!("{}", String::from_utf8_lossy(&payload[..payload.len() - 1]));
                depth += 1;
            }
            StreamMagic::Bye => {
                println!();
                depth -= 1;

                if 0 == depth {
                    return Ok(());
                }
            }
            _ => {
                println!("{}", String::from_utf8_lossy(&payload[..payload.len() - 1]));
            }
        }
    }
}

fn read_string_record<R: Read>(header_size: u64, from: R) -> Result<Vec<u8>, Error> {
    match read_data_record(header_size, from) {
        Ok(ref vec) if vec.is_empty() => Ok(Vec::new()),
        Ok(mut vec) => {
            ensure!(
                vec[vec.len() - 1] == 0,
                "string record must be null-terminated"
            );
            vec.pop();
            Ok(vec)
        }
        Err(e) => Err(e),
    }
}

fn read_data_record<R: Read>(header_size: u64, mut from: R) -> Result<Vec<u8>, Error> {
    ensure!(
        header_size >= HEADER_TAG_LEN,
        "header missing / size wrong: {}",
        header_size
    );

    ensure!(
        header_size < RECORD_SIZE_LIMIT + HEADER_TAG_LEN,
        "refusing to support records over ~64kB, was: {}",
        header_size
    );

    let mut buf = vec![0u8; (header_size - HEADER_TAG_LEN) as usize];
    from.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn utf8_path(
    from: Vec<Box<[u8]>>,
) -> std::result::Result<String, ::std::string::FromUtf8Error> {
    let mut ret = String::new();
    for component in from {
        ret.push_str(String::from_utf8(component.into_vec())?.as_str());
        ret.push_str("/");
    }

    if !ret.is_empty() {
        let waste = ret.len() - 1;
        ret.truncate(waste);
    }

    Ok(ret)
}

fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    let mut buf = [0u8; 8];
    from.read_exact(&mut buf)?;
    Ok(u64::from_le_bytes(buf))
}
