//! Tesseract OCR pipeline binary.
//!
//! Reads annotated fishing screenshots, preprocesses them with the `images` crate,
//! runs Tesseract OCR over the resulting images, parses fish names / catch
//! parameters / EXP values, cross-validates results against a local statistics
//! database, and writes one normalised CSV row per screenshot.
//!
//! # Usage
//! ```
//! cargo run --release -p tesseract -- \
//!     --test <test_id> --config ./data/config.json \
//!     --map <map> --point <point> \
//!     [--d1 <preset>] [--d2 <preset>] [--d3 <preset>] [--not-device]
//! ```

mod tesseract;
mod image;
mod name_process;

use clap::Parser;
use control_log::ControlList;
use database_lib::{config::{algorythms::get_config_item, Config, DevicePreset, ExpType}, db_config::{DatabaseConfig, MistakeProb}};
use images::algorythms::image_process_to_ocr;
use chrono::{NaiveDateTime, TimeZone, Utc};

/// CLI arguments for one OCR detection run.
///
/// Passed directly from the command line via `clap`. All path-like fields are
/// plain strings so they can be concatenated with directory prefixes at runtime.
///
/// * `test`       – identifier of the test session (used as a sub-directory name).
/// * `map`        – fishing map name written to the output CSV.
/// * `point`      – fishing point written to the output CSV.
/// * `config`     – path to the JSON runtime config file.
/// * `not_device` – when set, device-slot resolution via the control log is
///                  skipped and device presets are inferred from detected EXP.
/// * `d1`/`d2`/`d3` – optional device preset names for control slots 1–3.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub test: String,
    #[arg(short, long)]
    pub map: String,
    #[arg(short, long)]
    pub point: String,
    #[arg(short, long)]
    pub config: String,
    #[arg(long)]
    pub not_device: bool,
    #[arg(long)]
    pub d1: Option<String>,
    #[arg(long)]
    pub d2: Option<String>,
    #[arg(long)]
    pub d3: Option<String>
}

use regex::Regex;
use std::{collections::{BTreeSet, HashMap}, io::{BufWriter, Write}};
use tesseract_lib::{commands::get_text_tesseract, fs::{dir_exists, list_files}};

use crate::name_process::{get_exp, get_fish_param};


/// Entry point.
///
/// Parses CLI arguments with `clap` and delegates all processing to
/// [`detect_main`]. Returns `Ok(())` on success or propagates any fatal error.
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Parse CLI flags and start detection pipeline.
    let args = Args::parse();
    detect_main(&args);
    Ok(())
}

/// Main detection pipeline.
///
/// Orchestrates the full end-to-end workflow for a batch of screenshots:
///
/// 1. Load the statistics database and runtime config.
/// 2. Build source / output directory paths and ensure they exist.
/// 3. Optionally load the control log to resolve active device slots by timestamp.
/// 4. Preprocess all screenshots to OCR-ready images using
///    [`image_process_to_ocr`] (D1 name/mass region + D2 EXP segments).
/// 5. For each screenshot:
///     a. Parse the capture datetime from the file name and convert to UTC ms.
///     b. Resolve the active device preset from the control log (or infer from EXP).
///     c. Run Tesseract OCR with two language models (`rus_hp1`, `rus`).
///     d. Detect the fish name via [`get_config_item`].
///     e. Detect mass and length via [`get_fish_param`].
///     f. Detect all EXP components via [`get_exp`].
///     g. Append one normalised semicolon-delimited row to the output CSV.
/// 6. Persist any config updates produced during processing.
///
/// # Arguments
/// * `args` – parsed CLI arguments (see [`Args`]).
fn detect_main(args: &Args){
    // Statistics DB used for validation and fallback checks.
    let mut db = DatabaseConfig::new(&"./database_data/".to_string());

    // Load runtime config (fallback to empty if missing).
    let config: Config = match Config::read(&args.config) {
        Some(d) => d,
        None => Config::new()
    };

    // Common project paths.
    let source_data = "data/source/".to_string();
    let blackwhite_data = "data/blackwhite/".to_string();
    let result_data = "data/result/".to_string();

    // Optional control-log mapping (timestamp -> active device slot).
    let controls = if !args.not_device {
        let source_control = source_data.clone() + args.test.as_str() + "_control.csv";
        ControlList::from_path(&source_control)        
    } else {
        ControlList(BTreeSet::new())
    };

    // Source image folder and file-name pattern with timestamp-like suffix.
    let source_path = source_data.clone() + args.test.as_str();
    let re = Regex::new(r"[0-9]{1,}_[0-9]{1,}.png").unwrap();

    // Ensure source and output directories exist.
    match dir_exists(&source_path, false) {
        Ok(_) => {},
        Err(err) => {
            let mess = format!("{}: {}", source_path.clone(), err);
            panic!("{}", mess)
        }
    }
    match dir_exists(blackwhite_data.clone() + "/" + args.test.as_str(), true) {
        Ok(_) => {},
        Err(err) => {
            let mess = format!("{}: {}", blackwhite_data + "/" + args.test.as_str(), err);
            panic!("{}", mess)
        }
    }

    dbg!(&args);

    // Collect input screenshots and prepare output CSV file.
    let files = list_files(&source_path).unwrap();
    let stat_path = result_data.to_string() + args.test.as_str() + ".csv";
    let data_file = std::fs::File::create(&stat_path).unwrap();
    let mut data_file = BufWriter::new(data_file);
    let result = format!("mark;timestamp;fish_name;mass;long;test;map;point;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;long_mass_mark;long_mass_err;mass_exp_mark;mass_exp_err;device;exp_real\n");
    let _ = data_file.write_all(result.as_bytes());
    data_file.flush().unwrap();
    //dbg!(&files);

    // Map control slots (1/2/3) to configured device presets.
    let devices: HashMap<String, &DevicePreset> = if !args.not_device {
        let mut devices: HashMap<String, &DevicePreset> = HashMap::new();
        devices.insert("1".to_string(), config.device_presets.get(&args.d1.clone().unwrap_or("default".to_string())).unwrap());
        devices.insert("2".to_string(), config.device_presets.get(&args.d2.clone().unwrap_or("default".to_string())).unwrap());
        devices.insert("3".to_string(), config.device_presets.get(&args.d3.clone().unwrap_or("default".to_string())).unwrap());
        devices
    } else {
        HashMap::new()
    };
    
    // Preprocess all screenshots to OCR-ready variants (D1 + D2 segments).
    let bws: Vec<(String, Vec<String>)> = files.iter().map(|file| {
        //println!("{}", file);
        let file_path = source_path.clone() + "/" + file.as_str();
        let bw_file_path = blackwhite_data.clone() + args.test.as_str() + "/" + file.as_str()+ ".d1.png";
        let d2_file_path = blackwhite_data.clone() + args.test.as_str() + "/";
        let exp_paths = image_process_to_ocr(&file_path, &bw_file_path, &d2_file_path, &file);
        (bw_file_path, exp_paths)
    }).collect();

    // OCR and parse each preprocessed screenshot.
    bws.iter().for_each(|(file, exp_paths)| {
        //println!("{}", file);

        // Extract capture datetime from file name and convert to unix milliseconds.
        let date = re.find(file).unwrap().as_str().to_string().replace(".png", "");
        let format = "%Y%m%d_%H%M%S";
        let naive = NaiveDateTime::parse_from_str(&date, format)
            .expect("Invalid date format in file name");
        let datetime = Utc.from_utc_datetime(&naive);
        let fish_time: u128 = datetime.timestamp_millis() as u128 - 2*60*60*1000;

        // Resolve active device from control log if device mode is enabled.
        let device = if args.not_device {
            None
        } else {
            Some(devices[&controls.get_last_device_event(fish_time)])
        };

        // OCR passes with different language models for better robustness.
        let res_hp1 = get_text_tesseract(file, "rus_hp1").to_lowercase();
        let res_rus = get_text_tesseract(file, "rus").to_lowercase();
        let ress = vec![res_hp1, res_rus];

        // Detect fish, fish parameters, and EXP components.
        let fish = get_config_item(&ress, &config.tags, &config.fishes);
        let param = get_fish_param(&config, &mut db, &ress, &fish);
        let langs = vec!["rus_hp2".to_string(), "rus".to_string()];
        let exps = get_exp(&config, &mut db, &exp_paths, &langs, &fish, param.0, &device);

        // Extract all expected EXP values with fallback defaults.
        let exp_base = match exps.get(&ExpType::Base) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        //dbg!(&(param.clone(), exp_base.clone()));
        let exp_lbonus = match exps.get(&ExpType::LBonus) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        let exp_happy = match exps.get(&ExpType::HappyBonus) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        let exp_premium = match exps.get(&ExpType::PremBonus) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        let exp_all = match exps.get(&ExpType::AllExp) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        let exp_drink = match exps.get(&ExpType::DrinkBonus) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };
        let exp_real = match exps.get(&ExpType::Real) {
            Some(d) => {
                d.clone()
            },
            None => (0, MistakeProb::NotEnoughData)
        };

        // Device fallback in no-device mode based on detected light bonus.
        let device = if args.not_device {
            if exp_lbonus.0 == 0 {
                config.device_presets.get(&"mah_losinoe_default".to_string()).unwrap()
            } else {
                config.device_presets.get(&"match_losinoe_default".to_string()).unwrap()
            }
        } else {
            device.unwrap()
        };
        let (long_mass_mark, long_mass_err) = param.2.get_text();
        let (mass_exp_mark, mass_exp_err) = exp_base.1.get_text();
        //let (mass_exp_mark, mass_exp_err) = param.3.get_text();
        
        // Append one normalized output row.
        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stat_path)
            .unwrap();
        let mut data_file = BufWriter::new(data_file);
        let result = format!("{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n", if fish == None {"FAIL NAME"} else {""}, &date, &fish.unwrap_or(("DUMMY".to_string(), f64::MAX)).0, param.0, param.1, args.test, args.map, args.point, exp_base.0, exp_lbonus.0, exp_happy.0, exp_premium.0, exp_all.0, exp_drink.0, long_mass_mark, long_mass_err, mass_exp_mark, mass_exp_err, device.id.to_string(), exp_real.0);
        let _ = data_file.write_all(result.as_bytes());
        data_file.flush().unwrap();
        //dbg!(&res);
        //(file.to_string(), fish.0, fish.1, param)
    });
    //dbg!(texts);

    // Persist any config updates produced during processing.
    let _ = config.save(&args.config);
}

// cargo.exe run --release -p tesseract -- --test worm_92_79 --config .\data\config.json --map losinoe --point 92_79 --not-device 2> out_tess
//3h_w_143_113

// cargo.exe run --release -p tesseract -- --test vyp_nav_89_81 --config .\data\config.json --map losinoe --point 89_81 --d1 mah_d1_t1 --d2 mah_d2_t1 --d3 match_d3_t1 2> out_tess

// cargo.exe run --release -p tesseract -- --test 3h_w_51_77 --config .\data\config.json --map losinoe --point 51_77 --d1 mah_d1_t0 --d2 mah_d2_t0 --d3 match_d3_t0 2> out_tess

// cargo.exe run --release -p tesseract -- --test vyp_sver_52_74 --config .\data\config.json --map losinoe --point 52_74 --d1 mah_d1_t2 --d2 mah_d2_t2 --d3 match_d3_t2 2> out_tess

// cargo.exe run --release -p tesseract -- --test spin1_67_79 --config .\data\config.json --map losinoe --point 67_79 --d1 spin_1 --d2 fider_default --d3 match_d3_t0 2> out_tess