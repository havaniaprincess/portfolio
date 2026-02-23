# images

A Rust library crate for PNG image loading, manipulation, and OCR preprocessing.  
Part of the `text_reader` workspace.

## Overview

The crate provides:

- A safe, high-level wrapper around the [`png`](https://crates.io/crates/png) crate for reading and writing PNG files.
- An `Algorythms` trait with common image-processing operations (grayscale conversion, per-pixel transforms, Lanczos resampling, nearest-neighbor resize, crop, white-pixel counting).
- Ready-made OCR preprocessing pipelines that prepare game screenshot regions for Tesseract.

## Modules

| Module | Description |
|---|---|
| `png::base` | Core data structures: `ImagePNG`, `ColorTypePNG`, `BitDepthPNG`, `PixelSize`. Read/write PNG files, access raw pixel bytes and metadata. |
| `png::algorythms` | Implements the `Algorythms` trait for `ImagePNG`. Contains Lanczos resampling kernel, nearest-neighbor resize, crop, grayscale and generic per-pixel transforms. |
| `algorythms` | Defines the `Algorythms` trait. Contains high-level OCR preprocessing functions (`image_process_to_ocr`, `image_process_to_ocr_short`) and the EXP-strip segmentation helper `exp_cut`. |

## Key Types

### `ImagePNG`

The main image container. Stores decoded pixel bytes alongside metadata (width, height, color type, bit depth, DPI, optional palette).

```rust
let img = ImagePNG::read(&"screenshot.png".to_string());
img.write(&"output.png".to_string());
```

### `ColorTypePNG`

Mirrors the PNG standard color types:

| Variant | Channels |
|---|---|
| `Grayscale` | 1 |
| `GrayscaleAlpha` | 2 |
| `Rgb` | 3 |
| `Rgba` | 4 |
| `Indexed` | palette-based (3-byte entries) |

### `PixelSize`

Distinguishes between direct-color images (`Direct(n)`) and indexed/palette images (`Hash(n)`). Controls how algorithms iterate over the pixel buffer.

## `Algorythms` Trait

Implemented for `ImagePNG`. All methods return a new image; the original is not modified.

```rust
pub trait Algorythms {
    fn to_gray(&self, func: fn(&[u8]) -> u8) -> Self;
    fn to_need(&self, func: fn(&[u8]) -> Vec<u8>) -> Self;
    fn resize(&self, w: u32, h: u32, window: usize) -> Self;       // Lanczos
    fn resize_bomzh(&self, w: u32, h: u32, window: usize) -> Self; // nearest-neighbor
    fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Self;
    fn white_count(&self) -> u64;
}
```

### `to_gray`

Collapses every pixel to a single luminance byte using a caller-supplied function.  
For indexed PNG the palette entries are transformed; for already-grayscale images the image is returned unchanged.

```rust
// Average of all channels
let gray = img.to_gray(|px| {
    let sum: u16 = px.iter().map(|&b| b as u16).sum();
    (sum / px.len() as u16) as u8
});
```

### `to_need`

Applies an arbitrary per-pixel transform that may change channel values or count.  
Useful for color quantization, color-range remapping, or channel reordering.

```rust
// Round each channel to the nearest multiple of 8 (color quantization)
let quantized = img.to_need(|px| px.iter().map(|b| (b / 8) * 8).collect());
```

### `resize` (Lanczos)

High-quality resize using the Lanczos resampling kernel.  
`window` is the support radius (2 = Lanczos-2, 3 = Lanczos-3). Larger values produce sharper results at higher computational cost.

```rust
let resized = img.resize(1280, 720, 3);
```

### `resize_bomzh` (nearest-neighbor)

Fast resize by direct source-pixel sampling. No interpolation — faster but produces
blocky artefacts when upscaling. Suitable for enlarging binary/thresholded images
before OCR.

```rust
let upscaled = img.resize_bomzh(2560, 2560, 5);
```

### `crop`

Extracts a rectangular sub-image. Pixels outside image bounds are silently skipped;
the returned image may be smaller than requested.

```rust
let region = img.crop(100, 50, 400, 300); // x, y, width, height
```

### `white_count`

Returns the number of pixels where all three color channels are ≥ 250.
Used as a signal-strength heuristic during EXP-strip segmentation.

## OCR Preprocessing Pipelines

### `image_process_to_ocr_short`

Lightweight pipeline for the fish-name / mass region (D1):

1. Square center crop based on image height.
2. Color quantization — each channel rounded to the nearest multiple of 8.
3. Two targeted color-to-white promotions (game UI accent colors).
4. Binary threshold to black/white grayscale (channels ≥ 230 → 255, else 0).
5. Crop to the central top quarter.
6. Upscale to 2560 × 2560 via nearest-neighbor.
7. Write result to `out_path_d1`.

### `image_process_to_ocr`

Full pipeline that produces both the D1 image and a set of D2 EXP-segment images:

- **D1 branch**: same steps as the short pipeline, then an additional crop to the
  horizontal name band (`y = 810`, height = 320).
- **D2 branch**: crops the lower-center 1600 × 1600 area of the screenshot, upscales
  to 2560 × 2560, strips to the EXP row (`y = 1695`, height = 261), then delegates
  to `exp_cut` which slices the strip into individual segment files.

Returns a `Vec<String>` of paths to the saved D2 segment images.

### `exp_cut`

Sliding-window segmentation of a horizontal EXP strip:

- A 243 px-wide window slides one pixel at a time left-to-right.
- White-pixel count is evaluated at each position.
- Local maxima (count starts falling after rising) are saved as segment images.
- Near-empty segments (≤ 3 % of maximum possible white pixels) are discarded.
- After saving a segment the window jumps 250 px ahead to avoid duplicate frames.

## Dependencies

| Crate | Purpose |
|---|---|
| [`png`](https://crates.io/crates/png) `0.17` | PNG decode / encode |
| [`rayon`](https://crates.io/crates/rayon) `1.10` | Parallel pixel-window iteration |
| [`tokio`](https://crates.io/crates/tokio) `1.46` | Async runtime (I/O utilities) |
| [`flume`](https://crates.io/crates/flume) `0.11` | Multi-producer / multi-consumer channels |

## Usage

Add the crate to your workspace member's `Cargo.toml`:

```toml
[dependencies]
images = { path = "../images" }
```

Then import what you need:

```rust
use images::png::base::ImagePNG;
use images::algorythms::Algorythms;

let img = ImagePNG::read(&"input.png".to_string());
let gray = img.to_gray(|px| px[0]);
gray.write(&"output.png".to_string());
```
