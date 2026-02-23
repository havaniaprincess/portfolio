# tesseract

A command-line binary for automated OCR-based analysis of fishing game screenshots.  
Part of the `text_reader` workspace.

## Overview

The binary reads a batch of annotated PNG screenshots, preprocesses them with the
`images` crate, runs Tesseract OCR over the results, parses and cross-validates the
detected data (fish name, catch parameters, EXP breakdown), and writes one
normalised CSV row per screenshot.

## Modules

| Module | Description |
|---|---|
| `main` | CLI argument parsing, directory setup, preprocessing orchestration, OCR dispatch, and CSV output. |
| `name_process` | OCR text post-processing: tag-distance fish identification, mass/length extraction, and full EXP reconstruction. |
| `tesseract` | Thin wrappers around the Tesseract CLI for running OCR and generating `.box` training files. |
| `image` | Re-exports the `images` crate types used by the pipeline. |

## Pipeline

```
screenshots (PNG)
       │
       ▼
image_process_to_ocr()          ← images crate
  ├── D1: name/mass region      → blackwhite/<test>/<file>.d1.png
  └── D2: EXP strip segments    → blackwhite/<test>/<file>.<i>.d2.png
       │
       ▼
Tesseract OCR (rus_hp1 + rus)   ← D1 image, two language models
       │
       ▼
get_config_item()               → fish name (tag-distance match)
get_fish_param()                → mass (g) + length (mm)
get_exp()                       → full EXP breakdown
       │
       ▼
CSV row appended to data/result/<test>.csv
```

## Usage

```
cargo run --release -p tesseract -- \
    --test  <test_id>           \
    --config ./data/config.json \
    --map   <map_name>          \
    --point <point_name>        \
    [--d1 <preset>]             \
    [--d2 <preset>]             \
    [--d3 <preset>]             \
    [--not-device]
```

### Arguments

| Flag | Required | Description |
|---|---|---|
| `--test` / `-t` | yes | Test session ID; used as the source sub-directory name and output file stem. |
| `--map` / `-m` | yes | Fishing map name written verbatim to the CSV. |
| `--point` / `-p` | yes | Fishing point identifier written verbatim to the CSV. |
| `--config` / `-c` | yes | Path to the JSON runtime config file. |
| `--not-device` | no | Skip control-log device resolution; infer device preset from detected light-bonus EXP instead. |
| `--d1` | no | Device preset name for control slot 1 (default: `"default"`). |
| `--d2` | no | Device preset name for control slot 2 (default: `"default"`). |
| `--d3` | no | Device preset name for control slot 3 (default: `"default"`). |

### Example calls

```powershell
# Without device tracking
cargo run --release -p tesseract -- `
    --test worm_92_79 --config .\data\config.json `
    --map losinoe --point 92_79 --not-device 2> out_tess

# With explicit device presets per control slot
cargo run --release -p tesseract -- `
    --test vyp_nav_89_81 --config .\data\config.json `
    --map losinoe --point 89_81 `
    --d1 mah_d1_t1 --d2 mah_d2_t1 --d3 match_d3_t1 2> out_tess
```

## Directory Layout

```
data/
  source/<test_id>/          ← input PNG screenshots
  source/<test_id>_control.csv  ← device control log (optional)
  blackwhite/<test_id>/      ← preprocessed OCR images (auto-created)
  result/<test_id>.csv       ← output CSV
database_data/               ← statistics database files
```

## Output CSV

Semicolon-delimited, one row per screenshot:

| Column | Description |
|---|---|
| `mark` | `FAIL NAME` when fish detection failed, empty otherwise. |
| `timestamp` | Capture datetime parsed from the file name (`YYYYmmdd_HHMMSS`). |
| `fish_name` | Detected fish name from the config. |
| `mass` | Catch mass in grams. |
| `long` | Catch length in millimetres. |
| `test` | `--test` argument value. |
| `map` | `--map` argument value. |
| `point` | `--point` argument value. |
| `exp` | Base EXP. |
| `exp_l` | Light-bonus EXP. |
| `exp_happy` | Happy-hour bonus EXP. |
| `exp_prem` | Premium bonus EXP. |
| `exp_sum` | Total EXP (all components summed). |
| `exp_drink` | Drink-bonus EXP. |
| `long_mass_mark` | Validation quality label for the mass/length pair. |
| `long_mass_err` | Validation error value for the mass/length pair. |
| `mass_exp_mark` | Validation quality label for the base EXP. |
| `mass_exp_err` | Validation error value for the base EXP. |
| `device` | Resolved device preset ID. |
| `exp_real` | Rig-adjusted real EXP (`base_exp × rig.exp_mul`). |

## `name_process` — Key Functions

### `get_fish_tags`

Computes the minimum sliding-window Levenshtein distance between the OCR string
and every known tag. Returns a `Vec<(tag, distance)>`.

### `get_fish_param`

Runs `get_fish_mass` in parallel over all OCR candidate strings, sorts results by
validation error, and returns the best `(mass, length, error)` triple.

### `get_fish_mass`

Parses an OCR string line-by-line with a numeric regex, takes the first two numbers
on each line as `(mass, length)`, cross-validates against the statistics database,
and returns the candidate with the smallest error.

### `get_exp`

For each EXP snippet file:
1. Runs Tesseract with every configured language model.
2. Identifies the EXP category via tag matching.
3. Parses numeric tokens and validates against the DB.
4. Converts raw values to clear (rig-independent) base using the device preset.
5. Calls `find_base_exp` to pick the most reliable base value.
6. Derives all bonus components and the rig-adjusted real EXP.

Returns a `HashMap<ExpType, (value, error)>`.

## `tesseract` Module

Provides `get_box_learn`, a thin wrapper around the Tesseract CLI that generates
`.box` training files for a given image. Reads the Tesseract binary path from the
`TESSDIR` environment variable.

```
TESSDIR=C:\Tesseract-OCR
```

## Dependencies

| Crate | Purpose |
|---|---|
| [`clap`](https://crates.io/crates/clap) `4.5` | CLI argument parsing |
| [`regex`](https://crates.io/crates/regex) `1.11` | Numeric token extraction from OCR text |
| [`rayon`](https://crates.io/crates/rayon) `1.10` | Parallel processing of OCR candidates |
| [`tokio`](https://crates.io/crates/tokio) `1.46` | Async runtime |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | Config file serialisation |
| [`strsim`](https://crates.io/crates/strsim) `0.11` | Levenshtein distance for tag matching |
| [`chrono`](https://crates.io/crates/chrono) `0.4` | Timestamp parsing from file names |
| `images` | PNG preprocessing pipelines (workspace crate) |
| `tesseract_lib` | Tesseract CLI wrapper and filesystem helpers (workspace crate) |
| `database_lib` | Statistics database and config types (workspace crate) |
| `control_log` | Device control-log reader (workspace crate) |
