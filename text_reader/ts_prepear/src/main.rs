//! Tesseract training-data preparation binary (`ts_prepear`).
//!
//! Takes a folder of raw fishing-game screenshots and produces Tesseract
//! LSTM training assets (`.png`, `.box`, `.gt.txt`) ready for fine-tuning.
//!
//! # Workflow
//! 1. Reads all PNG screenshots from `data/source/<data_dir>/`.
//! 2. Preprocesses each screenshot via [`image_process_to_ocr`] to produce:
//!    - **D1** image: name/mass region (`data/d1/<data_dir>/`).
//!    - **D2** images: EXP strip segments (`data/d2/<data_dir>/`).
//! 3. For every preprocessed image, [`make_img`] renames the file to a
//!    training-friendly name, generates a `.box` annotation file, and saves
//!    the OCR reference text as a `.gt.txt` ground-truth file.

use std::path::Path;

use images::algorythms::image_process_to_ocr;
use tesseract_lib::{commands::{get_text_tesseract, make_box}, fs::{dir_exists, list_files}};
use clap::Parser;

/// CLI arguments for the training-data preparation binary.
///
/// * `data_dir` – name of the dataset sub-directory located under
///   `data/source/`. The same name is reused as the output sub-directory
///   under `data/d1/` and `data/d2/`.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub data_dir: String,
}

/// Entry point for the training-data preparation pipeline.
///
/// Performs the following steps:
///
/// 1. **Clean previous artifacts** — removes the `data/d1/<data_dir>/` and
///    `data/d2/<data_dir>/` directories from any prior run.
/// 2. **Validate directories** — asserts that the source path exists and
///    recreates the D1/D2 output directories.
/// 3. **Preprocess screenshots** — calls [`image_process_to_ocr`] for each
///    PNG in the source folder, producing D1 and D2 preprocessed images.
/// 4. **Generate training assets** — calls [`make_img`] for every D1 image
///    and all corresponding D2 segment images to create `.box` and `.gt.txt`
///    files alongside each renamed PNG.
///
/// # Panics
/// Panics if the source directory does not exist or if output directories
/// cannot be created.
fn main() {
    // Parse CLI arguments.
    let args: Args = Args::parse();

    // Base directories used by preprocessing/training preparation.
    let source_data = "data/source/".to_string();
    let d1_data = "data/d1/".to_string();

    // Clean previous generated artifacts for this dataset.
    let _ = std::fs::remove_dir_all(&(d1_data.to_string() + "/" + args.data_dir.as_str()));
    let d2_data = "data/d2/".to_string();
    let _ = std::fs::remove_dir_all(&(d2_data.to_string() + "/" + args.data_dir.as_str()));

    // Source folder containing the raw PNG screenshots.
    let source_path = source_data.clone() + args.data_dir.as_str();

    // Ensure the source directory exists and create the output directories.
    match dir_exists(&source_path, false) {
        Ok(_) => {},
        Err(err) => {
            let mess = format!("{}: {}", source_path.clone(), err);
            panic!("{}", mess)
        }
    }
    match dir_exists(d1_data.clone() + args.data_dir.as_str(), true) {
        Ok(_) => {},
        Err(err) => {
            let mess = format!("{}: {}", d1_data + args.data_dir.as_str(), err);
            panic!("{}", mess)
        }
    }
    match dir_exists(d2_data.clone() + args.data_dir.as_str(), true) {
        Ok(_) => {},
        Err(err) => {
            let mess = format!("{}: {}", d2_data + args.data_dir.as_str(), err);
            panic!("{}", mess)
        }
    }

    //dbg!(&source_path);

    // Read source screenshots and generate D1 (name/mass) and D2 (EXP) images.
    let files = list_files(&source_path).unwrap();
    //dbg!(&files);
    let d1: Vec<(String, Vec<String>)> = files.iter().map(|file| {
        //println!("{}", file);
        let file_path = source_path.clone() + "/" + file.as_str();
        let d1_path = d1_data.clone() + args.data_dir.as_str() + "/" + file.as_str()+ ".d1.png";
        let d2_path = d2_data.clone() + args.data_dir.as_str() + "/";
        let d2s = image_process_to_ocr(&file_path, &d1_path, &d2_path, &file.replace(".png", ""));
        (d1_path, d2s)
    }).collect();

    // For every D1 image and all associated D2 segment images, generate
    // the training assets (.box annotation + .gt.txt ground-truth text).
    let _d1: Vec<(String, Vec<String>)> = d1.iter().map(|(img, d2_vec)| {
        let main_res = make_img(&args, img, &d1_data, &"rus+rus_hp1".to_string());
        let d2_paths: Vec<String> = d2_vec.iter()
            .map(|im_d2 | {
                make_img(&args, im_d2, &d2_data, &"rus+rus_hp1".to_string())
            }).collect();
        (main_res, d2_paths)
    }).collect();

}

/// Prepares a single preprocessed PNG image as a Tesseract training sample.
///
/// Steps performed:
/// 1. **Rename** — converts dots in the file stem to underscores to produce
///    a Tesseract-compatible file name (e.g. `foo.d1_png` → `foo_d1.gt.png`).
///    The `_gt` suffix is preserved as `.gt` per Tesseract naming conventions.
/// 2. **Generate `.box`** — calls [`make_box`] on the renamed image with
///    the specified language model(s) to produce character bounding-box
///    annotations.
/// 3. **Save `.gt.txt`** — runs [`get_text_tesseract`] on the renamed image
///    and writes the recognised text to a `.gt.txt` ground-truth file.
///
/// # Arguments
/// * `args`      – parsed CLI arguments (provides `data_dir`).
/// * `img`       – full path to the source preprocessed PNG.
/// * `data_path` – base output directory (`data/d1/` or `data/d2/`).
/// * `lang`      – Tesseract language model string (e.g. `"rus+rus_hp1"`).
///
/// # Returns
/// The path to the renamed PNG file as a `String`.
fn make_img(args: &Args, img: &String, data_path: &String, lang: &String) -> String {
    
        let img_no_ext = Path::new(img).file_stem().unwrap().to_str().unwrap().to_string();
        let img_path = data_path.to_string() + args.data_dir.as_str() + "/" + img_no_ext.as_str() + "." + "png";
        let png_path = Path::new(&img_path);
        //let png_path = Path::with_extension(&img_path, "d1.".to_string() + extention);
        let new_img_path = data_path.to_string() + args.data_dir.as_str() + "/" + img_no_ext.replace(".", "_").replace("_gt", ".gt").as_str() + "." + "png";
        let new_png_path = Path::new(&new_img_path);
        //let new_png_path = Path::with_extension(&new_img_path, extention);
        //dbg!((&img_no_ext, &png_path, &new_png_path));
        let _f = std::fs::rename(png_path, new_png_path);
        let img_no_ext = Path::join(Path::new(&(data_path.to_string() + args.data_dir.as_str())), Path::new(Path::new(new_png_path).file_stem().unwrap())) ;

        //dbg!(&img_no_ext);
        make_box(img_no_ext.as_path(), lang);
        let img_no_ext = img_no_ext.as_path().with_extension("gt.txt") ;
        let res = get_text_tesseract(&new_img_path, lang);
        let _ = std::fs::write(img_no_ext, res);
        //dbg!(f);
        new_png_path.to_str().unwrap().to_string()
}