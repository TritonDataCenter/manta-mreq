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
pub use log_muskie::mri_audit_entry;
pub use log_muskie::MuskieAuditInfo;

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
pub struct MantaRequestInfo {
    mri_muskie : MuskieAuditInfo,
    mri_timeline_overall : timeline::Timeline,
    mri_timeline_muskie : timeline::Timeline
}

pub fn mri_parse_files(mli : &MantaLogParserInput)
    -> Result<MantaRequestInfo, String>
{
    let muskie_log = mri_parse_muskie_file(&mli.mli_muskie_filename)?;
    let muskie_entry = muskie_log.muskie_entries[0].clone();
    let audit_entry = mri_audit_entry(&muskie_entry)?;
    let (overall_timeline, muskie_timeline) = mri_timelines(&audit_entry)?;

    Ok(MantaRequestInfo {
        mri_muskie: audit_entry,
        mri_timeline_overall: overall_timeline,
        mri_timeline_muskie: muskie_timeline
    })
}

pub fn mri_dump(mri : &MantaRequestInfo)
{
    let muskie_info = &mri.mri_muskie;
    let remote_ip = &muskie_info.mai_remote_address_logical;
    let dns_name = muskie_info.mai_req_headers["host"].as_string(); // XXX
    let min_duration_option = Some(chrono::Duration::milliseconds(1));

    // TODO add: whether client requested keep-alive and whether it got it
    println!("MANTA CLIENT:");
    println!("  remote IP:      {}", remote_ip);
    println!("  Manta DNS name: {}", dns_name);
    println!("    (inferred from client \"Host\" header)");
    println!("  agent: {}",
        muskie_info.mai_req_headers["user-agent"].as_string());
    println!("");

    // TODO Any information about the load balancer
    // TODO Any information about mako instances

    println!("WEBAPI SERVER:  ZONE {} PID {}", muskie_info.mai_hostname,
        muskie_info.mai_pid);
    // TODO add warning for missing x-server-name or x-server-name not matching
    println!("");

    // TODO handle cases of missing headers
    // TODO warn if server request id differs from client's?
    println!("REQUEST DETAILS:");
    println!("  request id:       {}",
        muskie_info.mai_response_headers["x-request-id"].as_string());
    println!("  method:           {}", muskie_info.mai_req_method);
    println!("  operation:        {}", muskie_info.mai_operation);
    println!("  billable op:      {}", muskie_info.mai_billable_operation);
    println!("  url:              {}", muskie_info.mai_req_url);
    println!("  caller account:   {} ({})", muskie_info.mai_req_caller_login,
        muskie_info.mai_req_caller_uuid);
    println!("  caller privilege: {}",
        if muskie_info.mai_req_caller_operator { "OPERATOR" }
        else { "unprivileged account" });
    println!("  owner account:    {}", muskie_info.mai_req_owner_uuid);
    println!("  route:            {}", muskie_info.mai_route);
    println!("");

    println!("RESPONSE DETAILS:");
    println!("  status code:     {}", muskie_info.mai_response_status_code);
    println!("  muskie latency:  {} ms (calculated from timers)",
        mri.mri_timeline_muskie.total_elapsed().num_milliseconds());
    println!("  x-response-time: {} ms (\"x-response-time\" header)",
        muskie_info.mai_response_headers["x-response-time"].as_i64());
    println!("    (This is the latency-to-first-byte reported by the \
        server.)");
    println!("");

    // TODO should probably include "headobject"?
    if muskie_info.mai_route == "putobject" ||
        muskie_info.mai_route == "getstorage" ||
        muskie_info.mai_route == "deletestorage" {
        mri_dump_object_metadata(&muskie_info);
    }

    if muskie_info.mai_route == "putobject" ||
        muskie_info.mai_route == "getstorage" {
        mri_dump_shark_info(&muskie_info);
    }

    match &muskie_info.mai_error {
        None => println!("ERROR INFORMATION: no error found in log entry"),
        Some(ref error) => {
            println!("ERROR INFORMATION:");
            println!("    name:    {}", error.mle_error_name);
            println!("    message: {}", error.mle_error_message);
            // XXX add verbose option to print stack too
        }
    }
    println!("");

    // TODO check transfer-encoding here and emit warning in weird case.  See
    // RFC 2616 4.3, though -- some methods don't allow bodies.  See 4.4 for how
    // to know the body length.
    println!("DATA TRANSFER:");
    println!("  request headers:           {} bytes",
        muskie_info.mai_req_header_length);
    println!("  request content length:    {}",
        match muskie_info.mai_req_headers.get("content-length") {
            // TODO handle cases of wrong header value type (e.g., string here)
            Some(header_value) => format!("{} bytes", header_value.as_i64()),
            None => String::from("unspecified\n    (presumably streamed using \
                chunked transfer encoding)")
        });
    println!("  response headers:          {} bytes",
        muskie_info.mai_response_header_length);
    println!("  response content length:   {}",
        match muskie_info.mai_response_headers.get("content-length") {
            // TODO handle cases of wrong header value type (e.g., string here)
            Some(header_value) => format!("{} bytes", header_value.as_i64()),
            None => String::from("unspecified\n    (presumably streamed using \
                chunked transfer encoding)")
        });
    println!("  object bytes transferred:  {}",
        match muskie_info.mai_bytes_transferred {
            None => String::from("unknown"),
            Some(b) => format!("{}", b)
        });
    println!("");

    println!("TIMELINE:\n    starts at {}\n",
        mri.mri_timeline_overall.events()[0].wall_start().format("%FT%T.%3fZ"));
    mri_dump_timeline(&mri.mri_timeline_overall, true,
        chrono::Duration::milliseconds(0), min_duration_option, 0);
}

fn mri_dump_timeline(timeline : &timeline::Timeline, dump_header : bool,
    base : chrono::Duration, min_duration_option : Option<chrono::Duration>,
    depth : u8)
    -> u16
{
    if dump_header {
        println!("{:13} {:>6} {:>6} {:>6} {}", "WALL TIME",
            "rSTART", "rCURR", "ELAPSD", "EVENT");
    }

    let mut nskipped = 0;

    for event in timeline.events() {
        if let Some(min_duration) = min_duration_option {
            //
            // TODO we're using 0 as a special value for important events that
            // have no elapsed time, but this should probably be an option
            // instead.
            //
            if !event.duration().is_zero() && event.duration() < min_duration {
                nskipped += 1;
                continue;
            }
        }

        // let wall_start = format!("{}", event.wall_start());
        let wall_start = event.wall_start().format("%T.%3fZ");
        print!("{:13} {:6} {:6} ", wall_start,
            (base + event.relative_start()).num_milliseconds(),
            event.relative_start().num_milliseconds());

        let maybe_subtimeline = event.subtimeline();
        if let Some(subtimeline) = maybe_subtimeline {
            println!("{:>6} {} {{", "-", event.label());
            nskipped += mri_dump_timeline(subtimeline, false,
                event.relative_start(), min_duration_option, depth + 1);

            let wall_end = event.wall_end().format("%T.%3fZ");
            println!("{:13} {:6} {:>6} {:6} {:width$}}} (subtimeline ended)",
                wall_end, (base + event.relative_start() +
                event.duration()).num_milliseconds(), "-",
                event.duration().num_milliseconds(), "",
                width = (depth * 4) as usize);
        } else {
            println!("{:6} {:width$}{}", event.duration().num_milliseconds(),
                "", event.label(), width = (depth * 4) as usize);
        }
    }

    if depth == 0 {
        if nskipped > 0 {
            println!("\nNOTE: {} timeline event{} with duration less than {} \
                ms {} not shown above.", nskipped,
                if nskipped == 1 { "" } else { "s" },
                min_duration_option.expect(
                    "must be min_duration_option if events were filtered").
                    num_milliseconds(),
                if nskipped == 1 { "was" } else { "were" });
        }

        println!("");
        println!("   rSTART   relative time (in milliseconds) since the first \
            event\n            in the whole timeline\n");
        println!("   rCURR    relative time (in milliseconds) since the first \
            event\n            in the current subtimeline\n");
        println!("   ELAPSD   elapsed time (in milliseconds) for this event");
    }

    return nskipped;
}

fn mri_timelines(muskie_info : &MuskieAuditInfo)
    -> Result<(timeline::Timeline, timeline::Timeline), String>
{
    /*
     * The Muskie audit log entry is the only anchor point we have for this
     * timeline.  Other events (namely, execution of Muskie request handlers)
     * mostly have durations associated with them, so we have to work backwards
     * from the completion time.
     */
    let walltime_end = muskie_info.mai_time;
    let mut muskie_timeline = timeline::TimelineBuilder::new_ending(
        walltime_end);
    muskie_timeline.prepend("muskie created audit log entry",
        &chrono::Duration::microseconds(0));

    let handler_durations = muskie_info.mai_timers.map();
    let mut handler_names : Vec<&String> = handler_durations.keys().collect();
    handler_names.reverse();
    for handler_name in handler_names {
        let duration_us = handler_durations[handler_name].as_i64().expect(
            "timer was not a 64-bit integer");
        muskie_timeline.prepend(&format!("{}", handler_name),
            &chrono::Duration::microseconds(duration_us));
    }

    muskie_timeline.prepend("muskie began processing request",
        &chrono::Duration::microseconds(0));
    let muskie_timeline = Box::new(muskie_timeline.finish());

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
    let client_time = muskie_info.mai_req_headers["date"].as_string();
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

    timeline.add_timeline("muskie handlers", muskie_timeline.clone());
    return Ok((timeline.finish(), *muskie_timeline));
}

fn mri_dump_object_metadata(mip : &MuskieAuditInfo)
{
    println!("MANTA OBJECT:");
    println!("  path:                     {}", mip.mai_req_url);
    println!("  objectid:                 {}",
        mip.mai_objectid.as_ref().unwrap_or(&String::from("unknown")));
    println!("  metadata on shard:        {}",
        mip.mai_shard_entry.as_ref().unwrap_or(&String::from("unknown")));
    // TODO explicitly note case of parent metadata being a synthetic directory
    // like "/account/stor"?
    println!("  parent metadata on shard: {}",
        mip.mai_shard_parent.as_ref().unwrap_or(&String::from("unknown")));

    println!("  durability level:         {}",
        mip.mai_response_headers.get("durability-level").map_or(
            String::from("unknown"), |x| format!("{}", x.as_i64())));
    println!("  md5sum (HTTP):            {}",
        mip.mai_response_headers.get("content-md5").map_or(
            &String::from("unknown"), |x| x.as_string()));

    println!("");
}

fn mri_dump_shark_info(mip : &MuskieAuditInfo)
{
    if let None = mip.mai_sharks_contacted {
        println!("SHARKS CONTACTED: not found in log entry");
        println!("");
    }

    let sharks = mip.mai_sharks_contacted.as_ref().unwrap();
    println!("SHARKS CONTACTED:");
    println!("  {:13} {:>6} {:>6} {:>4} {}", "START", "TTFB", "TOTAL", "OK?",
        "STOR_ID");
    for shark in sharks {
        println!("  {:13} {:>6} {:>6} {:>4} {}",
            shark.mai_shark_time_start.format("%T.%3fZ"),
            shark.mai_shark_latency_ttfb.num_milliseconds(),
            shark.mai_shark_latency_total.num_milliseconds(),
            if shark.mai_shark_success { "OK" } else { "FAIL" },
            shark.mai_shark_storid);
    }
    println!("\n");
}
