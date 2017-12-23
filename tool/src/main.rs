extern crate casync_format;
extern crate clap;
#[macro_use]
extern crate error_chain;

mod errors;

use std::fs;
use std::io;
use std::io::Read;

use casync_format::Chunk;
use clap::{App, AppSettings, Arg, SubCommand};

use errors::*;

fn takes_indexes<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name("CAIDX")
            .help("the index file(s) to inspect")
            .required(true)
            .multiple(true),
    ).arg(
        Arg::with_name("store")
            .help("the castore which the indexes reference")
            .long("store")
            .required(true)
            .takes_value(true),
    )
}

fn run() -> Result<()> {
    let matches = App::new("casync-rs")
        .setting(AppSettings::SubcommandRequired)
        .subcommand(takes_indexes(
            SubCommand::with_name("fast-export")
                .about("fast-export some archives")
                .arg(
                    Arg::with_name("ref-prefix")
                        .help("prefix for ref; index of argument appended")
                        .long("ref-prefix")
                        .required(true)
                        .takes_value(true),
                ),
        ))
        .subcommand(takes_indexes(
            SubCommand::with_name("mtree").about("dump data about some archives"),
        ))
        .get_matches();

    match matches.subcommand() {
        ("fast-export", Some(matches)) => {
            let castr = matches.value_of("store").unwrap();
            let ref_prefix = matches.value_of("ref-prefix").unwrap();
            println!("feature done");

            for (nth, caidx) in matches.values_of("CAIDX").unwrap().enumerate() {
                println!("# {}", caidx);
                println!("commit {}{}", ref_prefix, nth);

                // TODO: recover dates or even authors
                println!("committer casync-rs <solo-casync-rs@goeswhere.com> 0 +0000");

                // commit message: 0 bytes
                println!("data 0");
                println!();
                println!("deleteall");

                fast_export(castr, caidx)?;
            }

            println!("done");
        }
        ("mtree", Some(matches)) => {
            let castr = matches.value_of("store").unwrap();
            for caidx in matches.values_of("CAIDX").unwrap() {
                mtree(castr, caidx)?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}

fn fast_export(castr: &str, caidx: &str) -> Result<()> {
    let file = fs::File::open(caidx).chain_err(|| "opening index file")?;

    let mut v: Vec<Chunk> = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).chain_err(|| "reading index file")?;

    let mut it = v.into_iter();

    let reader = casync_format::ChunkReader::new(|| {
        Ok(match it.next() {
            Some(chunk) => Some(chunk.open_from(castr)?),
            None => None,
        })
    }).chain_err(|| "initialising reader")?;

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    casync_format::read_stream(reader, |path, entry, data| {
        if entry.is_dir() {
            ensure!(data.is_none(), "data for directory");
            return Ok(());
        }

        let mut data = data.ok_or("non-directory has no contents")?;

        ensure!(
            entry.is_reg(),
            "only directories and regular files are supported"
        );
        let executable = 0o100 == (entry.mode & 0o100);

        println!(
            "M {} inline {}",
            if executable { "100755" } else { "100644" },
            casync_format::utf8_path(path)?
        );
        println!("data {}", data.limit());
        io::copy(&mut data, &mut io::stdout())?;

        Ok(())
    }).chain_err(|| format!("reading stream of index {}", caidx))
}

fn mtree(castr: &str, caidx: &str) -> Result<()> {
    let file = fs::File::open(caidx).chain_err(|| "opening index file")?;

    let mut v: Vec<Chunk> = vec![];
    casync_format::read_index(file, |chunk| {
        v.push(chunk);
        Ok(())
    }).chain_err(|| "reading index file")?;

    let mut it = v.into_iter();

    let reader = casync_format::ChunkReader::new(|| {
        Ok(match it.next() {
            Some(chunk) => Some(chunk.open_from(castr)?),
            None => None,
        })
    }).chain_err(|| "initialising reader")?;

    //    io::copy(&mut reader, &mut fs::File::create("a").unwrap()).unwrap();

    casync_format::read_stream(reader, |path, entry, data| {
        println!("{}, {:?}", casync_format::utf8_path(path)?, entry);
        let mut buf = vec![];
        if data.is_some() {
            data.unwrap().read_to_end(&mut buf)?;
        }
        Ok(())
    }).chain_err(|| format!("reading stream of index {}", caidx))
}

quick_main!(run);
