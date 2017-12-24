use std;
use std::io;
use std::io::Read;
use std::fmt;

use errors::*;
use format::StreamMagic;

use byteorder::{LittleEndian, ReadBytesExt};

const HEADER_TAG_LEN: u64 = 16;

#[derive(Clone)]
pub struct Item {
    pub name: Box<[u8]>,
    pub entry: Option<Entry>,
}

#[derive(Clone, Default)]
pub struct Entry {
    pub mode: u64,
    pub uid: u64,
    pub gid: u64,
    pub mtime: u64,
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

pub struct Stream<R: Read> {
    inner: R,
    path: Vec<Item>,
}

impl<R: Read> Stream<R> {
    pub fn new(inner: R) -> Stream<R> {
        Stream {
            inner,
            path: vec![Item::new(".")],
        }
    }

    pub fn as_ref(&self) -> &R {
        &self.inner
    }

    pub fn as_mut(&mut self) -> &mut R {
        &mut self.inner
    }

    pub fn into_inner(self) -> R {
        self.inner
    }
}

#[derive(Clone, Debug)]
pub enum ItemType {
    File(u64),
    Directory,
}

impl<R: Read> Iterator for Stream<R> {
    type Item = Result<(Vec<Item>, ItemType)>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.path.is_empty() {
            return None;
        }
        match process_item(&mut self.inner, &mut self.path) {
            Ok(item) => {
                let copy = self.path.clone();
                self.path.pop();
                Some(Ok((copy, item)))
            },
            Err(e) => Some(Err(e)),
        }
    }
}

pub fn process_item<R: Read>(mut from: &mut R, path: &mut Vec<Item>) -> Result<ItemType> {
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;
        //println!("header: {:?}", header_format);
        match header_format {
            StreamMagic::Entry => {
                ensure!(
                    (8 * 6) + HEADER_TAG_LEN == header_size,
                    "incorrect ENTRY length; not supported by us: 48 != {}",
                    header_size
                );

                // BORROW CHECKER
                let end = path.len() - 1;
                let end = &mut path[end];

                ensure!(end.entry.is_none(), "entry found without data");
                end.entry = Some(load_entry(&mut from)?);
            }
            StreamMagic::User => {
                // BORROW CHECKER
                let end = path.len() - 1;
                let end = &mut path[end];

                end.entry.as_mut().ok_or("user without entry")?.user_name =
                    Some(read_string_record(header_size, &mut from)?.into_boxed_slice());
            }
            StreamMagic::Group => {
                // BORROW CHECKER
                let end = path.len() - 1;
                let end = &mut path[end];

                end.entry.as_mut().ok_or("group without entry")?.group_name =
                    Some(read_string_record(header_size, &mut from)?.into_boxed_slice());
            }
            StreamMagic::Name => {
                let mut new_name = read_data_record(header_size, &mut from)?;

                ensure!(
                    new_name
                        .pop()
                        .map(|last_char| 0 == last_char)
                        .unwrap_or(false),
                    "filename must be non-empty and null-terminated"
                );

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

fn load_entry<R: Read>(mut from: R) -> Result<Entry> {
    let mut entry = Entry::default();

    leu64(&mut from)?; // feature_flags

    entry.mode = leu64(&mut from)?;

    leu64(&mut from)?; // flags

    entry.uid = leu64(&mut from)?;
    entry.gid = leu64(&mut from)?;
    entry.mtime = leu64(&mut from)?;

    Ok(entry)
}

pub fn dump_packets<R: Read>(mut from: R) -> Result<()> {
    let mut in_entry = false;
    let mut depth = 0usize;
    loop {
        let header_size = leu64(&mut from)?;
        let header_format = StreamMagic::from(leu64(&mut from)?)?;

        let payload_len = header_size - 16;
        let mut payload = vec![0; usize_from(payload_len)];
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

fn read_string_record<R: Read>(header_size: u64, from: R) -> Result<Vec<u8>> {
    ensure!(
        header_size < 256 + HEADER_TAG_LEN,
        "refusing to support names over ~255 characters, was: {}",
        header_size
    );
    read_data_record(header_size, from)
}

fn read_data_record<R: Read>(header_size: u64, mut from: R) -> Result<Vec<u8>> {
    ensure!(
        header_size >= HEADER_TAG_LEN,
        "header missing / size wrong: {}",
        header_size
    );

    let mut buf = vec![0u8; (header_size - HEADER_TAG_LEN) as usize];
    from.read_exact(&mut buf)?;
    Ok(buf)
}

pub fn utf8_path(from: &[Vec<u8>]) -> std::result::Result<String, ::std::string::FromUtf8Error> {
    let mut ret = String::new();
    for component in from {
        ret.push_str(String::from_utf8(component.clone())?.as_str());
        ret.push_str("/");
    }

    if !ret.is_empty() {
        let waste = ret.len() - 1;
        ret.truncate(waste);
    }

    Ok(ret)
}

fn leu64<R: Read>(mut from: R) -> io::Result<u64> {
    from.read_u64::<LittleEndian>()
}

fn usize_from(val: u64) -> usize {
    assert!(val <= std::usize::MAX as u64);
    val as usize
}
