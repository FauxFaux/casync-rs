use std::io;

use anyhow::Error;
use clap::Args;
use clap::Parser;
use clap::Subcommand;

#[derive(Parser)]
#[command(name = "casync-rs")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// fast-export some archives
    FastExport {
        #[command(flatten)]
        indexes: Indexes,

        /// prefix for ref; index of argument appended
        #[arg(long)]
        ref_prefix: String,
    },

    /// dump data about some archives
    Mtree {
        #[command(flatten)]
        indexes: Indexes,
    },
}

#[derive(Args)]
struct Indexes {
    /// the index file(s) to inspect
    #[arg(required = true)]
    caidx: Vec<String>,

    /// the castore which the indexes reference
    #[arg(long)]
    store: String,
}

fn main() -> Result<(), Error> {
    let cli = Cli::parse();

    match cli.command {
        Command::FastExport {
            indexes,
            ref_prefix,
        } => {
            println!("feature done");

            for (nth, caidx) in indexes.caidx.iter().enumerate() {
                println!("# {}", caidx);
                println!("commit {}{}", ref_prefix, nth);

                // TODO: recover dates or even authors
                println!("committer casync-rs <solo-casync-rs@goeswhere.com> 0 +0000");

                // commit message: 0 bytes
                println!("data 0");
                println!();
                println!("deleteall");

                casync::tools::fast_export(io::stdout(), &indexes.store, caidx)?;
            }

            println!("done");
        }
        Command::Mtree { indexes } => {
            for caidx in &indexes.caidx {
                casync::tools::mtree(io::stdout(), &indexes.store, caidx)?;
            }
        }
    }

    Ok(())
}
