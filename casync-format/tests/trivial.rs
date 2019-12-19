extern crate casync_format;

use std::fs;
use std::io;
use std::io::Read;

use failure::Error;

#[test]
fn load_index() -> Result<(), Error> {
    let file = io::Cursor::new(&include_bytes!("data/trivial.caidx")[..]);
    let (_sizes, v) = casync_format::read_index(file)?;

    assert_eq!(1, v.len());
    assert_eq!(0, v[0].offset);
    assert_eq!([0u8; 32], v[0].id);

    Ok(())
}

#[test]
fn two() {
    let file = &include_bytes!("data/two.catar")[..];
    let mut stream = casync_format::Stream::new(io::Cursor::new(file));
    while let Some(res) = stream.next().unwrap() {
        let (path, content) = res;
        println!("{:?} {:?}", path, content);
        if let casync_format::Content::File(mut io) = content {
            let mut buf = Vec::with_capacity(io.limit() as usize);
            io.read_to_end(&mut buf).unwrap();
        }
    }
}

/// rm -rf nums; mkdir nums && seq 10000 > nums/data && casync make --store=nums.castr nums.caidx nums
#[test]
fn load_nums() -> Result<(), Error> {
    let file = fs::File::open("tests/data/nums.caidx").unwrap();
    let (_sizes, v) = casync_format::read_index(file)?;

    let reader = casync_format::FlatReader::new(
        v.into_iter()
            .map(|chunk| chunk.read_from("tests/data/nums.castr")),
    );

    let mut paths = Vec::new();

    let mut stream = casync_format::Stream::new(reader);
    while let Some(path_content) = stream.next().unwrap() {
        let (path, content) = path_content;
        let _last = path.end().clone();
        let names: Vec<Box<[u8]>> = path.into_iter().map(|item| item.name).collect();

        paths.push(casync_format::utf8_path(names).unwrap());

        match content {
            casync_format::Content::File(mut data) => {
                let mut buf = Vec::new();
                data.read_to_end(&mut buf).unwrap();
            }
            casync_format::Content::Directory => {}
        }
    }

    assert_eq!(&["./data".to_string(), ".".to_string(),], paths.as_slice());
    Ok(())
}
