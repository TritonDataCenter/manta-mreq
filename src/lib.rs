/*
 * src/lib.rs: entry point for library use
 */

extern crate serde;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod log_common;
mod log_muskie;

pub use log_common::mri_read_file;
pub use log_muskie::mri_parse_muskie_file;
pub use log_muskie::MuskieLogEntry;

/*
 * Represents validated end-user input.
 */
pub struct MantaLogParserInput {
    pub mli_muskie_filename : String
}

/*
 * MantaRequestInfo records all information we've collected about the Manta
 * request.
 */
#[derive(Debug)]
pub struct MantaRequestInfo {
    mri_muskie : Option<MuskieLogEntry>
}

pub fn mri_dump(mri : &MantaRequestInfo)
{
    println!("{:?}", mri);
}

pub fn mri_parse_files(mli : &MantaLogParserInput)
    -> Result<MantaRequestInfo, String>
{
    let muskie_log = mri_parse_muskie_file(&mli.mli_muskie_filename)?;
    let muskie_entry = muskie_log.muskie_entries[0].clone();

    Ok(MantaRequestInfo {
        mri_muskie: Some(muskie_entry)
    })
}
