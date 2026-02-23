//! Tesseract CLI wrappers for OCR inference and model training.
//!
//! All functions in this module are thin wrappers around the Tesseract-OCR
//! and LSTM-training executables installed at
//! `C:/Program Files/Tesseract-OCR/`. Custom tessdata lives under
//! `./learning_data/start_data`.

use std::path::Path;

use std::process::Command;



/// Runs Tesseract OCR on a single image and returns the recognised text.
///
/// Invokes the Tesseract CLI with page-segmentation mode 3 (fully automatic).
/// Output is written to a temporary `output.txt` file which is read and then
/// deleted. Custom tessdata is loaded from `./learning_data/start_data`.
///
/// # Arguments
/// * `path` – path to the input PNG image.
/// * `lang` – Tesseract language model name (e.g. `"rus"`, `"rus_hp1"`).
///
/// # Returns
/// The full recognised text as a `String` (may contain newlines).
///
/// # Panics
/// Panics if the Tesseract process cannot be launched or if `output.txt`
/// cannot be read after OCR completes.
pub fn get_text_tesseract(path: &String, lang: &str) -> String {
    let status = Command::new("C:/Program Files/Tesseract-OCR/tesseract.exe")
        .arg(path)     // input file
        .arg("output")  
        .arg("--tessdata-dir")
        .arg("./learning_data/start_data")     // output file name (without extension)
        .arg("-l")
        .arg(lang)          // language
        .arg("--psm")
        .arg("3") 
        .status()
        .expect("Error launching Tesseract");

    dbg!(status.success());
    let contents = std::fs::read_to_string("output.txt").unwrap();

    let _ = std::fs::remove_file("output.txt");
    contents
}

/// Creates an LSTM `.box` file for a training image.
///
/// Invokes Tesseract in `lstmbox` mode to generate character bounding-box
/// annotations from the PNG located at `path` (with `.png` extension).
/// Generation is skipped when the `.box` file already exists, making repeated
/// calls idempotent.
///
/// # Arguments
/// * `path` – base path (stem) of the training image; extensions `.png` and
///            `.box` are derived from it automatically.
/// * `lang` – Tesseract language model used during box generation.
///
/// # Returns
/// The absolute-or-relative path to the `.box` file as a `String`.
///
/// # Panics
/// Panics if the Tesseract process cannot be launched.
pub fn make_box(path: &Path, lang: &str) -> String {
    let box_path = Path::with_extension(&path, "box");
    let png_path = Path::with_extension(&path, "png");
    if !box_path.exists() {
        //tesseract image output --psm X box.train
        Command::new("C:/Program Files/Tesseract-OCR/tesseract.exe")
            .arg(png_path)     // input file
            .arg(path)  
            .arg("--tessdata-dir")
            .arg("./learning_data/start_data") 
            .arg("-l")  
            .arg(lang)      // output file name (without extension)
            .arg("--psm")
            .arg("3")  
            .arg("lstmbox")          // language
            .status()
            .expect("Error launching Tesseract");
    }
    box_path.to_str().unwrap().to_string()
}

/// Generates an `.lstmf` training-sample file from a PNG input image.
///
/// Invokes Tesseract in `lstm.train` mode using the Russian language model
/// and 300 DPI hint. The resulting `.lstmf` binary is placed alongside the
/// source image and is consumed by [`make_train`] during LSTM fine-tuning.
///
/// # Arguments
/// * `path` – base path (stem) of the training image; `.png` and `.lstmf`
///            extensions are derived automatically.
///
/// # Returns
/// The path to the generated `.lstmf` file as a `String`.
///
/// # Panics
/// Panics if the Tesseract process cannot be launched.
pub fn make_lstmf(path: &Path) -> String {
    let box_path = Path::with_extension(&path, "lstmf");
    let png_path = Path::with_extension(&path, "png");
        Command::new("C:/Program Files/Tesseract-OCR/tesseract.exe")
            .arg(png_path)     
            .arg(path)  
            .arg("--tessdata-dir")
            .arg("./learning_data/start_data")
            .arg("-l")  
            .arg("rus")   
            .arg("--dpi")  
            .arg("300")    
            .arg("--psm")
            .arg("3")  
            .arg("lstm.train")          
            .status()
            .expect("Error launching Tesseract");
    // & 'C:/Program Files/Tesseract-OCR/tesseract.exe' --tessdata-dir ./learning_data/start_data ./learning_data/training_data/train_1/rf4_4_0_22487_20250513_220826_png_d1.png ./learning_data/training_data/train_1/rf4_4_0_22487_20250513_220826_png_d1 --print-parameters -l rus --psm 6 lstm.train
    box_path.to_str().unwrap().to_string()
}

// Reference command used to produce the above .lstmf files (PowerShell):
// & 'C:/Program Files/Tesseract-OCR/tesseract.exe' --tessdata-dir ./learning_data/start_data \
//   <image>.png <stem> --print-parameters -l rus --psm 6 lstm.train

// lstmtraining reference command:
// lstmtraining --model_output output/ --continue_from model/rus.lstm \
//   --old_traineddata /usr/local/share/tessdata/rus.traineddata \
//   --traineddata train/rus/rus.traineddata \
//   --train_listfile train/rus.training_files.txt \
//   --eval_listfile eval/rus.training_files.txt \
//   --U train/my.unicharset --max_iterations 140000

/// Fine-tunes the Russian LSTM model using a prepared list of `.lstmf` samples.
///
/// Continues training from the existing `rus.lstm` checkpoint, writing
/// intermediate and final model checkpoints to `./learning_data/out_model_2/`.
/// Training stops after at most 100 000 iterations.
///
/// # Arguments
/// * `lstmf_file` – path to a text file listing one `.lstmf` training sample
///                  per line (produced by [`make_lstmf`]).
///
/// # Panics
/// Panics if the `lstmtraining` process cannot be launched.
pub fn make_train(lstmf_file: &String/* , eval_lstmf_file: &String */) {
        Command::new("C:/Program Files/Tesseract-OCR/lstmtraining.exe")
            .arg("--model_output")    
            .arg("./learning_data/out_model_2/")   
            .arg("--continue_from")  
            .arg("./learning_data/start_data/rus.lstm")      
            .arg("--old_traineddata")
            .arg("./learning_data/start_data/rus.traineddata")
            .arg("--traineddata")
            .arg("./learning_data/start_data/rus/rus.traineddata")  
            .arg("--train_listfile")  
            .arg(lstmf_file)  
            //.arg("--eval_listfile")  
            //.arg(eval_lstmf_file)
            .arg("--max_iterations")
            .arg("100000")       
            .status()
            .expect("Error launching Tesseract");
}

/// Alternative LSTM training configuration (legacy / experimental).
///
/// Identical in purpose to [`make_train`] but uses a fixed training list file
/// (`train_1.txt`) and writes output to `./learning_data/out_model/rus_model`.
/// Kept for reference and comparison against the primary training run.
///
/// # Panics
/// Panics if the `lstmtraining` process cannot be launched.
pub fn make_train_2() {
        Command::new("C:/Program Files/Tesseract-OCR/lstmtraining.exe")
            .arg("--model_output")     // input file
            .arg("./learning_data/out_model/rus_model")   
            .arg("--continue_from")  
            .arg("./learning_data/start_data/rus.lstm")      // output file name (without extension)
            .arg("--traineddata")
            .arg("./learning_data/start_data/rus.traineddata")  
            .arg("--train_listfile")  
            .arg("./learning_data/start_data/train_1.txt")       // language
            .status()
            .expect("Error launching Tesseract");
}

// Reference PowerShell command for combine_lang_model:
// & 'C:/Program Files/Tesseract-OCR/combine_lang_model.exe' \
//   --input_unicharset ./learning_data/start_data/hp1.unicharset \
//   --script_dir ./learning_data/start_data/langdata \
//   --words ./learning_data/start_data/langdata/rus.wordlist \
//   --numbers ./learning_data/start_data/langdata/rus.numbers \
//   --puncs ./learning_data/start_data/langdata/rus.punc \
//   --output_dir ./learning_data/start_data --lang rus

/// Builds combined language-model assets from prepared language data files.
///
/// Invokes `combine_lang_model.exe` to assemble a `.traineddata` file from
/// a unicharset, wordlist, numbers list, and punctuation list. Output is
/// written to `./learning_data/start_data` under the `rus` language prefix.
///
/// Run this step after [`make_unichar`] and before [`make_train`].
///
/// # Panics
/// Panics if the `combine_lang_model` process cannot be launched.
pub fn make_combine() {
        Command::new("C:/Program Files/Tesseract-OCR/combine_lang_model.exe")
            .arg("--input_unicharset")     // input file
            .arg("./learning_data/start_data/hp1.unicharset")   
            .arg("--script_dir")  
            .arg("./learning_data/start_data/langdata")
            .arg("--numbers")
            .arg("./learning_data/start_data/langdata/rus.numbers")  
            .arg("--puncs")  
            .arg("./learning_data/start_data/langdata/rus.punc")
            .arg("--output_dir")  
            .arg("./learning_data/start_data")
            .arg("--lang")  
            .arg("rus")       // language
            .status()
            .expect("Error launching Tesseract");
}

// Reference PowerShell command for unicharset_extractor:
// & 'C:\Program Files\Tesseract-OCR\unicharset_extractor.exe' \
//   --output_unicharset .\learning_data\start_data/hp1.unicharset --norm_mode 2

/// Extracts a unicharset from a collection of `.box` files.
///
/// Invokes `unicharset_extractor.exe` with normalisation mode 2 over all
/// paths supplied in `path`. The resulting `hp1.unicharset` is written to
/// `./learning_data/start_data/` and is consumed by [`make_combine`] in the
/// next training-preparation step.
///
/// # Arguments
/// * `path` – list of paths to `.box` files produced by [`make_box`].
///
/// # Panics
/// Panics if the `unicharset_extractor` process cannot be launched.
pub fn make_unichar(path: &Vec<String>) {
    let mut command = Command::new("C:/Program Files/Tesseract-OCR/unicharset_extractor.exe");
    command.arg("--output_unicharset") 
            .arg("./learning_data/start_data/hp1.unicharset")   
            .arg("--norm_mode")  
            .arg("2");

    for p in path.iter() {
        command.arg(p);
    }

    // Append each box-file path as a positional argument, then execute.
    command.status()
            .expect("Error launching unicharset_extractor");
}