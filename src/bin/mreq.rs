/*
 * src/bin/mreq.rs: read log entries from various Manta components and put
 * together a request timeline
 */

use std::process;

/* Name of this program (used for error messages) */
const ARG0 : &str = "mreq";

/* Standard process exit codes */
const EXIT_FAILURE : i32 = 1;
const EXIT_USAGE : i32 = 2;

extern crate manta_mreq;
use manta_mreq::MantaLogParserInput;
use manta_mreq::mri_dump;
use manta_mreq::mri_parse_files;

fn main()
{
    let argv : Vec<String> = std::env::args().collect();
    if argv.len() != 2 {
        usage();
    }

    let input = MantaLogParserInput {
        mli_muskie_filename: argv[1].to_string()
    };

    match mri_parse_files(&input) {
        Ok(mli) => mri_dump(&mli),
        Err(error) => fatal(error)
    }
}

fn usage()
{
    eprintln!("usage: {} MUSKIE_LOG", ARG0);
    process::exit(EXIT_USAGE);
}

fn fatal(error : String)
{
    eprintln!("{}: {}", ARG0, error);
    process::exit(EXIT_FAILURE);
}
