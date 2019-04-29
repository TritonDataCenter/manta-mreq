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

pub fn mri_parse_files(mli : &MantaLogParserInput)
    -> Result<MantaRequestInfo, String>
{
    let muskie_log = mri_parse_muskie_file(&mli.mli_muskie_filename)?;
    let muskie_entry = muskie_log.muskie_entries[0].clone();

    Ok(MantaRequestInfo {
        mri_muskie: Some(muskie_entry)
    })
}

pub fn mri_dump(mri : &MantaRequestInfo)
{
    if let None = mri.mri_muskie {
        if let None = mri.mri_muskie.as_ref().unwrap().mle_request {
            println!("missing Muskie log entry or required fields");
        }
    }

    let muskie_entry = mri.mri_muskie.as_ref().unwrap();
    let muskie_request = muskie_entry.mle_request.as_ref().unwrap();
    let remote_ip = muskie_entry.mle_remote_address_logical.clone().unwrap_or(
        String::from("unknown remote IP address"));
    let dns_name = muskie_request.mle_req_headers["host"].string();

    println!("MANTA CLIENT:");
    println!("  remote IP:      {}", remote_ip);

    if let Some(ref caller) = muskie_request.mle_req_caller {
        println!("  account:        {} ({})", caller.mle_req_caller_login,
            caller.mle_req_caller_uuid);
    } else {
        println!("  account: none provided (anonymous request)");
    }

    println!("  Manta DNS name: {}", dns_name);
    println!("  (inferred from client \"Host\" header)");
    println!("  agent: {}",
        muskie_request.mle_req_headers["user-agent"].string());
    println!("");

    // TODO Any information about the load balancer
    // TODO Any information about mako instances

    println!("WEBAPI SERVER:  ZONE {} PID {}", muskie_entry.mle_hostname,
        muskie_entry.mle_pid);
    println!("");

    println!("REQUEST DETAILS:");
    println!("  method:        {}", muskie_request.mle_req_method);
    println!("  operation:     {}",
        muskie_entry.mle_operation.clone().unwrap_or(String::from("unknown")));
    println!("  billable op:   {}",
        muskie_entry.mle_billable_operation.clone().unwrap_or(
        String::from("unknown")));
    println!("  url:           {}", muskie_request.mle_req_url);
    println!("  owner account: {}", muskie_request.mle_req_owner);
    println!("  route:         {}",
        muskie_entry.mle_route.clone().unwrap_or(String::from("unknown")));
    // TODO shards (parent, entry)
    // TODO sharks contacted?
    // TODO data transfer (including headers)

    // XXX timeline
}
