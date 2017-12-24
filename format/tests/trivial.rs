extern crate casync_format;

use std::io;
use std::fs;

use std::io::Read;

#[test]
fn load_index() {
    let file = io::Cursor::new(&include_bytes!("data/trivial.caidx")[..]);
    let mut v = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).unwrap();

    for chunk in v {
        println!("{:?}", chunk)
    }
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
            io.read_to_end(&mut buf);
        }
    }
}

/// rm -rf nums; mkdir nums && seq 10000 > nums/data && casync make --store=nums.castr nums.caidx nums
#[test]
fn load_nums() {
    let file = fs::File::open("tests/data/nums.caidx").unwrap();
    let mut v = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).unwrap();

    let mut it = v.into_iter();

    let reader = casync_format::ChunkReader::new(|| {
        Ok(it.next()
            .map(|chunk| chunk.open_from("tests/data/nums.castr").unwrap()))
    }).unwrap();

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    unimplemented!()
    /*
    , |path, entry, data| {
        println!("{}, {:?}", path.len(), entry);
        let mut buf = vec![];
        if data.is_some() {
            data.unwrap().read_to_end(&mut buf)?;
        }
        Ok(())
    }).unwrap();
    */
}
