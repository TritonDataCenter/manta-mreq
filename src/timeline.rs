/*
 * src/timeline.rs: basic timeline abstractions
 */

//!
//! # Timeline
//!
//! A `Timeline` represents a sequence of `TimelineEvent`s, each having a start
//! time, duration, and label.  Each event may itself be associated with another
//! timeline, indicating that all the events of the subtimeline happened at
//! logically this point on the main timeline.
//!
//! Timelines are used within this package to keep track of events that happen
//! while processing a request.
//!
//! Timelines are constructed using a `TimelineBuilder`.
//!
//! The current expectation is that events will not overlap in time on a
//! timeline (though it's possible for there to be gaps between events).
//!

///
/// Timelines represent a sequence of events at specific wall clock times.
/// Create these using a `TimelineBuilder`.  See module-level documentation for
/// details.
///
#[derive(Clone, Debug)]
pub struct Timeline {
    ///
    /// The list of events in this timeline, in wall-clock order.
    ///
    tl_events : Vec<TimelineEvent>,

    ///
    /// The end time of this timeline.  For our purposes, timelines are always
    /// anchored by an end time (which must be provided when constructing them),
    /// so this is meaningful even for a timeline with no actual events.
    ///
    tl_end : chrono::DateTime<chrono::Utc>,

    ///
    /// The wall-clock time for the start of this timeline.  For a timeline with
    /// no events, the start time will match the end time.
    ///
    tl_start : chrono::DateTime<chrono::Utc>
}

impl Timeline
{
    ///
    /// Returns the total wall-clock time elapsed between the start and end of
    /// this timeline.
    ///
    pub fn total_elapsed(&self)
        -> chrono::Duration
    {
        return self.tl_end - self.tl_start;
    }

    ///
    /// Returns a vector of events in the timeline.
    /// TODO Should this be an iterator instead?
    ///
    pub fn events(&self)
        -> &Vec<TimelineEvent>
    {
        return &self.tl_events;
    }
}

///
/// Represents one event in a timeline.
///
#[derive(Clone, Debug)]
pub struct TimelineEvent {
    te_wall_start : chrono::DateTime<chrono::Utc>,
    te_relative_start : chrono::Duration,
    te_duration : chrono::Duration,
    te_label : String,
    te_timeline : Option<Box<Timeline>>
}

impl TimelineEvent {
    /// Returns the wall clock time when this event started.
    pub fn wall_start(&self)
        -> chrono::DateTime<chrono::Utc>
    {
        return self.te_wall_start.clone();
    }

    ///
    /// Returns the duration of this event (i.e., the delta between the start
    /// and end times).
    ///
    pub fn duration(&self)
        -> chrono::Duration
    {
        return self.te_duration.clone();
    }

    ///
    /// Returns the wall clock time when this event ended.
    ///
    pub fn wall_end(&self)
        -> chrono::DateTime<chrono::Utc>
    {
        return self.te_wall_start + self.te_duration;
    }

    ///
    /// Returns the delta between when this timeline started and when this event
    /// started.
    ///
    pub fn relative_start(&self)
        -> chrono::Duration
    {
        return self.te_relative_start.clone(); // TODO
    }

    ///
    /// Returns a human-readable label for this event.
    ///
    pub fn label(&self)
        -> String
    {
        return self.te_label.clone();
    }

    ///
    /// For events that themselves summarize a number of events in a
    /// subtimeline, returns the subtimeline.  If this is a simple event with no
    /// associated subtimeline, returns `None`.
    ///
    pub fn subtimeline(&self)
        -> Option<&Box<Timeline>>
    {
        match &self.te_timeline {
            Some(t) => Some(&t),
            None => None
        }
    }
}

///
/// Consumers use an instance of `TimelineBuilder` to construct a timeline.  See
/// `TimelineBuilder::new_ending()` to construct a builder.
///
/// Some fields in the `Timeline` can only be calculated once all events in the
/// `Timeline` are known, so separating construction in this way allows us to
/// provide an appropriately-typed interface for the pre-construction and
/// constructed states.
///
#[derive(Debug)]
pub struct TimelineBuilder {
    /// The sequence of events in the timeline (still under construction)
    tlb_events : Vec<TimelineBuilderEvent>,
    /// The end time of the timeline
    tlb_end : chrono::DateTime<chrono::Utc>,
}

impl TimelineBuilder {
    ///
    /// This is the interface through which consumers begin constructing a
    /// timeline.  `end` is a wall-clock time for the end of the timeline.  This
    /// might seem odd for a general-purpose facility, but it works well for
    /// this program because we're typically building timelines from logs, and
    /// the log entry timestamp itself is a natural end point for the timeline.
    /// In the future, we could also have a `new_starting()` constructor or
    /// other constructors as make sense.
    ///
    pub fn new_ending(end: chrono::DateTime<chrono::Utc>)
        -> TimelineBuilder
    {
        return TimelineBuilder {
            tlb_events : Vec::new(),
            tlb_end : end.clone()
        }
    }

    ///
    /// Add an event to the timeline starting at wall-clock time `start` for
    /// duration `duration`.  If `subtimeline` is specified, this behaves like
    /// `add_timeline()`.
    ///
    pub fn add(&mut self, label : &str, start : &chrono::DateTime<chrono::Utc>,
        duration : &chrono::Duration, subtimeline : Option<Box<Timeline>>)
    {
        //
        // TODO This interface shouldn't allow a subtimeline having a start time
        // that differs from `start`.
        //
        self.tlb_events.insert(0, TimelineBuilderEvent {
            tbe_wall_start : start.clone(),
            tbe_duration : duration.clone(),
            tbe_label: String::from(label).clone(),
            tbe_timeline: subtimeline
        });

        // TODO doing it like this makes this O(N^2) to insert N events
        self.tlb_events.sort_by(|a, b|
            (&a.tbe_wall_start).partial_cmp(&b.tbe_wall_start).unwrap());
    }

    ///
    /// Add `timeline` as a subtimeline to the current timeline.  This creates
    /// an event on the current timeline that refers to the subtimeline.  This
    /// indicates that all the events on the subtimeline happened at the
    /// designated time on the current timeline as well.
    ///
    pub fn add_timeline(&mut self, label : &str, timeline : Box<Timeline>)
    {
        self.add(label, &timeline.tl_start.clone(), &timeline.total_elapsed(),
            Some(timeline));
    }

    ///
    /// Prepend an event of length `duration` to the current timeline.  This
    /// creates a new event ending at the current start of the timeline having
    /// duration `duration`.
    ///
    pub fn prepend(&mut self, label : &str, duration : &chrono::Duration)
    {
        let end_wall_time = if self.tlb_events.len() == 0 {
            self.tlb_end
        } else {
            self.tlb_events[0].tbe_wall_start
        };

        self.add(label, &(end_wall_time - *duration), duration, None);
    }

    ///
    /// Returns a fully-constructed `Timeline` object, consume the builder
    /// itself in the process.
    ///
    pub fn finish(mut self)
        -> Timeline
    {
        if self.tlb_events.len() == 0 {
            return Timeline {
                tl_events : Vec::new(),
                tl_end: self.tlb_end,
                tl_start: self.tlb_end
            }
        }

        let basetime = self.tlb_events[0].tbe_wall_start;
        return Timeline {
            tl_events: self.tlb_events.into_iter().map(
                |builder_event| TimelineEvent {
                    te_label: builder_event.tbe_label,
                    te_wall_start: builder_event.tbe_wall_start,
                    te_duration: builder_event.tbe_duration,
                    te_timeline: builder_event.tbe_timeline,
                    te_relative_start: builder_event.tbe_wall_start - basetime
                }).collect(),
            tl_start: basetime,
            tl_end: self.tlb_end
        }
    }
}

///
/// Represents an event on a timeline that's currently being built.  This is
/// similar to a `TimelineEvent` but does not yet have fields that can only be
/// calculated once the whole timeline is known (e.g., the relative start time).
///
#[derive(Debug)]
struct TimelineBuilderEvent {
    tbe_wall_start : chrono::DateTime<chrono::Utc>,
    tbe_duration : chrono::Duration,
    tbe_label : String,
    tbe_timeline : Option<Box<Timeline>>
}
