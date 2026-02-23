
//use std::fmt::Error;

use png::{BitDepth, ColorType, OutputInfo, PixelDimensions, SourceChromaticities};
//use tokio::{fs::File};
use std::{borrow::Cow, io::BufWriter};

// Describes how many bytes a pixel occupies:
// fixed direct size or palette/indexed mode marker.
#[derive(Clone, PartialEq, Debug)]
pub enum PixelSize {
    Direct(usize),
    Hash(usize)
}

// Internal representation of PNG color types.
#[derive(Clone, PartialEq, Debug)]
#[repr(u8)]
pub enum ColorTypePNG {
    Grayscale = 0,
    Rgb = 2,
    Indexed = 3,
    GrayscaleAlpha = 4,
    Rgba = 6,
}

impl ColorTypePNG {
    // Converts png crate color type into local enum.
    pub fn from_png(png_type: &ColorType) -> Self {
        match png_type {
            ColorType::Grayscale => Self::Grayscale,
            ColorType::Rgb => Self::Rgb,
            ColorType::Indexed => Self::Indexed,
            ColorType::GrayscaleAlpha => Self::GrayscaleAlpha,
            ColorType::Rgba => Self::Rgba,
            //_ => Self::Rgba,
        }
    }

    // Converts local enum into png crate color type.
    pub fn to_png(&self) -> ColorType {
        match self {
            Self::Grayscale => ColorType::Grayscale,
            Self::Rgb => ColorType::Rgb,
            Self::Indexed => ColorType::Indexed,
            Self::GrayscaleAlpha => ColorType::GrayscaleAlpha,
            Self::Rgba => ColorType::Rgba,
            //_ => ColorType::Rgba,
        }

    }

    // Returns channel count (or equivalent payload size) per pixel.
    pub fn get_pixel_size(&self) -> Option<u32> {
        match self {
            ColorTypePNG::Grayscale => Some(1),
            ColorTypePNG::GrayscaleAlpha => Some(2),
            ColorTypePNG::Rgb => Some(3),
            ColorTypePNG::Rgba => Some(4),
            ColorTypePNG::Indexed => Some(3)
        }
    }
}

// Internal representation of PNG bit depth.
#[derive(Clone, Debug)]
#[repr(u8)]
pub enum BitDepthPNG {
    One = 1,
    Two = 2,
    Four = 4,
    Eight = 8,
    Sixteen = 16,
}

impl BitDepthPNG {
    // Converts png crate bit depth into local enum.
    pub fn from_png(png_type: &BitDepth) -> Self {
        match png_type {
            BitDepth::One => Self::One,
            BitDepth::Two => Self::Two,
            BitDepth::Four => Self::Four,
            BitDepth::Eight => Self::Eight,
            BitDepth::Sixteen => Self::Sixteen,
            //_ => Self::Eight,
        }
    }

    // Converts local enum into png crate bit depth.
    pub fn to_png(&self) -> BitDepth {
        match self {
            Self::One => BitDepth::One,
            Self::Two => BitDepth::Two,
            Self::Four => BitDepth::Four,
            Self::Eight => BitDepth::Eight,
            Self::Sixteen => BitDepth::Sixteen,
            //_ => BitDepth::Eight,
        }
    }
}

// Serializable container for source chromaticity coordinates.
#[derive(Clone, Debug)]
pub struct ChromaticPNG {
    pub white:  (f64, f64),
    pub red:  (f64, f64),
    pub green:  (f64, f64),
    pub blue:  (f64, f64),
}

impl ChromaticPNG {
    // Converts png crate chromaticity metadata into local struct.
    pub fn from_png(&png: &SourceChromaticities) -> Self {
        Self { 
            white: (png.white.0.into_value() as f64, png.white.1.into_value() as f64), 
            red: (png.red.0.into_value() as f64, png.red.1.into_value() as f64), 
            green: (png.green.0.into_value() as f64, png.green.1.into_value() as f64), 
            blue: (png.blue.0.into_value() as f64, png.blue.1.into_value() as f64) 
        }
    }

    // Converts local chromaticity metadata back to png crate type.
    pub fn to_png(&self) -> SourceChromaticities {
        png::SourceChromaticities::new(     // Using unscaled instantiation here
            (self.white.0 as f32, self.white.1 as f32),
            (self.red.0 as f32, self.red.1 as f32),
            (self.green.0 as f32, self.green.1 as f32),
            (self.blue.0 as f32, self.blue.1 as f32)
        )
    }
}

// Image metadata used by this module.
#[derive(Clone, Debug)]
pub struct InfoPNG {
    pub width: u32,
    pub height: u32,
    pub color_type: ColorTypePNG,
    pub bit_depth: BitDepthPNG,
    pub line_size: usize, 
    pub gamma: f64,
    pub chromatic: ChromaticPNG,
    pub palette: Option<Vec<u8>>,
    pub dpi: Option<PixelDimensions>
}

impl InfoPNG {
    // Builds local metadata object from decoded PNG frame info
    // plus extra chunks (gamma/chromaticity/palette/dpi).
    pub fn from_png(png_info: &OutputInfo, gamma: f64, chromatic: ChromaticPNG, palette: Option<Vec<u8>>, dpi: Option<PixelDimensions>) -> Self {
        Self {
            width: png_info.width,
            height: png_info.height,
            color_type: ColorTypePNG::from_png(&png_info.color_type),
            bit_depth: BitDepthPNG::from_png(&png_info.bit_depth),
            line_size: png_info.line_size,
            gamma: gamma,
            chromatic: chromatic,
            palette: palette,
            dpi: dpi
        }
    }
}

// In-memory PNG image: raw bytes + decoded metadata.
#[derive(Clone, Debug)]
pub struct ImagePNG {
    pub bytes: Vec<u8>,
    pub info: InfoPNG,
}

impl ImagePNG {
    // Creates a new image object from raw bytes and basic dimensions.
    // Missing optional metadata is filled with project defaults.
    pub fn new(bytes: Vec<u8>, width: u32, height: u32, color_type: ColorTypePNG, bit_depth: BitDepthPNG, dpi: Option<PixelDimensions>) -> Self {
        let pixel_size = color_type.get_pixel_size().unwrap();
        let chromo = None.unwrap_or(png::SourceChromaticities::new(
            (0.31270, 0.32900),
            (0.64000, 0.33000),
            (0.30000, 0.60000),
            (0.15000, 0.06000)
        ));
        let info = InfoPNG {
            width: width,
            height: height,
            color_type: color_type,
            bit_depth: bit_depth,
            line_size: (width as usize) * (pixel_size as usize),
            gamma: None.unwrap_or(png::ScaledFloat::new(1.0 / 2.2)).into_value() as f64,
            chromatic: ChromaticPNG::from_png(&chromo),
            palette: None,
            dpi: if dpi.is_none() { Some(PixelDimensions{
                xppu: 300,
                yppu: 300,
                unit: png::Unit::Unspecified
            }) } else { dpi },
        };
        Self { 
            bytes: bytes.to_vec(), 
            info: info,
        }
    }

    // Reads PNG from disk and decodes image bytes + metadata chunks.
    pub fn read(path: &String) -> Self {
        let decoder = png::Decoder::new(std::fs::File::open(path).unwrap());
        let mut reader = decoder.read_info().unwrap();
        let mut buf = vec![0; reader.output_buffer_size()];
        let info = reader.next_frame(&mut buf).unwrap();
        let bytes = &buf[..info.buffer_size()];
        //let in_animation = reader.info().frame_control.is_some();

        // Normalize palette from borrowed/owned cow into owned Vec<u8>.
        let palette = match reader.info().palette.clone() {
            Some(data) => {
                let arr = match data {
                    Cow::Owned(d) => d,
                    Cow::Borrowed(d) => d.to_vec()
                };
                Some(arr)
            },
            None => None
        };
        let dpi = reader.info().pixel_dims.clone();
        //dbg!(&palette);

        // Use file chromaticities if present, otherwise standard fallback.
        let chromo = reader.info().source_chromaticities.unwrap_or(png::SourceChromaticities::new(
            (0.31270, 0.32900),
            (0.64000, 0.33000),
            (0.30000, 0.60000),
            (0.15000, 0.06000)
        ));
        Self { 
            bytes: bytes.to_vec(), 
            info: InfoPNG::from_png(
                &info, 
                reader.info().gama_chunk.unwrap_or(png::ScaledFloat::new(1.0 / 2.2)).into_value() as f64,
                ChromaticPNG::from_png(&chromo),
                palette,
                dpi
            )
        }
    }

    // Writes the current image to PNG with stored metadata.
    pub fn write(&self, path: &String) -> Result<(), String> {
        let file = std::fs::File::create(path).unwrap();
        let ref mut w = BufWriter::new(file);

        let mut encoder = png::Encoder::new(w, self.info.width, self.info.height); // Width is 2 pixels and height is 1.
        encoder.set_color(self.info.color_type.to_png());
        encoder.set_depth(self.info.bit_depth.to_png());
        //encoder.set_source_gamma(png::ScaledFloat::from_scaled(45455)); // 1.0 / 2.2, scaled by 100000
        encoder.set_source_gamma(png::ScaledFloat::new(self.info.gamma as f32));     // 1.0 / 2.2, unscaled, but rounded
        let source_chromaticities = self.info.chromatic.to_png();
        encoder.set_source_chromaticities(source_chromaticities);
        encoder.set_pixel_dims(self.info.dpi);
        
        // Attach palette for indexed images when available.
        match self.info.palette.clone() {
            Some(d) => {encoder.set_palette(d);},
            None => {}
        };
        
        let mut writer = encoder.write_header().unwrap();
        let data = self.bytes.as_slice(); // An array containing a RGBA sequence. First pixel is red and second pixel is black.
        //dbg!(data.len());
        //dbg!(&self.info);
        //dbg!(&data);
        writer.write_image_data(data).unwrap();
        Ok(())
    }

    // Returns pixel payload mode/size for current color type.
    pub fn get_pixel_size(&self) -> Option<PixelSize> {
        match self.info.color_type {
            ColorTypePNG::Grayscale => Some(PixelSize::Direct(1)),
            ColorTypePNG::GrayscaleAlpha => Some(PixelSize::Direct(2)),
            ColorTypePNG::Rgb => Some(PixelSize::Direct(3)),
            ColorTypePNG::Rgba => Some(PixelSize::Direct(4)),
            ColorTypePNG::Indexed => Some(PixelSize::Hash(3))
        }
    }

    // Indicates whether image color type includes alpha channel.
    pub fn is_alpha(&self) -> bool {
        match self.info.color_type {
            ColorTypePNG::GrayscaleAlpha => true,
            ColorTypePNG::Rgba => true,
            _ => false
        }
    }
}



#[cfg(test)]
mod tests {
    use super::*;
    //use png::{BitDepth, ColorType, OutputInfo, SourceChromaticities};
    
    #[test]
    fn image_test() {

        let _path = "../data/rf4_4.0.22487_20250513_115814.png".to_string();
        let path = "../data/indexed.png".to_string();
        let result = ImagePNG::read(&path);
        dbg!(result.info.clone());
        let _path = "../data/rf4_4.0.22487_20250513_115814_out.png".to_string();
        let _path = "../data/indexed_out.png".to_string();
        //let _ = result.write(&path);
        assert_eq!(2 as u8, ((255 as u16 + 253 as u16 + 253 as u16) / (3 as u16)) as u8);
    }
}
