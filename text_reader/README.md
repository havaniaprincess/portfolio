# text_reader

A Rust workspace for automated OCR-based analysis of fishing-game screenshots.  
The system captures in-game catch data (fish name, mass, length, EXP breakdown),
cross-validates it against a local statistics database, and writes normalised CSV
reports for further analysis.

## Workspace Structure

```
text_reader/
├── control_log/     library  – device control-log reader
├── database_lib/    library  – statistics database and runtime config types
├── hp_overlay/      binary   – keyboard/mouse event logger (control log recorder)
├── hp_statistic/    library  – catch-statistics aggregation and reporting
├── images/          library  – PNG processing and OCR preprocessing pipelines
├── tensor/          library  – tensor / matrix algebra utilities
├── tesseract/       binary   – main OCR detection and CSV output pipeline
├── tesseract_lib/   library  – Tesseract CLI wrappers and filesystem helpers
├── ts_learning/     binary   – Tesseract LSTM fine-tuning launcher
├── ts_prepear/      binary   – training-data preparation from screenshots
└── src/             root crate (fish_view, placeholder)
```

## Crate Overview

### `control_log`

Provides `ControlList` — a chronologically sorted set of `(timestamp_ms, device_slot)`
events. Loaded from a semicolon-delimited CSV produced by `hp_overlay`. Used by
the `tesseract` pipeline to resolve which device preset was active at each capture
moment.

→ See [control_log/README.md](control_log/README.md)

---

### `database_lib`

Persistent statistics database stored as CSV files under `database_data/`.  
Provides:
- `DatabaseConfig` — aggregate of per-fish catch records; used to cross-validate
  OCR-detected mass, length, and EXP values.
- `Config` — runtime configuration: fish definitions, tag lists, device presets,
  rig multipliers, EXP-type tags.
- `add_test_to_db` — ingests a processed result CSV into the database.

→ See [database_lib/README.md](database_lib/README.md)

---

### `hp_overlay`

A keyboard/mouse event listener that records device-slot change events in real
time. Writes a semicolon-delimited `<test>_control.csv` file consumed later by
the `tesseract` pipeline to associate each screenshot with the correct device
preset.

→ See [hp_overlay/README.md](hp_overlay/README.md)

---

### `hp_statistic`

Reads the `database_data/` directory and generates summary statistics across all
recorded catch sessions. Outputs aggregated CSV reports for analysis (catch counts
per map/point/session, per-fish data distributions, etc.).

→ See [hp_statistic/README.md](hp_statistic/README.md)

---

### `images`

Core image-processing library. Provides:
- `ImagePNG` — read/write wrapper around the `png` crate.
- `Algorythms` trait — grayscale conversion, per-pixel transforms, Lanczos
  resampling, nearest-neighbor resize, crop, white-pixel counting.
- `image_process_to_ocr` / `image_process_to_ocr_short` — ready-made OCR
  preprocessing pipelines that isolate the fish-name/mass region (D1) and EXP
  strip segments (D2) from a screenshot.

→ See [images/README.md](images/README.md)

---

### `tensor`

Linear-algebra utilities (tensor and matrix operations). Supports internal
mathematical computations used across the workspace.

---

### `tesseract`

**Main binary.** Orchestrates the full end-to-end detection pipeline:

1. Preprocesses screenshots with `images::image_process_to_ocr`.
2. Runs Tesseract OCR with multiple language models.
3. Detects fish name, mass, length, and all EXP components.
4. Cross-validates results against `database_lib`.
5. Appends one normalised semicolon-delimited row per screenshot to a result CSV.

→ See [tesseract/README.md](tesseract/README.md)

---

### `tesseract_lib`

Thin wrappers around the Tesseract-OCR CLI executables:
- `get_text_tesseract` — run OCR and return text.
- `make_box` / `make_lstmf` — generate training assets.
- `make_train` / `make_train_2` — launch LSTM fine-tuning.
- `make_combine` / `make_unichar` — assemble language-model data.
- Filesystem helpers: `dir_exists`, `list_files`, `cvs_file_exists`, `cvs_file_adding`.

→ See [tesseract_lib/README.md](tesseract_lib/README.md)

---

### `ts_prepear`

**Training-data preparation binary.** Converts raw screenshots into Tesseract
LSTM training assets:

1. Preprocesses screenshots to D1/D2 images.
2. Renames files to Tesseract-compatible stems.
3. Generates `.box` character-annotation files.
4. Saves `.gt.txt` ground-truth reference texts.

Run this before `ts_learning`.

→ See [ts_prepear/README.md](ts_prepear/README.md)

---

### `ts_learning`

**LSTM fine-tuning binary.** Takes the assets produced by `ts_prepear` and runs
the full Tesseract training-preparation pipeline:

1. Generates `.lstmf` binary samples from PNG images.
2. Extracts a unicharset from `.box` files.
3. Assembles combined language-model data.
4. Launches `lstmtraining` for up to 100 000 iterations.

→ See [ts_learning/README.md](ts_learning/README.md)

---

## End-to-End Workflow

```
┌─────────────────────────────────────────────────────────────┐
│  DATA COLLECTION                                            │
│                                                             │
│  hp_overlay  →  <test>_control.csv  (device slot events)   │
│  Game        →  data/source/<test>/*.png  (screenshots)     │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  OCR DETECTION  (tesseract binary)                          │
│                                                             │
│  images::image_process_to_ocr                               │
│    → D1 (name/mass region)                                  │
│    → D2 (EXP strip segments)                                │
│  Tesseract OCR  →  fish name, mass, length, EXP             │
│  database_lib   →  cross-validation                         │
│    → data/result/<test>.csv                                 │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  DATABASE UPDATE                                            │
│                                                             │
│  database_lib::add_test_to_db  →  database_data/            │
└──────────────────────────┬──────────────────────────────────┘
                           │
                           ▼
┌─────────────────────────────────────────────────────────────┐
│  STATISTICS  (hp_statistic)                                 │
│                                                             │
│  database_data/  →  statistic.csv / per-fish reports        │
└─────────────────────────────────────────────────────────────┘
```

## Model Fine-Tuning Workflow

```
Raw screenshots
      │
      ▼  ts_prepear  →  .png + .box + .gt.txt
      │
      ▼  ts_learning →  .lstmf → unicharset → .traineddata → LSTM checkpoints
      │
      ▼  Deploy updated .traineddata to learning_data/start_data/
```

## Requirements

- **Rust** 2024 edition (stable toolchain).
- **Tesseract-OCR** installed at `C:/Program Files/Tesseract-OCR/`.
- Language model files (`.traineddata`) in `./learning_data/start_data/`.
- `database_data/` directory for the statistics database.

## Quick Start

```powershell
# 1. Record device events during a fishing session
cargo run --release -p hp_overlay -- --test my_session

# 2. Run OCR detection on captured screenshots
cargo run --release -p tesseract -- `
    --test my_session --config ./data/config.json `
    --map losinoe --point 89_81 --not-device

# 3. (Optional) Ingest results into the statistics database
# add_test_to_db is called internally by the tesseract pipeline

# 4. Generate statistics reports
# hp_statistic tests / API calls

# 5. Fine-tune language model (when new training data is available)
cargo run --release -p ts_prepear  -- --data-dir train_1
cargo run --release -p ts_learning -- --test train_1 --lstmf-file ./learning_data/start_data/train_1.txt
```
