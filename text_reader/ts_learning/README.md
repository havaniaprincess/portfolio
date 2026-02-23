# ts_learning

A command-line binary that automates the full Tesseract LSTM fine-tuning
preparation pipeline.  
Part of the `text_reader` workspace.

## Overview

`ts_learning` takes a directory of annotated training images (PNG + `.box` files),
generates all intermediate artefacts required by Tesseract's LSTM trainer, and
launches the training run in one command.

## Pipeline

```
./learning_data/training_data/<test>/
  ├── image_1.png  +  image_1.box
  ├── image_2.png  +  image_2.box
  └── ...
         │
         ▼  make_lstmf()  (per image)
  image_1.lstmf, image_2.lstmf, ...
         │
         ▼  write training list file  (--lstmf-file)
  <lstmf_file>.txt
         │
         ├──▶  make_unichar()   →  hp1.unicharset
         │
         ├──▶  make_combine()   →  .traineddata assets
         │
         └──▶  make_train()     →  ./learning_data/out_model_2/  (LSTM checkpoints)
```

### Steps in detail

| # | Function | Description |
|---|---|---|
| 1 | `list_files(..., "png")` | Collects all `.png` training images from `./learning_data/training_data/<test>/`. |
| 2 | `make_lstmf()` | Calls `tesseract lstm.train` on each image to produce a binary `.lstmf` sample. Paths are appended to the training list file. |
| 3 | `make_unichar()` | Runs `unicharset_extractor` over all `.box` files to produce `hp1.unicharset`. |
| 4 | `make_combine()` | Runs `combine_lang_model` to assemble `.traineddata` assets from the unicharset and language data files. |
| 5 | `make_train()` | Launches `lstmtraining`, continuing from `rus.lstm`, for up to 100 000 iterations. |

## Usage

```
cargo run --release -p ts_learning -- \
    --test  <session_name>   \
    --lstmf-file <path/to/train_list.txt>
```

### Arguments

| Flag | Required | Description |
|---|---|---|
| `--test` / `-t` | yes | Name of the training session. Resolves to `./learning_data/training_data/<test>/`. |
| `--lstmf-file` / `-l` | yes | Path to the output training list file. Any existing file at this path is deleted and recreated. |

### Example

```powershell
cargo run --release -p ts_learning -- `
    --test train_1 `
    --lstmf-file ./learning_data/start_data/train_1.txt
```

## Directory Layout

```
learning_data/
  training_data/
    <test>/               ← annotated PNG + .box pairs go here
  start_data/             ← base tessdata (rus.lstm, rus.traineddata, langdata/, ...)
  out_model_2/            ← LSTM checkpoint output (auto-created by lstmtraining)
```

## Modules

| Module | Description |
|---|---|
| `main` | CLI parsing and pipeline orchestration. |
| `fs` | Filesystem helpers: `dir_exists`, `list_files`, `list_files_name`, `check_lstmf`. |

### `fs::check_lstmf`

A diagnostic utility that reads and pretty-prints the image path and ground-truth
text embedded in a `.lstmf` binary file. Useful for verifying generated samples
before starting a training run:

```rust
use ts_learning::fs::check_lstmf;
use std::path::Path;

check_lstmf(Path::new("./learning_data/training_data/train_1/sample.lstmf"));
// Image path: ./learning_data/training_data/train_1/sample.png
// Text: some ground truth
// Length (in bytes): 16
```

## Requirements

- **Tesseract-OCR** installed at `C:/Program Files/Tesseract-OCR/`  
  (requires `tesseract.exe`, `lstmtraining.exe`, `combine_lang_model.exe`,
  `unicharset_extractor.exe`).
- Base language data in `./learning_data/start_data/`  
  (`rus.lstm`, `rus.traineddata`, `langdata/`, unicharset, wordlist, etc.).

## Dependencies

| Crate | Purpose |
|---|---|
| [`clap`](https://crates.io/crates/clap) `4.5` | CLI argument parsing |
| [`rayon`](https://crates.io/crates/rayon) `1.10` | Parallel iteration (available for future use) |
| [`tokio`](https://crates.io/crates/tokio) `1.46` | Async runtime |
| [`serde`](https://crates.io/crates/serde) / [`serde_json`](https://crates.io/crates/serde_json) | JSON serialisation (available for future use) |
| [`byteorder`](https://crates.io/crates/byteorder) `1.5` | Little-endian binary reading for `.lstmf` inspection |
| `images` | PNG utilities (workspace crate) |
| `tesseract_lib` | Tesseract CLI wrappers (workspace crate) |
