# hp_overlay

A lightweight Rust binary that records global keyboard and mouse events to a
semicolon-delimited CSV file, timestamped in Unix milliseconds. It is used to
capture device control events during a fishing session so they can later be
correlated with catch data for statistical analysis.

## How it works

1. The binary is launched with a `--test` argument that names the current session.
2. It creates `data/source/<test>_control.csv` with the header `timestamp;key`.
3. A global input listener (via `rdev`) runs on the main thread and intercepts
   the following events:

| Event                  | CSV label    |
|------------------------|-------------|
| Key `1`                | `1`         |
| Key `2`                | `2`         |
| Key `3`                | `3`         |
| Key `0`                | `0`         |
| `Backspace`            | `backspace` |
| `Space`                | `space`     |
| `F12`                  | `f12`       |
| Mouse left release     | `left`      |
| Mouse right release    | `right`     |
| Mouse middle release   | `middle`    |

4. Each captured event is immediately appended to the CSV file and flushed to
   disk, so no events are lost if the process is terminated.

## Output format

```
timestamp;key
1708600000123;1
1708600005456;left
1708600010789;2
```

| Column      | Type    | Description                          |
|-------------|---------|--------------------------------------|
| `timestamp` | `u128`  | Unix timestamp in milliseconds       |
| `key`       | `String`| Label of the key or mouse button     |

## Usage

```bash
hp_overlay --test my_session
# output: data/source/my_session_control.csv
```

### Short form

```bash
hp_overlay -t my_session
```

The program runs until it is manually stopped (`Ctrl+C` or process kill).

## Building

```bash
cargo build --release
```

The compiled binary will be at `target/release/hp_overlay`.

## Dependencies

| Crate    | Purpose                                    |
|----------|--------------------------------------------|
| `rdev`   | Cross-platform global keyboard/mouse hooks |
| `clap`   | CLI argument parsing                       |
| `chrono` | Date/time utilities (reserved)             |
