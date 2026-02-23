# tesseract_lib

A Rust library crate providing Tesseract OCR CLI wrappers and filesystem utilities.  
Part of the `text_reader` workspace. Used primarily by the `tesseract` binary crate.

## Overview

The crate contains two modules:

| Module | Description |
|---|---|
| `commands` | Thin wrappers around Tesseract-OCR and LSTM-training CLI executables. |
| `fs` | Filesystem helpers for directory validation, file listing, and CSV I/O. |

## Modules

### `commands`

All functions call executables installed at `C:/Program Files/Tesseract-OCR/`.  
Custom tessdata is loaded from `./learning_data/start_data`.

#### `get_text_tesseract`

Runs Tesseract OCR on a single PNG image and returns the recognised text.

```rust
pub fn get_text_tesseract(path: &String, lang: &str) -> String
```

- Invokes the Tesseract CLI in page-segmentation mode 3 (fully automatic).
- Output is written to a temporary `output.txt`, read, and then deleted.
- `lang` accepts any installed language model name (e.g. `"rus"`, `"rus_hp1"`, `"rus_hp2"`).

#### `make_box`

Generates an LSTM `.box` character-annotation file for a training image.

```rust
pub fn make_box(path: &Path, lang: &str) -> String
```

- Runs Tesseract in `lstmbox` mode.
- Skips generation if the `.box` file already exists (idempotent).
- Returns the path to the `.box` file.

#### `make_lstmf`

Converts a training PNG into a binary `.lstmf` sample consumed by LSTM training.

```rust
pub fn make_lstmf(path: &Path) -> String
```

- Uses the Russian language model with 300 DPI hint.
- Returns the path to the generated `.lstmf` file.

#### `make_train`

Fine-tunes the Russian LSTM model from a list of `.lstmf` training samples.

```rust
pub fn make_train(lstmf_file: &String)
```

- Continues from `./learning_data/start_data/rus.lstm`.
- Writes checkpoints to `./learning_data/out_model_2/`.
- Stops after at most 100 000 iterations.
- `lstmf_file` is a text file with one `.lstmf` path per line.

#### `make_train_2`

Alternative (legacy) LSTM training configuration.

```rust
pub fn make_train_2()
```

- Same purpose as `make_train` but uses a fixed training list `train_1.txt` and
  outputs to `./learning_data/out_model/rus_model`.
- Kept for comparison and reference.

#### `make_combine`

Assembles a combined `.traineddata` file from language data components.

```rust
pub fn make_combine()
```

- Invokes `combine_lang_model.exe` with the `hp1` unicharset, Russian wordlist,
  numbers list, and punctuation list.
- Writes output to `./learning_data/start_data`.
- Run after `make_unichar` and before `make_train`.

#### `make_unichar`

Extracts a unicharset from a collection of `.box` files.

```rust
pub fn make_unichar(path: &Vec<String>)
```

- Invokes `unicharset_extractor.exe` with normalisation mode 2.
- Writes `hp1.unicharset` to `./learning_data/start_data/`.
- Run before `make_combine`.

---

### `fs`

Pure filesystem utilities with no external dependencies.

#### `dir_exists`

```rust
pub fn dir_exists<P: AsRef<Path>>(path: P, need_create: bool) -> Result<(), String>
```

Checks whether a path exists. When `need_create` is `true`, missing directories
are created recursively (`fs::create_dir_all`). Returns `Err` if the path is
absent and creation was not requested.

#### `list_files`

```rust
pub fn list_files<P: AsRef<Path>>(path: P) -> Result<Vec<String>, String>
```

Returns the file names (not full paths) of all direct file children in the given
directory. Subdirectories are ignored.

#### `cvs_file_exists`

```rust
pub fn cvs_file_exists<P: AsRef<Path>>(path: P, header: &String) -> Result<(), String>
```

Ensures a CSV file exists. When the file is absent it is created and `header` is
written as the first line. Existing files are left untouched.

#### `cvs_file_adding`

```rust
pub fn cvs_file_adding<P: AsRef<Path>>(path: P, str: &String) -> Result<(), String>
```

Appends one row/string to an existing CSV file in append mode.

## Training Workflow

The `commands` module supports a complete LSTM fine-tuning pipeline:

```
PNG training images
       │
       ▼
make_box()          → .box files (character annotations)
       │
       ▼
make_unichar()      → hp1.unicharset
       │
       ▼
make_combine()      → .traineddata (language model assets)
       │
       ▼
make_lstmf()        → .lstmf files (binary training samples)
       │
       ▼
make_train()        → fine-tuned LSTM checkpoints
```

## Usage

Add the crate to your workspace member's `Cargo.toml`:

```toml
[dependencies]
tesseract_lib = { path = "../tesseract_lib" }
```

Minimal OCR example:

```rust
use tesseract_lib::commands::get_text_tesseract;
use tesseract_lib::fs::list_files;

let files = list_files("data/source/my_test").unwrap();
for file in files {
    let text = get_text_tesseract(&file, "rus");
    println!("{}", text);
}
```

## Requirements

- **Tesseract-OCR** installed at `C:/Program Files/Tesseract-OCR/`
  (includes `tesseract.exe`, `lstmtraining.exe`, `combine_lang_model.exe`,
  `unicharset_extractor.exe`).
- Language model files (`.traineddata`) placed under
  `./learning_data/start_data/`.

## Dependencies

No external crate dependencies. Uses only the Rust standard library.
