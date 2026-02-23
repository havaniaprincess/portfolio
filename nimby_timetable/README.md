# nimby_timetable

A week-long transit timetable generator and train-roster scheduler written in Rust.  
Given a set of line definitions and a fleet of rolling stock, the tool produces departure runs, groups them into driver shifts, and assigns every shift to a specific train — tracking maintenance countdowns and work-time across the whole seven-day period.

---

## Features

- **Run generation** — computes second-accurate departure times for a full 7-day week based on configurable base headways and time-of-day / day-of-week peak multipliers.
- **Shift building** — chains consecutive runs (alternating directions) into driver shifts automatically.
- **Train assignment** — two-pass daily allocation:
  1. Trains already at a terminal station are matched first to avoid unnecessary depot trips.
  2. Remaining shifts pull trains from the depot, chosen randomly from eligible candidates to distribute mileage evenly.
- **Maintenance scheduling** — four service tiers (A / B / C / D) with individual countdown timers per train; due services are injected into the timetable before revenue assignments each day.
- **RON I/O** — all input and output files use the [RON](https://github.com/ron-rs/ron) (Rusty Object Notation) format for human-readable configuration and results.

---

## Project structure

```
nimby_timetable/
├── src/
│   ├── main.rs          # Entry point — orchestrates the full pipeline
│   ├── line.rs          # Line / LineDir types and run-generation logic
│   ├── run.rs           # Run, RunArray, ShiftType primitives
│   ├── shifts.rs        # Groups runs into day-indexed driver shifts
│   ├── train.rs         # Train, TrainShift, TrainShiftOut, Place types
│   ├── types.rs         # Seconds / Hms newtypes with serde HH:MM:SS support
│   ├── way_getting.rs   # Shift chaining (get_way) and maintenance dispatch (service_look)
│   ├── math.rs          # Small numeric utilities
│   └── config.rs        # Config struct loaded from config.ron
├── lines.ron            # Line definitions (id, direction, headway, depth, duration)
├── config.ron           # Runtime parameters (now_time, now_week)
├── data/
│   └── <line_id>/
│       ├── trains.ron            # Initial train roster
│       ├── trains_out_<day>.ron  # Train states after each simulated day
│       ├── trains_stat_out_<day>.ron # Intermediate states (post-station pass)
│       ├── trains_out_.ron       # Final end-of-week train states
│       └── weeks/
│           └── 0/
│               ├── shifts.ron       # Day-indexed driver shift map
│               ├── train_shifts.ron # Per-train per-day shift assignment map
│               └── out_shifts.ron   # Display-ready shift output (local-day times)
└── Cargo.toml
```

---

## Configuration

### `lines.ron`

Defines one or more `Line` objects in an array.  Each line has two entries — one per direction (`Right` / `Back`).

| Field       | Type                           | Description |
|-------------|--------------------------------|-------------|
| `id`        | `String`                       | Line identifier (e.g. `"sok"`) |
| `direction` | `Right` \| `Back`              | Direction of travel |
| `base_time` | `"HH:MM:SS"`                   | Default headway between departures |
| `depth`     | `[(divisor, start, end), …]`   | Peak-hour overrides; effective headway = `base_time / divisor` within `[start, end)` |
| `duration`  | `"HH:MM:SS"`                   | One-way travel time (terminal to terminal) |
| `way_time`  | `"HH:MM:SS"`                   | Additional way time (reserved for future use) |

`depth` entries use the absolute week-second clock (`"0:00:00"` = Sunday midnight, `"24:00:00"` = Monday midnight, …).  Multiple overlapping entries are resolved by taking the minimum headway.

### `config.ron`

| Field      | Type         | Description |
|------------|--------------|-------------|
| `now_time` | `"HH:MM:SS"` | Reference clock offset added to departure times during train-assignment comparisons |
| `now_week` | `u32`        | Current simulation week index (reserved for future use) |

### `data/<id>/trains.ron`

A map of `(line_id, train_id) → Train`.  Each `Train` carries:

- `place` — current location (`Depot(name)`, `Station(dir)`, or `Service((end_time, type))`)
- `work_time` — cumulative revenue running time
- `time_to_type_a/b/c/d` — maintenance countdown timers
- `dam_*_prob` — per-second random-fault probabilities (A–D)
- `free_time` — absolute second the train next becomes available
- `home_station` — terminal direction the train is normally associated with

---

## Maintenance service tiers

| Tier | Duration | Interval  | Location          |
|------|----------|-----------|-------------------|
| A    | 1 day    | ~30 days  | Home depot        |
| B    | 3 days   | ~4 months | Home depot        |
| C    | 14 days  | ~15 months| Specialist depot  |
| D    | 45 days  | ~5 years  | Manufacturer      |

When a countdown drops below 24 hours at the start of a simulation day, the service is injected into the train's shift record and all affected counters are reset.

---

## Building and running

**Prerequisites:** Rust toolchain (stable, edition 2021).

```powershell
# Build
cargo build --release

# Run
cargo run --release
```

Output files are written to the `data/` directory as described above.

---

## Dependencies

| Crate        | Purpose |
|--------------|---------|
| `ron`        | RON serialisation / deserialisation |
| `serde`      | Derive macros for `Serialize` / `Deserialize` |
| `serde_json` | JSON support (available but not used in primary output) |
| `rand`       | Random train selection during depot-assignment pass |
