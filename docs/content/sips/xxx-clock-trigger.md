title = "SIP xxx - Clock Trigger"
template = "main"
date = "2022-05-23T16:46:52Z"
---

Summary: Support a clock trigger for Spin applications.

Owner: brian.hardock@fermyon.com

Created: May 23, 2022

Updated: May 23, 2022

## Background

This proposal aims to introduce a new `Clock` trigger type whereby components can define their own schedule to be automatically executed at specific times.
As an example, an application developer might use this trigger type to implement a component that sends out a daily report email or to update cached data every 30 seconds.

## Proposal

Components using the clock trigger type must implement the following WebAssembly interface:

```fsharp
// wit/ephemeral/spin-clock.wit

// Context for executing a clock triggered component
record clock-event {
    // Time at which this event fired represented as a unix timestamp
    timestamp: u64
    // Indicates if executing on start
    on-start: bool
}

// The entrypoint for a clock trigger component 
handle-clock-event: function(event: clock-event) -> expected<_, string>
```

To configure an application using clock triggered components, the application manifest (i.e. `spin.toml`)
must contain one of the following options to define the trigger's schedule. 

Using the [Crontab](https://crontab.guru/) syntax:

```toml
[[trigger.clock]]
# Execute every 5 minutes
crontab = "*/5 * * * *"
# Execute the component immediately on start
on_start = true
# The timezone (default "UTC") to use
timezone = "UTC"
# Component to execute
component = "send_report"

[[component]]
id = "send_report"
# ...
```

Or using a simple interval:

```toml
[[trigger.clock]]
# Execute every 30 seconds
interval = "30s" 
# Component to execute
component = "update_cache"

[[component]]
id = "update_cache"
# ...
```

NOTE: The interval above is an "end-time" interval meaning the component is executed exactly `30s` after the previous execution finishes.

In the future, this proposal could be extended to support a more sophisticated human-readable scheduling syntax, for example `every 10 minutes from 12:00 to 16:00`

### The Clock Rust SDK
```rust
use anyhow::Result;
use spin_sdk::clock_component;

#[clock_component]
fn clock_event_handler(event: ClockEvent) -> Result<()> {
    todo!("Handle event ...")
}
```

## Assumptions
* The clock trigger does not make strong guarantees about running triggers on time and not duplicating behavior on a schedule, i.e.
  if 2 instances are sending emails to customers, there is no guarantee that duplicate emails wont be sent.
* We do not consider clock changes on the host and the effect this has on trigger execution (Not feasible without some external persistence).

## Future Considerations

The following are future considerations not addressed by this proposal and are considered out-of-scope:

* How to handle restarts? Without persistence, it is impossible to adhere to a time-based schedule.
* How to handle retries on job failure? 
* How to address DST?
* https://www.explainxkcd.com/wiki/index.php/2266:_Leap_Smearing
* How to define start and end times for interval triggered components (e.g. "between 2 and 4 pm")?