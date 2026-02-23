
//! Tesseract LSTM fine-tuning preparation binary (`ts_learning`).
//!
//! Automates the full training-data preparation pipeline:
//!
//! 1. Scans the specified training image directory for PNG files.
//! 2. Generates a binary `.lstmf` sample for every image via Tesseract.
//! 3. Writes a training list file that enumerates all `.lstmf` paths.
//! 4. Extracts a `unicharset` from existing `.box` annotation files.
//! 5. Assembles combined language-model assets (`combine_lang_model`).
//! 6. Launches `lstmtraining` to fine-tune the Russian language model.

use std::{fs::OpenOptions, io::{Write, BufWriter}, path::Path};

use clap::Parser;
use rayon::prelude::*;
use tesseract_lib::commands::{make_box, make_combine, make_lstmf, make_train, make_unichar};

use crate::fs::{check_lstmf, list_files, list_files_name};


mod fs;

/// CLI arguments for the training-data preparation pipeline.
///
/// * `test`       – name of the training session / sub-directory under
///                  `./learning_data/training_data/` that contains the
///                  annotated PNG images and their `.box` files.
/// * `lstmf_file` – path to the output text file that will list every
///                  generated `.lstmf` training sample (one path per line).
///                  An existing file at this path is deleted and recreated.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub test: String,
    //#[arg(long)]
    //pub eval: String,
    #[arg(short, long)]
    pub lstmf_file: String,
    //#[arg(short, long)]
    //pub eval_lstmf_file: String,
}

/// Entry point for the LSTM training-preparation pipeline.
///
/// Orchestrates the following steps in order:
///
/// 1. **Collect images** — lists all `.png` files in
///    `./learning_data/training_data/<test>/`.
/// 2. **Generate `.lstmf` samples** — calls [`make_lstmf`] for each image
///    and appends the resulting path to the training list file specified by
///    `--lstmf-file`. Any existing list file is deleted first.
/// 3. **Extract unicharset** — collects all `.box` files from the same
///    directory and passes them to [`make_unichar`] to produce
///    `hp1.unicharset`.
/// 4. **Combine language model** — calls [`make_combine`] to assemble
///    `.traineddata` assets from the unicharset and language data files.
/// 5. **Run LSTM training** — calls [`make_train`] with the generated
///    training list file to fine-tune the model.
///
/// # Returns
/// `Ok(())` on success. Any subprocess failure panics via `expect`.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI flags.
    let args: Args = Args::parse();

    // Build training directory path.
    let data_path = "./learning_data/".to_string();
    let training_path = data_path.to_string() + "training_data/" + args.test.as_str();
    
    // Collect all PNG training images.
    let training_list = list_files(&training_path, Some("png".to_string())).unwrap();

    // Recreate output lstmf list file.
    let _ = std::fs::remove_file(&args.lstmf_file);
    //dbg!(&training_list);

    // Generate `.lstmf` file for each image and append path to train list file.
    training_list.iter().for_each(|img| {
        let img_path = training_path.to_string() + "/" + img.as_str();
        let img_path = Path::new(&img_path);
        let file_name = make_lstmf(&img_path);
        
        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&args.lstmf_file)
            .unwrap();
        let mut data_file = BufWriter::new(data_file);
        //let result = format!("{};{:?};{};{};{};{:?};{};{}\n");
        let _ = data_file.write_all((file_name + "\n").as_bytes());
    });

    // Build list of `.box` files for unicharset generation.
    let boxes: Vec<String> = list_files(&training_path, Some("box".to_string())).unwrap().into_iter().map(|p| training_path.to_string() + "/" + p.as_str() + ".box").collect();
    //check_lstmf(Path::new("./learning_data/training_data/train_1/rf4_4_0_22487_20250513_220826_png_d1.lstmf"));

    // Training preparation steps:
    // 1) extract unicharset, 2) combine language model data, 3) run lstmtraining.
    dbg!("BOX + LSTMF");
    make_unichar(&boxes);
    dbg!("UNICHAR");
    make_combine();
    dbg!("COMBINE");
    make_train(&args.lstmf_file);
    return  Ok(());
}
