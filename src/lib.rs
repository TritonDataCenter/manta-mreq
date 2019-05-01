/*
 * src/lib.rs: entry point for library use
 */

extern crate chrono;
extern crate serde;
#[cfg_attr(test, macro_use)]
extern crate serde_json;
#[macro_use]
extern crate serde_derive;

mod log_common;
mod log_muskie;
mod timeline;

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
    let muskie_response = muskie_entry.mle_response.as_ref().unwrap();
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
    // TODO add warning for missing x-server-name or x-server-name not matching
    println!("");

    println!("REQUEST DETAILS:");
    println!("  request id:      {}",
        muskie_response.mle_response_headers["x-request-id"].string());
    println!("  method:          {}", muskie_request.mle_req_method);
    println!("  operation:       {}",
        muskie_entry.mle_operation.clone().unwrap_or(String::from("unknown")));
    println!("  billable op:     {}",
        muskie_entry.mle_billable_operation.clone().unwrap_or(
        String::from("unknown")));
    println!("  url:             {}", muskie_request.mle_req_url);
    println!("  owner account:   {}", muskie_request.mle_req_owner);
    println!("  route:           {}",
        muskie_entry.mle_route.clone().unwrap_or(String::from("unknown")));
    println!("  x-response-time: {} ms (\"x-response-time\" header)",
        muskie_response.mle_response_headers["x-response-time"].as_i64());
    println!("      (This is the latency-to-first-byte reported by the \
        server.)");

    // TODO shards (parent, entry)
    // TODO sharks contacted?
    // TODO data transfer (including headers)

    let timeline = mri_timeline(mri);
    println!("");
    mri_dump_timeline(&timeline, true, chrono::Duration::milliseconds(0));
}

fn mri_dump_timeline(timeline : &timeline::Timeline, dump_header : bool,
    base : chrono::Duration)
{
    if dump_header {
        println!("{:30} {:>6} {:>6} {:>6} {}", "WALL TIMESTAMP",
            "TIMEms", "RELms", "ELAPms", "EVENT");
    }

    for event in timeline.events() {
        /*
         * The formatter for the timestamps does not appear to implement width
         * specifiers, so in order to do that properly, we must first format it
         * as a string and then separately format that string with a width
         * specifier.
         */
        let wall_start = format!("{}", event.wall_start());
        println!("{:30} {:6} {:6} {:6} {}", wall_start,
            (base + event.relative_start()).num_milliseconds(),
            event.relative_start().num_milliseconds(),
            event.duration().num_milliseconds(), event.label());

        let maybe_subtimeline = event.subtimeline();
        if let Some(subtimeline) = maybe_subtimeline {
            mri_dump_timeline(subtimeline, false, event.relative_start());
        }
    }
}

fn mri_timeline(mri : &MantaRequestInfo)
    -> timeline::Timeline
{
    let muskie_entry = mri.mri_muskie.as_ref().unwrap();
    let muskie_request = muskie_entry.mle_request.as_ref().unwrap();

    /*
     * The Muskie audit log entry is the only anchor point we have for this
     * timeline.  Other events (namely, execution of Muskie request handlers)
     * mostly have durations associated with them, so we have to work backwards
     * from the completion time.
     */
    let walltime_end : chrono::DateTime<chrono::Utc> =
        muskie_entry.mle_time.parse().unwrap();
    let mut muskie_timeline = timeline::TimelineBuilder::new_ending(
        walltime_end);
    muskie_timeline.prepend("muskie created audit log entry",
        &chrono::Duration::microseconds(0));

    let handler_durations = muskie_request.mle_req_timers.map();
    let mut handler_names : Vec<&String> = handler_durations.keys().collect();
    handler_names.reverse();
    for handler_name in handler_names {
        let duration_us = handler_durations[handler_name].as_i64().expect(
            "timer was not a 64-bit integer");
        muskie_timeline.prepend(&format!("muskie handler: {}", handler_name),
            &chrono::Duration::microseconds(duration_us));
    }

    muskie_timeline.prepend("muskie began processing request",
        &chrono::Duration::microseconds(0));
    let muskie_timeline = muskie_timeline.finish(); // TODO style

    //
    // TODO This could be more flexible.
    // We commonly see HTTP "Date" header values that look like this:
    //
    //   Fri, 26 Apr 2019 21:18:02 GMT
    //
    // Based on RFC 2616 (section 3.3.1, "Full Date"), this appears to be "RFC
    // 822, updated by RFC 1123".  I have not yet found a way to parse this
    // directly in Rust.  Additionally, note that the time zone is given by name
    // ("GMT") rather than offset (e.g., "+00:00").  The chrono library we use
    // for date and time manipulation does not seem to have any way to identify
    // the time zone (e.g., offset) from the name.  As a result, short of
    // implementing this ourselves with our own time zone database, we assume
    // the common case of GMT and handle that directly.
    //
    let client_time = muskie_request.mle_req_headers["date"].string();
    let client_timestamp = {
        let timestamp_parsed : Result<chrono::DateTime<chrono::Utc>, _>;

        if client_time.ends_with(" GMT") {
            let prefixlen = client_time.len() - " GMT".len();
            let timestamp_prefix = &client_time[0..prefixlen];
            let timestamp_formatted = format!("{} +00:00", timestamp_prefix);
            timestamp_parsed = match chrono::DateTime::parse_from_str(
                &timestamp_formatted, "%a, %d %h %Y %T %:z") {
                Ok(ts) => Ok(ts.with_timezone(&chrono::Utc)),
                Err(e) => Err(e)
            }
        } else {
            timestamp_parsed = client_time.parse();
        }

        timestamp_parsed
    };

    let mut timeline = timeline::TimelineBuilder::new_ending(walltime_end);

    match client_timestamp {
        Ok(when) => {
            timeline.add("client generated Date header", &when,
                &chrono::Duration::microseconds(0), None);
        },
        Err(e) => {
            // XXX want some other way to track warnings
            eprintln!("client timestamp (\"{}\"): error: {}", client_time, e);
        }
    }

    timeline.add_timeline("muskie processing", Box::new(muskie_timeline));
    return timeline.finish();
}
