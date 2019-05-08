# manta-mreq

This repository is part of the Joyent Manta project.  For contribution
guidelines, issues, and general documentation, visit the main
[Manta](http://github.com/joyent/manta) project page.

mreq is a small tool for summarizing the timeline of a Manta HTTP request.
[See MANTA-4232 for details.](http://smartos.org/bugview/MANTA-4232)


**This program is a very rough, very early prototype.**  It's not useful for
anything at all yet.


## Getting started

This is a Rust program.  Build with cargo:

    $ cargo build

Then run it:

    $ ./target/debug/mreq /path/to/muskie.log

where `/path/to/muskie.log` is a log file containing *one* Muskie audit log
entry.

Here's an example:

    $ ./target/debug/mreq ./testdata/muskie.log 
    MANTA CLIENT:
      remote IP:      172.20.5.18
      account:        dap (bc8cd146-fecb-11e1-bd8a-bb6f54b49808)
      Manta DNS name: manta.staging.joyent.us
      (inferred from client "Host" header)
      agent: restify/1.4.1 (x64-darwin; v8/3.14.5.9; OpenSSL/1.0.1t) node/0.10.45
    
    WEBAPI SERVER:  ZONE 6e59a763-6f6a-46a1-926e-90c1b7fc370b PID 783603
    
    REQUEST DETAILS:
      request id:      36a2e294-2f5d-4859-8793-bee652ec0fff
      method:          GET
      operation:       getpublicstorage
      billable op:     LIST
      url:             /dap/public?limit=1024
      owner account:   bc8cd146-fecb-11e1-bd8a-bb6f54b49808
      route:           getpublicstorage
    
    RESPONSE DETAILS:
      status code:     200
      muskie latency:  256 ms (calculated from timers)
      x-response-time: 153 ms ("x-response-time" header)
          (This is the latency-to-first-byte reported by the server.)
    
    WALL TIMESTAMP                 TIMEms  RELms ELAPms EVENT
    2019-04-26 21:18:01 UTC             0      0      0 client generated Date header
    2019-04-26 21:18:01.855288 UTC    855    855      - muskie processing {
    2019-04-26 21:18:01.855288 UTC    855      0      0 muskie began processing request
    2019-04-26 21:18:01.856151 UTC    856      0      3 muskie handler: loadCaller
    2019-04-26 21:18:01.859281 UTC    859      3      3 muskie handler: verifySignature
    2019-04-26 21:18:01.863041 UTC    863      7      3 muskie handler: loadOwner
    2019-04-26 21:18:01.866892 UTC    866     11     10 muskie handler: getMetadata
    2019-04-26 21:18:01.878245 UTC    878     22    107 muskie handler: getDirectoryCount
    2019-04-26 21:18:01.985449 UTC    985    130    126 muskie handler: getDirectory
    2019-04-26 21:18:02.112 UTC      1112    256      0 muskie created audit log entry
    2019-04-26 21:18:02.112 UTC      1112      -    256 } (subtimeline ended)
    
    NOTE: 21 timeline events with duration less than 1 ms were not shown above.


## Goals

When finished, `mreq` should take as input any combination of:

- a Muskie log entry
- an haproxy log entry (from the load balancer)
- any number of Mako access log entries (from storage nodes)
- a set of node-manta log entries (from the client)

and produce as complete a timeline as possible from the information provided.

Nice-to-haves:

- The initial goal is for this to work when provided files containing just one
  log entry each.  It would be neat if you could provide entire log files and
  specify a filter (e.g., a request id) on the command-line.  That would
  simplify the user's life so you'd merely need to collect the relevant logs and
  the tool would take care of filtering and matching up entries between the
  files.
- It would also be neat if you could provide the log file data in any number of
  files passed on the command-line (e.g., `mreq haproxy.log muskie.log
  mako-1.log mako-2.log`) and the command would figure out what each one was and
  incorporate the information, rather than requiring you to specify them in a
  particular order or having to specify what each one was.


## Current status

Currently, this can dump basic information about a Muskie request, but the
output is still very much evolving.

Next steps:
- See what else we should add to the output
  - what other fields are already there?
  - what about adding a summary of bytes transferred in each direction?
- Try with other types of requests:
  - directory fetch (what I'm currently testing with)
  - directory create
  - object fetch and create
    - will want to separate out latency-to-first-byte?
- Consider adding the calculated latency-to-first-byte
- Lots of XXXs and TODOs
