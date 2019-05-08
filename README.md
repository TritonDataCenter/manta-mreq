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
- Add calculated total latency for the Muskie timeline to the basic output
  - Consider adding the calculated latency-to-first-byte
- See what else we should add to the output (e.g., response status code!)
- Try with other types of requests:
  - directory fetch (what I'm currently testing with)
  - directory create
  - object fetch and create
    - will want to separate out latency-to-first-byte?
- Consider filtering timeline events with elapsed time `>` 0 and `<1ms`
- Lots of XXXs and TODOs
