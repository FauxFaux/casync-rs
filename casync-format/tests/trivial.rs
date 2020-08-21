use std::fs;
use std::io;
use std::io::Read;

use anyhow::Error;

use casync_format::chunks::from_index;
use casync_format::Stream;

#[test]
fn load_index() -> Result<(), Error> {
    let file = io::Cursor::new(&include_bytes!("data/trivial.caidx")[..]);
    let (_sizes, v) = casync_format::read_index(file)?;

    assert_eq!(1, v.len());
    assert_eq!(
        368, v[0].offset,
        "don't really know, shouldn't this be 0? Maybe it's the length."
    );
    assert_eq!(
        [
            134, 7, 242, 234, 232, 49, 36, 50, 105, 198, 119, 143, 240, 131, 31, 201, 215, 103,
            135, 18, 159, 231, 98, 22, 20, 141, 128, 46, 184, 212, 106, 57
        ],
        v[0].id
    );

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
    let mut paths = Vec::new();

    let mut stream = Stream::new(from_index("tests/data/nums.caidx", |path: &str| {
        fs::read(path)
    })?);
    while let Some((path, content)) = stream.next().unwrap() {
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
