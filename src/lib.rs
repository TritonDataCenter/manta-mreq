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
    println!("{:30} {:>6} {:>6} {}", "WALL TIMESTAMP", "TIMEms", "ELAPms",
        "EVENT");
    for event in timeline.tl_events {
        /*
         * The formatter for the timestamps does not appear to implement width
         * specifiers, so in order to do that properly, we must first format it
         * as a string and then separately format that string with a width
         * specifier.
         */
        let wall_start = format!("{}", event.wall_start());
        println!("{:30} {:6} {:6} {}", wall_start,
            event.relative_start().num_milliseconds(),
            event.duration().num_milliseconds(), event.label());
    }
}

fn mri_timeline(mri : &MantaRequestInfo)
    -> Timeline
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
    let mut timeline = Timeline::new_ending(walltime_end);
    timeline.prepend("muskie created audit log entry",
        &chrono::Duration::microseconds(0));

    let handler_durations = muskie_request.mle_req_timers.map();
    let mut handler_names : Vec<&String> = handler_durations.keys().collect();
    handler_names.reverse();
    for handler_name in handler_names {
        let duration_us = handler_durations[handler_name].as_i64().expect(
            "timer was not a 64-bit integer");
        timeline.prepend(&format!("muskie handler: {}", handler_name),
            &chrono::Duration::microseconds(duration_us));
    }

    timeline.prepend("muskie began processing request",
        &chrono::Duration::microseconds(0));

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

    match client_timestamp {
        Ok(when) => {
            // TODO The way we model this kind of sucks.  We want the timeline
            // to include this entry, but it sucks that the relative timestamps
            // are now counted from this point, instead of when Muskie started
            // processing the request.  For one, it makes it look like the
            // request took a long time to get to Muskie, when it could just be
            // the lack of precision in the client-generated header.
            //
            // Idea: maybe timelines can have other timelines as "events" inside
            // them?  Relative timestamps are always shown relative to the most
            // specific timeline?  Then we'd have a separate timeline for Muskie
            // request handling (which would also make it easier to calculate
            // total latency of all request handlers).
            timeline.add("client generated Date header", &when,
                &chrono::Duration::microseconds(0));
        },
        Err(e) => {
            // XXX want some other way to track warnings
            eprintln!("client timestamp (\"{}\"): error: {}", client_time, e);
        }
    }
        
    timeline.finalize();
    return timeline;
}

#[derive(Debug)]
struct Timeline {
    pub tl_events : Vec<TimelineEvent>,
    tl_end : chrono::DateTime<chrono::Utc>,
    tl_start : Option<chrono::DateTime<chrono::Utc>>
}

impl Timeline {
    pub fn new_ending(end: chrono::DateTime<chrono::Utc>)
        -> Timeline
    {
        return Timeline {
            tl_events : Vec::new(),
            tl_end : end.clone(),
            tl_start: None
        }
    }

    pub fn add(&mut self, label : &str, start : &chrono::DateTime<chrono::Utc>,
        duration : &chrono::Duration)
    {
        // TODO doing it like this makes this O(N^2) to insert N events
        self.tl_events.insert(0, TimelineEvent {
            te_wall_start : start.clone(),
            te_relative_start : None,
            te_duration : duration.clone(),
            te_label: String::from(label).clone()
        });

        self.tl_events.sort_by(|a, b|
            (&a.te_wall_start).partial_cmp(&b.te_wall_start).unwrap());
    }

    pub fn prepend(&mut self, label : &str, duration : &chrono::Duration)
    {
        let end_wall_time = if self.tl_events.len() == 0 {
            self.tl_end
        } else {
            self.tl_events[0].wall_start()
        };

        self.add(label, &(end_wall_time - *duration), duration);
    }

    pub fn finalize(&mut self)
    {
        assert_eq!(None, self.tl_start);
        if self.tl_events.len() == 0 {
            self.tl_start = Some(self.tl_end);
            return;
        }

        let basetime = self.tl_events[0].wall_start();
        for event in &mut self.tl_events {
            assert_eq!(event.te_relative_start, None);
            event.te_relative_start = Some(event.wall_start() - basetime);
        }

        self.tl_start = Some(basetime);
    }

    pub fn total_elapsed(&self)
        -> chrono::Duration
    {
        return self.tl_end - self.tl_start.expect("timeline not finalized");
    }
}

//
// TODO: want relative timestamp since request started
//
#[derive(Debug)]
struct TimelineEvent {
    te_wall_start : chrono::DateTime<chrono::Utc>,
    te_relative_start : Option<chrono::Duration>,
    te_duration : chrono::Duration,
    te_label : String
}

impl TimelineEvent {
    pub fn wall_start(&self)
        -> chrono::DateTime<chrono::Utc>
    {
        return self.te_wall_start.clone();
    }

    pub fn relative_start(&self)
        -> chrono::Duration
    {
        // XXX
        return self.te_relative_start.unwrap().clone();
    }

    pub fn wall_end(&self)
        -> chrono::DateTime<chrono::Utc>
    {
        return self.te_wall_start + self.te_duration;
    }

    pub fn duration(&self)
        -> chrono::Duration
    {
        return self.te_duration.clone();
    }

    pub fn label(&self)
        -> String
    {
        return self.te_label.clone();
    }
}
