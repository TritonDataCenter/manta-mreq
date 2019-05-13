/*
 * src/log_muskie.rs: Muskie log format parser
 *
 * TODO review log entry details (e.g., error, caller.groups, caller.user, etc.)
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

    #[serde(rename = "err")]
    pub mle_error : Option<MuskieErrorValue>,

    #[serde(rename = "objectId")]       pub mle_objectid : Option<String>,
    #[serde(rename = "entryShard")]     pub mle_shard_entry : Option<String>,
    #[serde(rename = "parentShard")]    pub mle_shard_parent : Option<String>,

    #[serde(rename = "sharksContacted")]
    pub mle_sharks_contacted : Option<Vec<MuskieLogSharkContacted>>,

    #[serde(rename = "bytesTransferred")]
    pub mle_bytes_transferred: Option<MuskieLogEntryMaybeNumeric>,
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
    #[serde(rename = "login")]  pub mle_req_caller_login : String,
    #[serde(rename = "uuid")]   pub mle_req_caller_uuid : String,
    #[serde(rename = "groups")] pub mle_req_caller_groups : Vec<String>
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
    pub fn as_string(&self) -> &String {
        match self {
            MuskieLogEntryHeaderValue::Str(s) => s,
            _ => panic!("header value is not a string")
        }
    }

    // XXX We should validate and handle this better.
    pub fn as_i64(&self) -> i64 {
        match self {
            MuskieLogEntryHeaderValue::Int(i64val) => *i64val,
            // XXX This is somewhat dubious, but the problem is that Muskie logs
            // all request headers as a string (probably since they initially
            // came in as strings from the client) while it logs response
            // headers that were originally numeric as numbers.  We have to deal
            // with this.  We could do it earlier when parsing, but it's simpler
            // for now to parse here if needed.
            // This technically violates Rust's naming convention, where "as_"
            // is supposed to be a "free" operation.  However, this best
            // reflects the caller's view of this operation (namely, that we're
            // just returning a particular representation of the header).  This
            // would be free if we'd parsed it earlier.
            MuskieLogEntryHeaderValue::Str(strval) => {
                match strval.parse() {
                    Ok(x) => x,
                    Err(e) => panic!("header value is not a number \
                        (parse error: {})", e)
                }
            }
        }
    }
}

impl std::fmt::Display for MuskieLogEntryHeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            MuskieLogEntryHeaderValue::Str(s) => write!(f, "{}", s),
            MuskieLogEntryHeaderValue::Int(n) => write!(f, "{}", n),
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

impl MuskieLogEntryTimers {
    pub fn map(&self) -> &Map<String, serde_json::Value> {
        match self {
            MuskieLogEntryTimers::Timers(map) => map
        }
    }
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
    pub mle_response_status_code : u16, // TODO parse this as enum
    #[serde(rename = "headers")]
    pub mle_response_headers : BTreeMap<String, MuskieLogEntryHeaderValue>
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MuskieErrorValue {
    Error(MuskieErrorObject),
    NoError(bool)
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieErrorObject {
    #[serde(rename = "stack")]      pub mle_error_stack : String,
    #[serde(rename = "name")]       pub mle_error_name : String,
    #[serde(rename = "message")]    pub mle_error_message : String
}

/*
 * Similar to headers, some other values in the log entry are expected to be
 * numbers, but may have been recorded as strings.  For example,
 * "bytesTransferred" appears to be a number on PUTs and a string on GETs.
 */
#[derive(Debug, Clone, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum MuskieLogEntryMaybeNumeric {
    Str(String),
    Int(i64)
}
impl MuskieLogEntryMaybeNumeric {
    // XXX We should validate and handle this better.
    pub fn as_i64(&self) -> i64 {
        match self {
            MuskieLogEntryMaybeNumeric::Int(i64val) => *i64val,
            // XXX This is somewhat dubious, but the problem is that Muskie logs
            // all request headers as a string (probably since they initially
            // came in as strings from the client) while it logs response
            // headers that were originally numeric as numbers.  We have to deal
            // with this.  We could do it earlier when parsing, but it's simpler
            // for now to parse here if needed.
            // This technically violates Rust's naming convention, where "as_"
            // is supposed to be a "free" operation.  However, this best
            // reflects the caller's view of this operation (namely, that we're
            // just returning a particular representation of the header).  This
            // would be free if we'd parsed it earlier.
            MuskieLogEntryMaybeNumeric::Str(strval) => {
                match strval.parse() {
                    Ok(x) => x,
                    Err(e) => panic!("numeric value is not a number \
                        (parse error: {})", e)
                }
            }
        }
    }
}

#[derive(Debug, Clone, Deserialize, PartialEq)]
pub struct MuskieLogSharkContacted {
    #[serde(rename = "shark")]           pub mle_shark_storid : String,
    #[serde(rename = "result")]          pub mle_shark_result : String,
    #[serde(rename = "timeToFirstByte")] pub mle_shark_latency_ttfb : u64,
    #[serde(rename = "timeTotal")]       pub mle_shark_latency_total : u64,
    #[serde(rename = "_startTime")]      pub mle_shark_time_start : u64
}

///
/// A MuskieAuditInfo object collects the valid parts of a MuskieLogEntry that
/// represent a completed request.
///
pub struct MuskieAuditInfo {
    // Bunyan fields
    pub mai_hostname : String,
    pub mai_pid : String,
    pub mai_time : chrono::DateTime<chrono::Utc>,

    // Muskie-specific fields
    pub mai_operation : String,
    pub mai_route : String,

    pub mai_remote_address_logical : String,    // TODO can this be missing?
    pub mai_billable_operation : String,        // TODO can this be missing?
    pub mai_timers : MuskieLogEntryTimers,

    pub mai_req_header_length : u16,
    pub mai_req_method : String,                // TODO should be enum?
    pub mai_req_url : String,                   // TODO should be enum?
    pub mai_req_http_version : String,
    pub mai_req_owner_uuid : String,
    pub mai_req_headers : BTreeMap<String, MuskieLogEntryHeaderValue>,
    pub mai_req_caller_operator : bool,
    pub mai_req_caller_uuid : String,           // TODO what does this look like
    pub mai_req_caller_login : String,          // when it's missing?

    pub mai_response_header_length : u16,
    pub mai_response_status_code : u16,               // TODO parse as enum
    pub mai_response_headers : BTreeMap<String, MuskieLogEntryHeaderValue>,

    pub mai_error : Option<MuskieErrorObject>,

    // For object-related requests
    pub mai_objectid : Option<String>,
    pub mai_shard_entry : Option<String>,
    pub mai_shard_parent : Option<String>,
    pub mai_bytes_transferred : Option<i64>,
    pub mai_sharks_contacted : Option<Vec<MuskieAuditSharkContacted>>
}

pub struct MuskieAuditSharkContacted {
    pub mai_shark_storid : String,
    pub mai_shark_success : bool,
    pub mai_shark_latency_ttfb : chrono::Duration,
    pub mai_shark_latency_total : chrono::Duration,
    pub mai_shark_time_start : chrono::DateTime<chrono::Utc>
}

///
/// Given a Muskie log entry, validates the entry.  If the entry represents a
/// well-formed audit log entry, then returns a MuskieAuditInfo object
/// describing the request and response.  Otherwises, returns an error.
///
pub fn mri_audit_entry(mle : &MuskieLogEntry)
    -> Result<MuskieAuditInfo, String>
{
    if mle.mle_bunyan_version != 0 {
        return Err(format!("expected bunyan version 0, but found {}",
            mle.mle_bunyan_version));
    }

    if mle.mle_audit.is_none() || !mle.mle_audit.unwrap() {
        return Err(format!("expected audit log entry (having \"audit\": true)"));
    }

    let wall_time : chrono::DateTime<chrono::Utc> = match mle.mle_time.parse() {
        Ok(t) => t,
        Err(e) => return Err(format!("{}", e))
    };
    let operation : &String = mle.mle_operation.as_ref().ok_or(
        String::from("expected \"operation\" field)"))?;
    let route : &String = mle.mle_route.as_ref().ok_or(
        String::from("expected \"route\" field)"))?;
    let remote_address_logical : &String =
        mle.mle_remote_address_logical.as_ref().ok_or(
        String::from("expected \"logicalRemoteAddress\" field)"))?;
    let billable_operation : &String =
        mle.mle_billable_operation.as_ref().ok_or(
        String::from("expected \"billable_operation\" field)"))?;
    let request : &MuskieLogEntryRequest =
        mle.mle_request.as_ref().ok_or(
        String::from("expected \"req\" field)"))?;
    let response : &MuskieLogEntryResponse =
        mle.mle_response.as_ref().ok_or(
        String::from("expected \"res\" field)"))?;
    let caller : &MuskieLogEntryCaller =
        request.mle_req_caller.as_ref().ok_or(
        String::from("expected \"req.caller\" field)"))?;

    let error = match &mle.mle_error {
        None => None,
        Some(error_value) => match error_value {
            MuskieErrorValue::Error(error_object) => Some(error_object.clone()),
            MuskieErrorValue::NoError(false) => None,
            MuskieErrorValue::NoError(true) => {
                return Err(format!("unexpected value for error: \"true\""))
            }
        }
    };

    let sharks = mri_audit_sharks(&mle)?;

    return Ok(MuskieAuditInfo {
        mai_hostname : mle.mle_hostname.clone(),
        mai_pid : mle.mle_pid.to_string(),
        mai_time : wall_time,
        mai_operation : operation.clone(),
        mai_route : route.clone(),
        mai_remote_address_logical : remote_address_logical.clone(),
        mai_billable_operation : billable_operation.clone(),
        mai_timers : request.mle_req_timers.clone(),
        mai_req_header_length : *mle.mle_request_header_length.as_ref().ok_or(
            String::from("expected \"reqHeaderLength\" field)"))?,
        mai_req_method : request.mle_req_method.clone(),
        mai_req_url : request.mle_req_url.clone(),
        mai_req_http_version : request.mle_req_http_version.clone(),
        mai_req_owner_uuid : request.mle_req_owner.clone(),
        mai_req_headers : request.mle_req_headers.clone(),
        mai_response_status_code : response.mle_response_status_code,
        mai_response_header_length : *mle.mle_response_header_length.as_ref().
            ok_or(String::from("expected \"reqHeaderLength\" field)"))?,
        mai_response_headers : response.mle_response_headers.clone(),
        mai_req_caller_operator : caller.mle_req_caller_groups.contains(
            &String::from("operators")),
        mai_req_caller_uuid : caller.mle_req_caller_uuid.clone(),
        mai_req_caller_login : caller.mle_req_caller_login.clone(),
        mai_error : error,
        mai_objectid : mle.mle_objectid.clone(),
        mai_shard_entry : mle.mle_shard_entry.clone(),
        mai_shard_parent : mle.mle_shard_parent.clone(),
        mai_bytes_transferred : mle.mle_bytes_transferred.as_ref().map(
            |x| x.as_i64()),
        mai_sharks_contacted : sharks
    });
}

fn mri_audit_sharks(mle : &MuskieLogEntry)
    -> Result<Option<Vec<MuskieAuditSharkContacted>>, String>
{
    if let Some(ref rawsharks) = mle.mle_sharks_contacted {
        let mut sharks : Vec<MuskieAuditSharkContacted> = Vec::new();

        for rawshark in rawsharks {
            if rawshark.mle_shark_result != "ok" &&
                rawshark.mle_shark_result != "fail" {
                    return Err(format!("log entry shark contacted (\"{}\"): \
                        expected \"result\" to be \"ok\" or \"fail\",
                        but found \"{}\"", rawshark.mle_shark_storid,
                        rawshark.mle_shark_result));
                }

            let start_time = chrono::NaiveDateTime::from_timestamp_opt(
                (rawshark.mle_shark_time_start / 1000) as i64,
                (1000000 * (rawshark.mle_shark_time_start % 1000)) as u32);
            if start_time == None {
                return Err(format!("log entry shark contacted (\"{}\"): \
                    unsupported millisecond timestamp: \"{}\"",
                    rawshark.mle_shark_storid,
                    rawshark.mle_shark_time_start));
            }

            sharks.push(MuskieAuditSharkContacted {
                mai_shark_storid : rawshark.mle_shark_storid.clone(),
                mai_shark_success : rawshark.mle_shark_result == "ok",
                mai_shark_latency_ttfb : chrono::Duration::milliseconds(
                    rawshark.mle_shark_latency_ttfb as i64),
                mai_shark_latency_total : chrono::Duration::milliseconds(
                    rawshark.mle_shark_latency_total as i64),
                mai_shark_time_start : chrono::DateTime::from_utc(
                    start_time.unwrap(), chrono::Utc)
            });
        }

        return Ok(Some(sharks));
    } else {
        return Ok(None);
    }
}
