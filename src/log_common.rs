/*
 * src/log_common.rs: common functions for log parsing
 */

use std::fs::File;
use std::io::Read;

//
// XXX This should really have a byte limit that stops when we've read that many
// bytes.  Could use `take()`, but want it to emit an error rather than just
// EOF.
//
pub fn mri_read_file(filename : &String)
    -> Result<String, String>
{
    let mut buffer = String::new();
    let mut reader = match File::open(filename) {
        Ok(f) => f,
        Err(e) => return Err(format!("open \"{}\": {}", filename, e))
    };

    if let Err(e) = reader.read_to_string(&mut buffer) {
        return Err(format!("{}", e));
    }

    return Ok(buffer);
}
