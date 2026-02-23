# control_log

A Rust library crate for loading and querying device control event logs stored in CSV files.

## Overview

`control_log` provides the `ControlList` type — a chronologically ordered set of `(timestamp_ms, event_code)` pairs. It is designed to answer questions like *"what was the device state at a given point in time?"* by replaying the event history up to that moment.

## Data Format

Events are stored in semicolon-delimited CSV files. Each row contains exactly two fields:

```
timestamp_ms;event_code
1708600000000;1
1708600005000;2
1708600010000;3
```

| Field           | Type     | Description                              |
|----------------|----------|------------------------------------------|
| `timestamp_ms` | `u128`   | Unix timestamp in milliseconds           |
| `event_code`   | `String` | Device state code (`"1"`, `"2"`, `"3"`) |

## API

### `ControlList::from_path(path: &String) -> Self`

Reads a CSV event log from the given file path and returns a populated `ControlList`.

**Panics** if the file cannot be opened or if any row fails to deserialize.

---

### `ControlList::get_last_device_event(time: u128) -> String`

Returns the most recent valid event code (`"1"`, `"2"`, or `"3"`) that occurred **before** `time`.  
Events at or after `time` are ignored. Returns `"1"` as a default if no qualifying event is found.

## Usage

```rust
use control_log::ControlList;

let log = ControlList::from_path(&"events.csv".to_string());
let state = log.get_last_device_event(1708600007000);
println!("Device state: {}", state); // "2"
```

## Dependencies

| Crate  | Purpose                   |
|--------|---------------------------|
| `csv`  | CSV parsing               |
| `serde`| Serialization / deserialization |
