use std::io;

use clap::App;
use clap::AppSettings;
use clap::Arg;
use clap::SubCommand;
use failure::Error;

fn takes_indexes<'a, 'b>(app: App<'a, 'b>) -> App<'a, 'b> {
    app.arg(
        Arg::with_name("CAIDX")
            .help("the index file(s) to inspect")
            .required(true)
            .multiple(true),
    )
    .arg(
        Arg::with_name("store")
            .help("the castore which the indexes reference")
            .long("store")
            .required(true)
            .takes_value(true),
    )
}

fn main() -> Result<(), Error> {
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

                casync::tools::fast_export(io::stdout(), castr, caidx)?;
            }

            println!("done");
        }
        ("mtree", Some(matches)) => {
            let castr = matches.value_of("store").unwrap();
            for caidx in matches.values_of("CAIDX").unwrap() {
                casync::tools::mtree(io::stdout(), castr, caidx)?;
            }
        }
        _ => unreachable!(),
    }

    Ok(())
}
