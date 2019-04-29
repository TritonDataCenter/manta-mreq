/*
 * src/log_muskie.rs: Muskie log format parser
 *
 * TODO review log entry details (e.g., error, caller.groups, caller.user, etc.)
 * TODO Validate the semantics of the log entry and switch to a type that
 * doesn't have so many Options
 */

use std::collections::BTreeMap;
use std::fmt;

use serde_json::Map;

use super::mri_read_file;

/*
 * Given a file containing a single Muskie log entry, return a MuskieLog
 * describing the file's contents.
 */
pub fn mri_parse_muskie_file(filename : &String)
    -> Result<MuskieLog, String>
{
    let buffer = mri_read_file(filename)?;
    let parsed : Result<MuskieLogEntry, serde_json::Error> =
        serde_json::from_str(&buffer);
    if let Err(e) = parsed {
        return Err(format!("parse \"{}\": {}", &filename, e));
    }

    Ok(MuskieLog {
        muskie_filename: filename.clone(),
        muskie_entries: vec!(parsed.unwrap())
    })
}

/*
 * A MuskieLog just identifies the filename it came from and a sequence of
 * MuskieLogEntry objects.
 */
pub struct MuskieLog {
    pub muskie_filename : String,
    pub muskie_entries : Vec<MuskieLogEntry>
}

/*
 * MuskieLogEntry and the related structs below are used to represent a
 * bunyan-formatted Muskie audit log entry.
 */
#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieLogEntry {
    // Bunyan fields
    #[serde(rename = "hostname")]   pub mle_hostname : String,
    #[serde(rename = "pid")]        pub mle_pid : u64, // TODO should be string
    #[serde(rename = "level")]      pub mle_level : i16,
    #[serde(rename = "time")]       pub mle_time : String, // TODO parse this
    #[serde(rename = "v")]          pub mle_bunyan_version : u16,
    #[serde(rename = "msg")]        pub mle_message : String,

    // Muskie-specific fields (for audit entries only)
    #[serde(rename = "audit")]          pub mle_audit : Option<bool>,
    #[serde(rename = "operation")]      pub mle_operation : Option<String>,
    #[serde(rename = "latency")]        pub mle_latency : Option<u32>,
    #[serde(rename = "route")]          pub mle_route : Option<String>,
    #[serde(rename = "remotePort")]     pub mle_remote_port : Option<u16>,
    #[serde(rename = "remoteAddress")]  pub mle_remote_address : Option<String>,

    #[serde(rename = "logicalRemoteAddress")]
    pub mle_remote_address_logical : Option<String>,
    #[serde(rename = "billable_operation")]
    pub mle_billable_operation : Option<String>,

    #[serde(rename = "reqHeaderLength")]
    pub mle_request_header_length : Option<u16>,
    #[serde(rename = "req")]
    pub mle_request : Option<MuskieLogEntryRequest>,

    #[serde(rename = "resHeaderLength")]
    pub mle_response_header_length : Option<u16>,
    #[serde(rename = "res")]
    pub mle_response : Option<MuskieLogEntryResponse>,
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieLogEntryRequest {
    #[serde(rename = "method")]         pub mle_req_method : String,
    #[serde(rename = "url")]            pub mle_req_url : String,
    #[serde(rename = "httpVersion")]    pub mle_req_http_version : String,
    #[serde(rename = "owner")]          pub mle_req_owner : String,

    #[serde(rename = "headers")]
    pub mle_req_headers : BTreeMap<String, MuskieLogEntryHeaderValue>,
    #[serde(rename = "caller")]
    pub mle_req_caller : Option<MuskieLogEntryCaller>,
    #[serde(rename = "timers")]
    pub mle_req_timers : MuskieLogEntryTimers
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieLogEntryCaller {
    #[serde(rename = "login")] pub mle_req_caller_login : String,
    #[serde(rename = "uuid")]  pub mle_req_caller_uuid : String
}

/*
 * Regrettably, header values may be reported as either strings or integers.  We
 * use an untagged enum representation and let serde do the work.
 */
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MuskieLogEntryHeaderValue {
    Str(String),
    Int(i64)
}

impl MuskieLogEntryHeaderValue {
    // XXX We should validate and handle this better.
    pub fn string(&self) -> &String {
        match self {
            MuskieLogEntryHeaderValue::Str(s) => s,
            _ => panic!("header value is not a string")
        }
    }
}

/*
 * This type only exists for us to be able to implement our own Debug trait.
 * It would be nice to at least enforce that the values are `i64`, but that
 * would require additionally implementing Clone, Deserialize, and PartialEq by
 * hand.
 */
#[derive(Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MuskieLogEntryTimers {
    Timers(Map<String, serde_json::Value>)
}

impl fmt::Debug for MuskieLogEntryTimers {
    fn fmt(&self, formatter: &mut fmt::Formatter) -> Result<(), fmt::Error> {
        let map = {
            match self {
                MuskieLogEntryTimers::Timers(map) => map
            }
        };

        for (key, value) in map.iter() {
            formatter.write_fmt(format_args!("    {:?} => {:?}\n", key, value))?;
        }

        Ok(())
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieLogEntryResponse {
    #[serde(rename = "statusCode")]
    pub mle_response_status_code : u8, // TODO parse this as enum
    #[serde(rename = "headers")]
    pub mle_response_headers : BTreeMap<String, MuskieLogEntryHeaderValue>
}
