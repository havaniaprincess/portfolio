# ts_prepear

A command-line binary that converts raw fishing-game screenshots into Tesseract
LSTM training assets (`.png`, `.box`, `.gt.txt`).  
Part of the `text_reader` workspace. Intended to be run **before** `ts_learning`.

## Overview

`ts_prepear` bridges the gap between raw screenshots and a ready-to-train dataset.
It preprocesses images with the `images` crate to isolate OCR regions, then uses
the `tesseract_lib` crate to generate the character-annotation and ground-truth
files that Tesseract's LSTM trainer requires.

## Pipeline

```
data/source/<data_dir>/
  ├── screenshot_1.png
  ├── screenshot_2.png
  └── ...
         │
         ▼  image_process_to_ocr()  (per screenshot)
         │
         ├── data/d1/<data_dir>/
         │     screenshot_1.png.d1.png   ← name/mass region
         │     ...
         │
         └── data/d2/<data_dir>/
               screenshot_1.<i>.d2.png   ← EXP strip segments
               ...
         │
         ▼  make_img()  (per D1 + each D2 segment)
         │
         ├── <stem>.png          ← renamed to training-friendly name
         ├── <stem>.box          ← character bounding-box annotations
         └── <stem>.gt.txt       ← OCR ground-truth text
```

### Steps in detail

| # | Step | Description |
|---|---|---|
| 1 | **Clean** | Deletes `data/d1/<data_dir>/` and `data/d2/<data_dir>/` from any prior run. |
| 2 | **Validate directories** | Asserts the source path exists; recreates D1/D2 output directories. |
| 3 | **Preprocess screenshots** | Calls `image_process_to_ocr` for each PNG, producing D1 and D2 preprocessed images. |
| 4 | **Generate training assets** | Calls `make_img` for every D1 and D2 image to rename, create `.box`, and write `.gt.txt`. |

### `make_img` — training asset generation

For each preprocessed image:

1. **Rename** — dots in the file stem are replaced with underscores to satisfy
   Tesseract's file-naming convention. The `_gt` suffix becomes `.gt` so Tesseract
   correctly associates the ground-truth file.
2. **Generate `.box`** — calls `tesseract lstmbox` via `make_box()` to produce
   per-character bounding-box annotations.
3. **Save `.gt.txt`** — runs OCR via `get_text_tesseract()` and writes the
   recognised text as the ground-truth reference for the LSTM trainer.

## Usage

```
cargo run --release -p ts_prepear -- --data-dir <dataset_name>
```

### Arguments

| Flag | Required | Description |
|---|---|---|
| `--data-dir` / `-d` | yes | Name of the dataset sub-directory under `data/source/`. The same name is used for the `data/d1/` and `data/d2/` output sub-directories. |

### Example

```powershell
cargo run --release -p ts_prepear -- --data-dir train_1
```

This reads all PNGs from `data/source/train_1/`, writes preprocessed images to
`data/d1/train_1/` and `data/d2/train_1/`, and generates `.box` + `.gt.txt`
files alongside each renamed PNG.

## Directory Layout

```
data/
  source/
    <data_dir>/          ← raw PNG screenshots (input)
  d1/
    <data_dir>/          ← D1 preprocessed images + .box + .gt.txt (auto-created)
  d2/
    <data_dir>/          ← D2 EXP segment images + .box + .gt.txt (auto-created)
```

## Relation to Other Crates

| Crate | Role |
|---|---|
| `images` | Provides `image_process_to_ocr` for screenshot preprocessing. |
| `tesseract_lib` | Provides `make_box`, `get_text_tesseract`, `dir_exists`, `list_files`. |
| `ts_learning` | Consumes the `.lstmf` samples generated from the assets produced here. |

## Requirements

- **Tesseract-OCR** installed at `C:/Program Files/Tesseract-OCR/`  
  (requires `tesseract.exe`).
- Language model files (`rus.traineddata`, `rus_hp1.traineddata`, etc.) placed  
  under `./learning_data/start_data/`.

## Dependencies

| Crate | Purpose |
|---|---|
| [`clap`](https://crates.io/crates/clap) `4.5` | CLI argument parsing |
| [`rayon`](https://crates.io/crates/rayon) `1.10` | Parallel iteration (available for future use) |
| [`tokio`](https://crates.io/crates/tokio) `1.46` | Async runtime |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | JSON serialisation (available for future use) |
| [`byteorder`](https://crates.io/crates/byteorder) `1.5` | Binary I/O (available for future use) |
| `images` | PNG preprocessing pipelines (workspace crate) |
| `tesseract_lib` | Tesseract CLI wrappers and filesystem helpers (workspace crate) |
