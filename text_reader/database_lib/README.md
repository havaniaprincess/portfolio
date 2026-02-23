# database_lib

A Rust library crate for managing a fishing catch database: loading application config,
accumulating per-species statistical models, validating new measurements against historical
data, and recalculating EXP columns in CSV log files.

## Overview

The crate is organised into three public modules:

| Module       | Purpose                                                                 |
|--------------|-------------------------------------------------------------------------|
| `config`     | Application config (fish dictionary, device presets, rigs, EXP types)  |
| `db_config`  | Per-species statistical aggregates and cross-metric validation           |
| `fs`         | JSON helpers for loading serialized aggregate tables from disk           |

The top-level `lib.rs` exposes three high-level entry points that tie all modules together.

---

## Data layout on disk

```
database_data/
├── fishes/          # per-species CSV catch logs
│   └── <name>.csv
├── maps/            # per-map / per-point CSV logs
│   └── <map>/<point>.csv
├── tests/           # per-session CSV logs
│   └── <test>.csv
└── stats/
    └── fishes/      # serialized statistical models
        └── <name>/
            ├── long_mass.json   # length(mm) → {sum, count}
            └── mass_exp.json    # mass(g)    → {sum, count}

data/
└── config.json      # application configuration (JSON)
```

### CSV column layout

```
mark ; timestamp ; fish_name ; mass ; long ; test ; map ; point ;
exp  ; exp_l ; exp_happy ; exp_prem ; exp_sum ; exp_drink ;
long_mass_mark ; long_mass_err ; mass_exp_mark ; mass_exp_err ;
device ; exp_real
```

---

## Modules

### `config`

Defines the full application configuration and exposes it through `Config`.

**Key types**

| Type            | Description                                                         |
|-----------------|---------------------------------------------------------------------|
| `Config`        | Top-level config: fish dict, tag sets, EXP mappings, presets, rigs |
| `Fish`          | Fish metadata: expected weights, recognition tags, primary tags     |
| `DevicePreset`  | Device configuration used during CSV recalculation                  |
| `Rig`           | EXP multiplier for a rod/rig combination                            |
| `Exp`           | EXP type descriptor with recognition tags                           |
| `ExpType`       | Enum of recognised EXP categories                                   |
| `LineType`      | Enum of fishing line material types                                 |

**Key methods**

```rust
Config::new() -> Config                      // empty default config
Config::read(path: &String) -> Option<Config> // load + derive tag sets from JSON
Config::save(path: &String)                  // serialize to pretty JSON
Config::get_clear_exp(preset, exp) -> f64    // total EXP → base EXP via rig multiplier
```

---

### `db_config`

Maintains per-species statistical models and validates new measurements.

**Key types**

| Type                 | Description                                              |
|----------------------|----------------------------------------------------------|
| `MistakeProb`        | Validation result enum with absolute deviation payload   |
| `StatFunc(sum, cnt)` | Single aggregate bucket; average = `sum / cnt`           |
| `FishDatabaseConfig` | Two-map model: `length→mass` and `mass→exp`              |
| `DatabaseConfig`     | Top-level manager with lazy disk loading and persistence |
| `Fish`               | Helper for processing one raw CSV record                 |

**`MistakeProb` variants**

| Variant              | Meaning                                          |
|----------------------|--------------------------------------------------|
| `FullAccuracy(f64)`  | Deviation ≤ 5 % of the historical average        |
| `MaybeMistake(f64)`  | Deviation > 5 % but may still be valid           |
| `FullyNotSure(f64)`  | Significant deviation from the historical model  |
| `NotEnoughData`      | No historical records available for comparison   |

**`DatabaseConfig` methods**

```rust
DatabaseConfig::new(path: &String) -> Self
DatabaseConfig::add_row(name, mass, long, exp)              // update + load from disk
DatabaseConfig::add_row_without_old_data(name, mass, long, exp) // update in-memory only
DatabaseConfig::save_db()                                   // flush all models to disk
DatabaseConfig::check_value_long_mass(name, mass, long) -> MistakeProb
DatabaseConfig::check_value_mass_exp(name, mass, exp)  -> MistakeProb
DatabaseConfig::recalculate_fish(fish_name)                 // replay CSV log
```

Validation uses **exact bucket lookup** when the measured value matches a historical
bucket key exactly, or **linear interpolation** (`y = ax + b`) between the two nearest
buckets otherwise.

---

### `fs`

Low-level JSON I/O helpers.

```rust
json_hashmap_load(path)     -> Option<BTreeMap<u64, StatFunc>>  // load aggregate table
json_regression_load(path)  -> Option<Vec<f64>>                 // load regression coefficients
```

Both functions return `None` silently if the file does not exist.

---

## Public API (`lib.rs`)

```rust
/// Read a CSV log and accumulate all records into the database, then flush to disk.
pub fn add_test_to_db(path: &Path);

/// Rebuild statistical models for every species listed in the config by replaying
/// their CSV logs, then flush to disk.
pub fn recalculate_stats();

/// Re-compute all EXP columns in a CSV log file in place.
/// Deletes the original file and writes the corrected version.
pub fn recalculate_csv(path: &Path);
```

### Usage example

```rust
use std::path::Path;
use database_lib::{add_test_to_db, recalculate_csv};

let path = Path::new("../data/result/hour_1.csv");
recalculate_csv(path);   // fix EXP columns in place
add_test_to_db(path);    // accumulate stats and persist
```

---

## Dependencies

| Crate           | Purpose                              |
|-----------------|--------------------------------------|
| `csv`           | CSV parsing                          |
| `serde`         | Serialization / deserialization      |
| `serde_json`    | JSON read / write                    |
| `rayon`         | Parallel iteration over config sets  |
| `strsim`        | Levenshtein distance for tag matching|
| `tokio`         | Async runtime (reserved)             |
| `tesseract_lib` | File system utilities                |
