use crate::png::base::ImagePNG;

/// Common image-processing operations implemented for PNG images.
///
/// Each method consumes `self` by value (or produces a new transformed image),
/// applying the requested operation and returning the result as a new instance
/// of the same type.
// Common image-processing operations implemented for PNG images.
pub trait Algorythms {
    /// Converts the image to grayscale using a caller-supplied per-pixel function.
    ///
    /// `func` receives a pixel's color channels as a byte slice and must return
    /// a single `u8` luminance value.
    fn to_gray(&self, func: fn(&[u8]) -> u8) -> Self;
    /// Applies a caller-supplied per-pixel transformation that may change the
    /// number of channels or the color values (e.g. quantization, color remapping).
    ///
    /// `func` receives the raw channel bytes of one pixel and returns the
    /// replacement bytes for that pixel.
    fn to_need(&self, func: fn(&[u8]) -> Vec<u8>) -> Self;
    /// Resizes the image to `w × h` pixels using a nearest-neighbour-style
    /// algorithm controlled by the `a` quality parameter.
    fn resize(&self, w: u32, h: u32, a: usize) -> Self;
    /// Resizes the image to `w × h` pixels using the "bomzh" upscaling strategy
    /// (block-based integer upscale), controlled by the `a` factor parameter.
    fn resize_bomzh(&self, w: u32, h: u32, a: usize) -> Self;
    /// Returns a rectangular sub-image starting at pixel `(x, y)` with
    /// dimensions `w × h`.
    fn crop(&self, x: usize, y: usize, w: usize, h: usize) -> Self;
    /// Returns the total number of white (`255`) pixels in the image.
    /// Used as a simple measure of "signal" content during segmentation.
    fn white_count(&self) -> u64;
}

/// Short OCR preprocessing pipeline for the D1 (fish name / mass) region.
///
/// Steps applied in order:
/// 1. Square center crop based on image height.
/// 2. Color quantization (round each channel to the nearest multiple of 8).
/// 3. Two targeted color-range promotions to white (game UI accent colors).
/// 4. Binary threshold to black/white grayscale.
/// 5. Crop to the central top quarter and upscale to 2560 × 2560 for OCR.
/// 6. Write the result to `out_path_d1`.
///
/// # Arguments
/// * `name`        – Path to the source screenshot PNG.
/// * `out_path_d1` – Destination path for the preprocessed D1 image.
// Short OCR preprocessing pipeline:
// center crop -> color quantization/filtering -> threshold -> resize -> save.
pub fn image_process_to_ocr_short(name: &String, out_path_d1: &String) {
    
        let result = ImagePNG::read(name);
        
        // Build square center crop based on image height.
        let side = result.info.height;

        let result = result.crop((result.info.width as usize) / 2 - (side as usize) / 2, (side as usize) / 2 - (side as usize) / 2, side as usize, side as usize);

        // Color quantization to reduce noise.
        let result = result.to_need(|elements| {
            elements.iter().map(|pix| (pix / 8) * 8).collect()
        });
        
        // Promote first target color range to white.
        let result = result.to_need(|elements| {
            if elements[0] >= 240 && elements[0] <= 246 && elements[1] >= 206 && elements[1] <= 212 && elements[2] >= 46 && elements[2] <= 52 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        // Promote second target color range to white.
        let result = result.to_need(|elements| {
            if elements[0] >= 172 && elements[0] <= 178 && elements[1] >= 188 && elements[1] <= 194 && elements[2] >= 53 && elements[2] <= 60 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        // Convert to binary-like grayscale (white/black).
        let result = result.to_gray(|elements| {
            if elements[0] >= 230 && elements[1] >= 230 && elements[2] >= 230 {
                255
            } else {
                0
            }
        });
        
        // Focus on central top area and upscale for OCR.
        let side = result.info.width;
        let result = result.crop(side as usize/4, 0, side as usize / 2, side as usize / 2);
        let result = result.resize_bomzh(2560, 2560, 5);
        
        // Save preprocessed D1 image.
        let _ = result.write(out_path_d1);
}



/// Full OCR preprocessing pipeline: produces the D1 image and extracts D2 EXP segments.
///
/// # D1 branch (fish name / mass region)
/// Applies the same pipeline as [`image_process_to_ocr_short`], then additionally
/// crops to the horizontal name band (`y=810`, height=320) before saving.
///
/// # D2 branch (EXP blocks region)
/// Crops the lower-center 1600 × 1600 area of the original screenshot, upscales
/// it to 2560 × 2560, and strips it to the EXP strip (`y=1695`, height=261).
/// The strip is then passed to [`exp_cut`] which slices it into individual EXP
/// segment files.
///
/// # Arguments
/// * `name`        – Path to the source screenshot PNG.
/// * `out_path_d1` – Destination directory/prefix for the D1 output image.
/// * `out_path_d2` – Destination directory/prefix for the D2 segment images.
/// * `file_name`   – Base file stem used when naming segment files.
///
/// # Returns
/// A `Vec<String>` of file paths pointing to every saved D2 EXP segment image.
// Full OCR preprocessing pipeline:
// produces D1 image and cuts D2 EXP segments into separate files.
pub fn image_process_to_ocr(name: &String, out_path_d1: &String, out_path_d2: &String, file_name: &String) -> Vec<String> {
    
        let start = ImagePNG::read(name);
        
        // D1 branch: center crop and color cleanup.
        let side = start.info.height;

        let result = start.crop((start.info.width as usize) / 2 - (side as usize) / 2, (side as usize) / 2 - (side as usize) / 2, side as usize, side as usize);

        let result = result.to_need(|elements| {
            elements.iter().map(|pix| (pix / 8) * 8).collect()
        });
        
        let result = result.to_need(|elements| {
            if elements[0] >= 240 && elements[0] <= 246 && elements[1] >= 206 && elements[1] <= 212 && elements[2] >= 46 && elements[2] <= 52 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_need(|elements| {
            if elements[0] >= 172 && elements[0] <= 178 && elements[1] >= 188 && elements[1] <= 194 && elements[2] >= 53 && elements[2] <= 60 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_gray(|elements| {
            if elements[0] >= 230 && elements[1] >= 230 && elements[2] >= 230 {
                255
            } else {
                0
            }
        });
        
        // Keep top-center region and scale it up.
        let side = result.info.width;
        let result = result.crop(side as usize/4, 0, side as usize / 2, side as usize / 2);
        let result = result.resize_bomzh(2560, 2560, 5);
        let result = result.crop(0, 810, 2560, 320);
        
        // Save D1 OCR image.
        let _ = result.write(out_path_d1);

        // D2 branch: crop lower central area where EXP blocks are expected.
        let side = 1600;
        //dbg!((start.info.height, side));
        let result = start.crop((start.info.width as usize) / 2 - (side as usize) / 2, (start.info.height as usize) - (side as usize), side as usize, side as usize);
        //dbg!(&result.bytes.len());
        let _side = result.info.width;
        //let result = result.crop(0, 0, side as usize / 2, side as usize / 2);
        let result = result.resize_bomzh(2560, 2560, 5);
        let result = result.crop(0, 1695, 2560, 261);

        // Segment D2 image into multiple EXP snippets and save them.
        let exp_paths = exp_cut(&result, out_path_d2, file_name);
        let _path = out_path_d2.to_string() + file_name.as_str() + ".d2.png";
        //let _ = result.write(&path);
        exp_paths
}

/// Splits a horizontal EXP strip into candidate segment images using a sliding-window
/// white-pixel count heuristic.
///
/// A 243-pixel-wide window slides one pixel at a time from left to right across
/// `img`. At each position the number of white pixels (via [`color_to_white`]) is
/// compared to the previous position:
/// - While the count is rising, the current window is remembered as `img_mem`.
/// - When the count drops after a rising phase, `img_mem` is considered a local
///   maximum and is saved as a segment — unless its white-pixel count is ≤ 3 % of
///   the maximum possible (near-empty segments are discarded).
/// - After saving, the window skips 250 px ahead to avoid writing near-duplicate
///   frames.
///
/// Saved segment files are named `<out_path_d2><file_name>.<i>.d2.png`.
///
/// # Arguments
/// * `img`         – Preprocessed horizontal EXP strip image.
/// * `out_path_d2` – Destination directory/prefix for segment files.
/// * `file_name`   – Base file stem for segment file names.
///
/// # Returns
/// A `Vec<String>` of paths to every saved segment file, in detection order.
    // Splits a horizontal EXP strip into candidate chunks based on white-pixel dynamics.
fn exp_cut(img: &ImagePNG, out_path_d2: &String, file_name: &String) -> Vec<String> {

    let w = img.info.width as usize;

    let w_row = w - 243 - 1;

    let _dx = 243 as usize / 2;
    let crops = img.crop(0, 0, 243, img.info.height as usize);

    let mut img_mem = crops;
    let test_img = color_to_white(&img_mem);
    let mut last_white = test_img.white_count();
    let mut fl = true;
    let mut i = 0;
    let mut rx = 0;
    let max_wite = 243 * (img.info.height as u64);
    let mut res_paths = Vec::new();

    // Sliding window scan over X axis to find local maxima/minima transitions.
    while rx < w_row {
        let crops = img.crop(rx, 0, 243, img.info.height as usize);
        //let crops = crops.resize_bomzh(243*3, 261*3, 5);

        let test_img = color_to_white(&crops);
        let white_now = test_img.white_count();
        if white_now >= last_white {
            //last_white = white_now;
            img_mem = crops;
            fl = true;
        } 
        //dbg!((last_white, white_now));
        if white_now < last_white && fl {
            //dbg!((rx, w_row, w, img_mem.info.height, img_mem.info.width));
            let to_save = color_to_white_ready(&img_mem);
            //dbg!((to_save.white_count(), to_save.info.height, to_save.info.width, 3 * max_wite/100, max_wite));

            // Ignore near-empty segments.
            if to_save.white_count() <= 3 * max_wite/100 {
                rx += 1;
                continue;
            }

            // Save detected segment and skip ahead to avoid near-duplicates.
            let path = out_path_d2.to_string() + file_name.as_str() + "." + i.to_string().as_str() + ".d2.png";
            let _ = to_save.write(&path);
            res_paths.push(path);
            fl = false;
            i += 1;
            rx += 250;
        }
        rx += 1;
        last_white = white_now;
    };
    res_paths
}

/// Applies game-UI color-range promotions to white, then thresholds to
/// a binary black/white grayscale image ready for saving as a final D2 segment.
///
/// Two specific RGB ranges (the gold accent and the teal accent of the game UI)
/// are remapped to `[255, 255, 255]` before the threshold step.
/// The threshold promotes pixels where all three channels ≥ 200 to white;   
/// all others become black.
///
/// # Arguments
/// * `img` – Source image (typically a 243-wide EXP segment crop).
///
/// # Returns
/// A new single-channel binary image suitable for OCR.
// Converts selected target colors to white and then thresholds to black/white.
// Used before saving final D2 segment images.
fn color_to_white_ready(img: &ImagePNG) -> ImagePNG {
        let result = img.to_need(|elements| {
            if elements[0] >= 238 && elements[0] <= 248 && elements[1] >= 204 && elements[1] <= 214 && elements[2] >= 44 && elements[2] <= 54 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_need(|elements| {
            if elements[0] >= 178 && elements[0] <= 188 && elements[1] >= 194 && elements[1] <= 204 && elements[2] >= 52 && elements[2] <= 62 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_gray(|elements| {
            if elements[0] >= 200 && elements[1] >= 200 && elements[2] >= 200 {
                255
            } else {
                0
            }
        });
        result
}

/// Produces a high-contrast mask used during the sliding-window scan in [`exp_cut`].
///
/// Promotes three color ranges to white:
/// 1. Near-neutral dark gray (background noise in EXP blocks).
/// 2. Gold game-UI accent color.
/// 3. Teal game-UI accent color.
///
/// Then applies a strict threshold (all channels ≥ 250) so that only the
/// promoted pixels are white and everything else is black. The white-pixel
/// count of the result is used to detect local maxima in [`exp_cut`].
///
/// # Arguments
/// * `img` – Sliding-window crop to analyze.
///
/// # Returns
/// A binary image where white pixels indicate detected EXP content.
    // Highlight pass for scan-time detection in `exp_cut`.
    // Produces high-contrast mask for white-pixel counting.
fn color_to_white(img: &ImagePNG) -> ImagePNG {
        let result = img.to_need(|elements| {
            if elements[0] >= 59 && elements[0] <= 69 && elements[1] >= 59 && elements[1] <= 69 && elements[2] >= 59 && elements[2] <= 69 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        let result = result.to_need(|elements| {
            if elements[0] >= 238 && elements[0] <= 248 && elements[1] >= 204 && elements[1] <= 214 && elements[2] >= 44 && elements[2] <= 54 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_need(|elements| {
            if elements[0] >= 178 && elements[0] <= 188 && elements[1] >= 194 && elements[1] <= 204 && elements[2] >= 52 && elements[2] <= 62 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        
        let result = result.to_gray(|elements| {
            if elements[0] >= 250 && elements[1] >= 250 && elements[2] >= 250 {
                255
            } else {
                0
            }
        });
        result
}