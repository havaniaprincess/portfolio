//! PNG image processing algorithms implementation.
//! Provides grayscale conversion, generic per-pixel transforms,
//! Lanczos resampling, nearest-neighbor resize, crop, and pixel counting.

use std::u8;

use crate::{algorythms::Algorythms, png::base::{ColorTypePNG, ImagePNG, PixelSize}};
use rayon::prelude::*;

/// Applies a user-supplied single-value transform `func` to every pixel window
/// of size `window_pixel` in `array` using parallel iteration.
///
/// Each window maps to exactly one output pixel. When `alpha` is `true` the
/// last channel of every output pixel is forced to 255 (fully opaque) while
/// all other channels receive the value returned by `func`.
///
/// Returns `Some(Vec<u8>)` with the transformed pixel data, or `None` if the
/// input cannot be processed.
fn graying_array(array: &Vec<u8>, alpha: bool, window_pixel: usize, func: fn(&[u8]) -> u8) -> Option<Vec<u8>> {
    let new_array: Vec<u8> = array
        .par_windows(window_pixel)
        .step_by(window_pixel)
        .map(|element| {
        
            let value = func(element);
            let mut result = vec![value; if alpha {window_pixel - 1} else {window_pixel}];
            if alpha {
                result.push(255);
            }
            result
        })
        .reduce(|| Vec::new(), |acc, right| [acc, right]
        .concat()
    );

    Some(new_array)
}

/// Applies a user-supplied multi-value transform `func` to every pixel window
/// of size `window_pixel` in `array` using parallel iteration.
///
/// Unlike [`graying_array`], `func` may produce an arbitrary-length `Vec<u8>`
/// per window, allowing channel-count changes or palette remapping. Results
/// from all windows are concatenated in order.
///
/// The `_alpha` parameter is reserved for future use and currently ignored.
///
/// Returns `Some(Vec<u8>)` with the flattened transformed data.
fn needing_array(array: &Vec<u8>, _alpha: bool, window_pixel: usize, func: fn(&[u8]) -> Vec<u8>) -> Option<Vec<u8>> {
    let new_array: Vec<u8> = array
        .par_windows(window_pixel)
        .step_by(window_pixel)
        .map(|element| {
        
            let result = func(element);
            result
        })
        .reduce(|| Vec::new(), |acc, right| [acc, right]
        .concat()
    );

    Some(new_array)
}

/// Normalised sinc function: $\text{sinc}(x) = \sin(\pi x) / (\pi x)$.
///
/// Returns `1.0` at `x == 0.0` (the analytically correct limit) and
/// $\sin(\pi x) / (\pi x)$ elsewhere. Used as the basis of the Lanczos
/// resampling kernel.
fn sinc(x: f64) -> f64 {
    if x == 0.0 {
        1.0
    } else {
        let pix = std::f64::consts::PI * x;
        (pix).sin() / pix
    }
}

/// Evaluates the Lanczos resampling kernel at distance `x` with support
/// radius `a`.
///
/// Defined as `sinc(x) * sinc(x / a)` for `|x| < a` and `0` otherwise.
/// Larger values of `a` produce smoother results at the cost of more
/// computation per output pixel.
fn lanczos_kernel(x: f64, a: usize) -> f64 {
    if x.abs() >= a as f64 {
        0.0
    } else {
        sinc(x) * sinc(x / a as f64)
    }
}

impl Algorythms for ImagePNG {
    /// Applies an arbitrary per-pixel transformation `func` to every pixel of
    /// this image and returns the modified image.
    ///
    /// * For **direct-color** images the transform is applied to the raw byte
    ///   buffer directly.
    /// * For **indexed** (palette-based) PNG images the transform is applied
    ///   to the palette entries instead of the pixel index data, so the
    ///   palette colours are remapped while the indices remain unchanged.
    /// * Grayscale and grayscale-alpha images are returned unchanged.
    ///
    /// Panics if the pixel size cannot be determined or the transform fails.
    fn to_need(&self, func: fn(&[u8]) -> Vec<u8>) -> Self {
        let mut result = self.clone();
        if self.info.color_type == ColorTypePNG::Grayscale || self.info.color_type == ColorTypePNG::GrayscaleAlpha {
            return result;
        }
        let window_size = result.get_pixel_size();
        if window_size == None {
            panic!("Error when try detect window size for graying");
        }
        let window_size = window_size.unwrap();
        

        match window_size {
            PixelSize::Direct(w) => {
                let r = needing_array(&result.bytes, result.is_alpha(), w, func);
                result.bytes = match r {
                    Some(ra) => ra,
                    None => panic!("Error on graying")
                };
            },
            PixelSize::Hash(w) => {
                let palette = match result.info.palette {
                    Some(d) => d,
                    None => panic!("There is't palette when Indexed PNG")
                };
                let r = needing_array(&palette, false, w, func);
                result.info.palette = r;
            },
        };

        result
    }

    /// Converts a color image to a single-channel (gray-like) representation
    /// by collapsing every pixel window to one byte using the provided `func`.
    ///
    /// * For **direct-color** images the collapsed values are written back
    ///   to the raw buffer; the alpha channel (if present) is forced to 255.
    /// * For **indexed** PNG images `func` is applied to each palette entry
    ///   so that the palette is converted to grayscale equivalents.
    /// * Already-grayscale images are returned unchanged.
    ///
    /// Panics if the pixel size cannot be determined or the transform fails.
    fn to_gray(&self, func: fn(&[u8]) -> u8) -> Self {
        let mut result = self.clone();
        if self.info.color_type == ColorTypePNG::Grayscale || self.info.color_type == ColorTypePNG::GrayscaleAlpha {
            return result;
        }
        let window_size = result.get_pixel_size();
        if window_size == None {
            panic!("Error when try detect window size for graying");
        }
        let window_size = window_size.unwrap();
        

        match window_size {
            PixelSize::Direct(w) => {
                let r = graying_array(&result.bytes, result.is_alpha(), w, func);
                result.bytes = match r {
                    Some(ra) => ra,
                    None => panic!("Error on graying")
                };
            },
            PixelSize::Hash(w) => {
                let palette = match result.info.palette {
                    Some(d) => d,
                    None => panic!("There is't palette when Indexed PNG")
                };
                let r = graying_array(&palette, false, w, func);
                result.info.palette = r;
            },
        };

        result
    }

    /// Resizes the image to `new_width × new_height` using Lanczos resampling.
    ///
    /// For each output pixel a `2 * window`-wide neighborhood of source pixels
    /// is sampled. The contribution of each source pixel is weighted by the
    /// product of the Lanczos kernel evaluated in the X and Y directions
    /// independently. The final channel value is the weighted average of all
    /// contributing source pixels.
    ///
    /// `window` controls the Lanczos support radius. Typical values are 2
    /// (Lanczos-2) or 3 (Lanczos-3). Larger values are sharper but slower.
    ///
    /// Returns a new `ImagePNG` with the requested dimensions preserving the
    /// original color type, bit depth, and DPI metadata.
    fn resize(&self, new_width: u32, new_height: u32, window: usize) -> Self {
        let (h, w) = (self.info.height as usize, self.info.width as usize);
        let pixel_size = match self.get_pixel_size().unwrap() {
            PixelSize::Direct(d) => d,
            PixelSize::Hash(d) => d,
        };
        let mut output = Vec::new();

        let scale_x = w as f64 / new_width as f64;
        let scale_y = h as f64 / new_height as f64;
        //let mut i = 0;

        // Iterate over every output pixel and reconstruct each channel
        // as the Lanczos-weighted sum of the surrounding source neighborhood.
        for y_out in 0..new_height {
            for x_out in 0..new_width {
                let x_in = x_out as f64 * scale_x;
                let y_in = y_out as f64 * scale_y;

                let x_base = x_in.floor() as isize;
                let y_base = y_in.floor() as isize;
                let mut sum = vec![0.0; pixel_size];
                let mut weight_sum = vec![0.0; pixel_size];

                for dy in -(window as isize - 1)..=(window as isize) {
                    let y = y_base + dy;
                    if y < 0 || y >= h as isize {
                        continue;
                    }

                    let ky = lanczos_kernel(y_in - y as f64, window);
                    for dx in -(window as isize - 1)..=(window as isize) {
                        let x = x_base + dx;
                        if x < 0 || x >= w as isize {
                            continue;
                        }

                        let kx = lanczos_kernel(x_in - x as f64, window);
                        let weight = kx * ky;
                        (0..pixel_size).for_each(|item| {
                            let data_value = self.bytes[((y as usize) * w + (x as usize)) * pixel_size + item];
                            if false {
                                weight_sum[item] += 2.0 * weight;
                                sum[item] += 2.0 * weight * (data_value as f64);
                            } else {
                                weight_sum[item] += weight;
                                sum[item] += weight * (data_value as f64);
                            }
                        });
                    }
                }
                //dbg!((x_out, x_base, y_out, y_base, &sum, &weight_sum));

                // Divide each channel's accumulated weighted sum by the total
                // weight to produce the normalised output value and push it.
                (0..pixel_size).for_each(|item| {
                    output.push((if weight_sum[item] != 0.0 { sum[item] / weight_sum[item] } else { 0.0 }) as u8);
                });
                //i += 1;
            }
        }
        //dbg!(i);
        Self::new(output, new_width, new_height, self.info.clone().color_type, self.info.clone().bit_depth, self.info.dpi)
    }

    /// Fast nearest-neighbor resize to `new_width × new_height`.
    ///
    /// Maps each output pixel directly to the nearest source pixel by flooring
    /// the scaled coordinates. No interpolation is performed, making this
    /// significantly faster than [`resize`] but producing blocky artefacts
    /// when upscaling.
    ///
    /// The `_window` parameter is accepted for API compatibility with [`resize`]
    /// but is not used.
    ///
    /// Returns a new `ImagePNG` with the requested dimensions.
    fn resize_bomzh(&self, new_width: u32, new_height: u32, _window: usize) -> Self {
        let (h, w) = (self.info.height as usize, self.info.width as usize);
        let pixel_size = match self.get_pixel_size().unwrap() {
            PixelSize::Direct(d) => d,
            PixelSize::Hash(d) => d,
        };
        let mut output = Vec::new();

        let scale_x = w as f64 / new_width as f64;
        let scale_y = h as f64 / new_height as f64;
        //dbg!((new_width, new_height, w, h, pixel_size, scale_x, scale_y, self.bytes.len()));
        for y_out in 0..new_height {
            for x_out in 0..new_width {
                let x_in = x_out as f64 * scale_x;
                let y_in = y_out as f64 * scale_y;

                let x_base = x_in.floor() as isize;
                let y_base = y_in.floor() as isize;

                (0..pixel_size).for_each(|item| {
                    let data_value = self.bytes[((y_base as usize) * w + (x_base as usize)) * pixel_size + item];
                    output.push(data_value as u8);
                });
            }
        }
        //dbg!(i);
        Self::new(output, new_width, new_height, self.info.clone().color_type, self.info.clone().bit_depth, self.info.dpi)
    }

    /// Crops a rectangular region from the image starting at pixel `(x, y)`
    /// and spanning `new_width × new_height` pixels.
    ///
    /// Pixels that fall outside the source image boundaries are silently
    /// skipped. The actual dimensions of the returned image are reduced
    /// accordingly (i.e. the result may be smaller than requested).
    ///
    /// Returns a new `ImagePNG` containing only the cropped region, preserving
    /// the original color type, bit depth, and DPI metadata.
    fn crop(&self, x: usize, y: usize, new_width: usize, new_height: usize) -> Self {
        let (h, w) = (self.info.height as usize, self.info.width as usize);
        let pixel_size = match self.get_pixel_size().unwrap() {
            PixelSize::Direct(d) => d,
            PixelSize::Hash(d) => d,
        };
        let mut output = Vec::new();
        let mut x_i = 0;
        let mut y_i = 0;
        //dbg!((y, new_height, x, new_width, pixel_size, self.bytes.len()));
        for y_out in y..(y + new_height) {
            if y_out >= h as usize {
                y_i += 1;
                continue;
            }
            for x_out in x..(x + new_width) {
                if x_out >= w as usize {
                    x_i += 1;
                    continue;
                }

                (0..pixel_size).for_each(|item| {
                    let data_value = self.bytes[((y_out as usize) * w + (x_out as usize)) * pixel_size + item];
                    output.push(data_value);
                });
            }
        }
        Self::new(output, new_width as u32 - x_i, new_height as u32 - y_i, self.info.clone().color_type, self.info.clone().bit_depth, self.info.dpi)
    }

    /// Counts the number of near-white pixels in the image.
    ///
    /// A pixel is considered "near-white" when all three of its first colour
    /// channels (R, G, B) are ≥ 250. The alpha channel (if present) is
    /// ignored.
    ///
    /// Returns the count as `u64` to avoid overflow on very large images.
    fn white_count(&self) -> u64 {
        let pixel_size = match self.get_pixel_size().unwrap() {
            PixelSize::Direct(d) => d,
            PixelSize::Hash(d) => d,
        };
        self.bytes.windows(pixel_size)
            .fold(0, |acc, pix| {
                let mut res = acc;
                if pix[0] >= 250 && pix[1] >= 250 && pix[2] >= 250 {
                    res += 1;
                }
                res
            })
    }
}


#[cfg(test)]
mod tests {
    use super::*;
    //use png::{BitDepth, ColorType, OutputInfo, SourceChromaticities};
    
    #[test]
    fn graying_test() {

        let path = "../data/rf4_4.0.23340_20250704_205001.png".to_string();
        //let path = "../data/indexed.png".to_string();
        let result = ImagePNG::read(&path);
        /* let result = result.to_gray(|elements| {
            elements.iter().fold(u8::MAX, |acc, right| {
                acc.min(*right)
            })
        });  */
        
        /* let result = result.to_gray(|elements| {
            let res = elements.iter().fold(u8::MAX, |acc, right| {
                acc.min(*right)
            });
            if res > 250 {
                res
            } else {
                0
            }
        }); */ 
        /* let result = result.to_gray(|elements| {
            let sum = elements.iter().fold(0 as u16, |acc, right| {
                acc + *right as u16
            });
            (sum / (elements.len() as u16)) as u8
        });  */
        /* let result = result.to_gray(|elements| {
            let sum = elements.iter().fold(0 as u16, |acc, right| {
                acc + *right as u16
            });
            let avg = (sum / (elements.len() as u16)) as u8;

            let sum = elements.iter().fold(0 as u16, |acc, right| {
                acc.max((avg as u16).abs_diff(*right as u16))
            });

            255 - sum as u8
        }); */
        let side = result.info.height;

        let result = result.crop((result.info.width as usize) / 2 - (side as usize) / 2, (side as usize) / 2 - (side as usize) / 2, side as usize, side as usize);

        let path = "../data/rf4_4.0.23340_20250704_205001_crop_gray.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        //let result = result.resize_bomzh(2560, 2560, 5);
        
        let _path = "../data/rf4_4.0.23340_20250704_205001_crop_resize_gray.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        //let _ = result.write(&path);
        let result = result.to_need(|elements| {
            elements.iter().map(|pix| (pix / 8) * 8).collect()
        });
        let path = "../data/rf4_4.0.23340_20250704_205001_bit.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        let result = result.to_need(|elements| {
            if elements[0] >= 240 && elements[0] <= 246 && elements[1] >= 206 && elements[1] <= 212 && elements[2] >= 46 && elements[2] <= 52 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        let path = "../data/rf4_4.0.23340_20250704_205001_bit_1.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        let result = result.to_need(|elements| {
            if elements[0] >= 172 && elements[0] <= 178 && elements[1] >= 188 && elements[1] <= 194 && elements[2] >= 53 && elements[2] <= 60 {
                vec![255; elements.len()]
            } else {
                elements.to_vec()
            }
        });
        let path = "../data/rf4_4.0.23340_20250704_205001_bit_2.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        let result = result.to_gray(|elements| {
            if elements[0] >= 230 && elements[1] >= 230 && elements[2] >= 230 {
                255
            } else {
                0
            }
        });
        let path = "../data/rf4_4.0.23340_20250704_205001_bit_3.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        let side = result.info.width;
        let result = result.crop(side as usize/4, 0, side as usize / 2, side as usize / 2);
        let result = result.resize_bomzh(2560, 2560, 5);
        let path = "../data/rf4_4.0.23340_20250704_205001_q_1.png".to_string();
        //let path = "../data/indexed_out.png".to_string();
        let _ = result.write(&path);
        

        assert_eq!(2 as u8, ((255 as u16 + 253 as u16 + 253 as u16) / (3 as u16)) as u8);
    }
}