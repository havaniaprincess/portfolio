# hp_statistic

A Rust library crate for generating aggregated catch statistics from the fishing
session database. It reads per-species and per-map/point CSV logs produced by
`database_lib` and outputs merged or summarized CSV files suitable for further
analysis.

## Overview

The crate exposes a single type — `DBStatistic` — that wraps the database root
path and provides two high-level operations:

| Method                 | Description                                              |
|------------------------|----------------------------------------------------------|
| `get_point_info_fish`  | Per-point summary (average mass, count, catch rate) for one species |
| `make_statistic`       | Full merged catch log from all map-point source files    |

---

## Data layout expected on disk

```
database_data/
├── fishes/
│   └── <fish_name>.csv       # per-species catch log
└── maps/
    └── <map>/
        └── <point>.csv       # per-point catch log
```

### Source CSV column layout (map/point files)

```
name ; test ; map ; point ; timestamp ; mass ; long ;
exp  ; exp_l ; exp_happy ; exp_prem ; exp_sum ; exp_drink ; device ; exp_real
```

Timestamps are stored as formatted strings: `YYYYMMDD_HHMMSS`.

---

## API

### `DBStatistic::get_point_info_fish`

```rust
pub fn get_point_info_fish(
    &self,
    fish_name: &String,
    out_name:  &String,
    maps:      &Option<Vec<String>>,
)
```

Reads `<db_root>/fishes/<fish_name>.csv`, groups records by
`(point, map, test)`, and writes a summary CSV to `out_name`.

**Output header:** `map;point;test;avg_mass;count;point_rate`

| Column       | Description                                              |
|--------------|----------------------------------------------------------|
| `map`        | Map name                                                 |
| `point`      | Fishing point identifier                                 |
| `test`       | Session name                                             |
| `avg_mass`   | Average catch mass in grams                              |
| `count`      | Number of catches at this point in this session          |
| `point_rate` | `count / total_attempts` — how often this point produced a catch |

If `maps` is `Some`, only records from the listed maps are included.

---

### `DBStatistic::make_statistic`

```rust
pub fn make_statistic(
    &self,
    out_name:  &String,
    maps:      &Option<Vec<String>>,
    timestamp: u128,
)
```

Scans all map-point CSV files (or only those in `maps` if provided), filters out
records older than `timestamp` (Unix milliseconds, inclusive lower bound), and
writes a merged log to `out_name`.

**Output header:**
```
name;test;map;point;timestamp;mass;long;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;device;exp_real
```

If `maps` is `None`, all subdirectories of `<db_root>/maps/` are discovered
automatically.

> **Note:** Source timestamps use a fixed −2 h offset applied during conversion
> to align with the UTC-based unix values used throughout the project.

**Panics** if any required directory cannot be read.

---

## Usage example

```rust
use hp_statistic::statistic::DBStatistic;

let db = DBStatistic("../database_data/".to_string());

// Full merged statistics since 2025-06-01 00:00 UTC
db.make_statistic(
    &"../statistic.csv".to_string(),
    &None,
    1748736000000,
);

// Per-point summary for one species
db.get_point_info_fish(
    &"bass".to_string(),
    &"../bass_points.csv".to_string(),
    &Some(vec!["lake_north".to_string()]),
);
```

---

## Dependencies

| Crate           | Purpose                                      |
|-----------------|----------------------------------------------|
| `csv`           | CSV parsing                                  |
| `serde`         | Serialization / deserialization              |
| `serde_json`    | JSON I/O (reserved)                          |
| `chrono`        | Timestamp parsing (`YYYYMMDD_HHMMSS` → UTC)  |
| `tesseract_lib` | File system utilities                        |
