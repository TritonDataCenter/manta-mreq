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

    $ ./target/debug/mreq testdata/muskie-ok-object-get.log 
    MANTA CLIENT:
      remote IP:      172.20.5.18
      Manta DNS name: manta.staging.joyent.us
        (inferred from client "Host" header)
      agent: restify/1.4.1 (x64-darwin; v8/3.14.5.9; OpenSSL/1.0.1t) node/0.10.45
    
    WEBAPI SERVER:  ZONE 204ac483-7e7e-4083-9ea2-c9ea22f459fd PID 969236
    
    REQUEST DETAILS:
      request id:       ec5d32fe-5ff8-43ae-a152-45fd1005afff
      method:           GET
      operation:        getstorage
      billable op:      GET
      url:              /dap/stor/1gfile.gz
      caller account:   dap (bc8cd146-fecb-11e1-bd8a-bb6f54b49808)
      caller privilege: unprivileged account
      owner account:    bc8cd146-fecb-11e1-bd8a-bb6f54b49808
      route:            getstorage
    
    RESPONSE DETAILS:
      status code:     200
      muskie latency:  148474 ms (calculated from timers)
      x-response-time: 123 ms ("x-response-time" header)
        (This is the latency-to-first-byte reported by the server.)
    
    MANTA OBJECT:
      path:                     /dap/stor/1gfile.gz
      objectid:                 97c40f30-ee7e-c398-a5ae-e855c84a37c0
      metadata on shard:        tcp://3.moray.staging.joyent.us:2020
      parent metadata on shard: unknown
      durability level:         2
      md5sum (HTTP):            +D3HJFxY5l+YqaQQZ1MjOg==
    
    ERROR INFORMATION: no error found in log entry
    
    DATA TRANSFER:
      request headers:           503 bytes
      request content length:    unspecified
        (presumably streamed using chunked transfer encoding)
      response headers:          371 bytes
      response content length:   1074069384 bytes
      object bytes transferred:  1074069384
    
    TIMELINE:
        starts at 2019-05-09T21:34:23.000Z
    
    WALL TIME     rSTART  rCURR ELAPSD EVENT
    21:34:23.000Z      0      0      0 client generated Date header
    21:34:23.507Z    507    507      - muskie handlers {
    21:34:23.507Z    507      0      0     muskie began processing request
    21:34:23.507Z    507      0      3     loadCaller
    21:34:23.511Z    511      4      4     verifySignature
    21:34:23.516Z    516      9      2     loadOwner
    21:34:23.518Z    518     11    105     getMetadata
    21:34:23.625Z    625    118 148356     streamFromSharks
    21:36:51.982Z 148982 148474      0     muskie created audit log entry
    21:36:51.982Z 148982      - 148474 } (subtimeline ended)
    
    NOTE: 29 timeline events with duration less than 1 ms were not shown above.
    
       rSTART   relative time (in milliseconds) since the first event
                in the whole timeline
    
       rCURR    relative time (in milliseconds) since the first event
                in the current subtimeline
    
       ELAPSD   elapsed time (in milliseconds) for this event
    

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
- Add information for object-fetch requests:
  - With this, we should be able to show an interesting timeline when we have to
    PUT an upload to different sets of sharks
- Try with other types of requests:
  - directory fetch (what I'm currently testing with)
  - directory create
  - directory delete
  - object fetch
    - will want to separate out latency-to-first-byte?
    - add guessed bytes in and out?
  - object upload: fixed length
    - will want to separate out latency-to-first-byte?
    - add guessed bytes in and out?
  - object upload: streaming
    - will want to separate out latency-to-first-byte?
    - add guessed bytes in and out?
  - object delete
  - less common requests
    - responses: 404, 401/403, 408?
    - responses: 503, 500, 507
    - job-related requests?
    - GET /
    - unsupported methods?
- Consider adding the calculated latency-to-first-byte
- Add haproxy log entry
- Add nginx log entry
- Add node-manta log entry?
- Lots of XXXs and TODOs

TODO Muskie bugs to file:
- remotePort is not always present
- We should record what error name and message we sent to the client, which is
  not always the same as what we logged for internal purposes.
- Is it a bug that request headers are always logged as strings, while response
  headers may be logged as either?
