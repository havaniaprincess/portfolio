use std::{fs::File, path::Path};

use csv::{Error, ReaderBuilder, StringRecord};
use tesseract_lib::fs::{cvs_file_adding, cvs_file_exists};

use crate::{config::Config, db_config::{DatabaseConfig, Fish}};

pub mod db_config;
pub mod fs;
pub mod config;

/// Reads a semicolon-delimited CSV file at `path`, feeds every record into the
/// in-memory [`DatabaseConfig`] aggregate, and then flushes all changes to disk.
///
/// The application config is loaded from `../data/config.json`; if that file is
/// missing a default [`Config`] is used instead.
///
/// # Arguments
/// * `path` – Path to the source CSV log file.
///
/// # Panics
/// Panics if the CSV file cannot be opened.
pub fn add_test_to_db(path: &Path) {
    // Initialize database storage using the default data directory.
    let mut database = DatabaseConfig::new(&"../database_data/".to_string());
    // Read configuration (fallback to default config if file is missing).
    let config_path = "../data/config.json".to_string();
    let _config: Config = match Config::read(&config_path) {
        Some(d) => d,
        None => Config::new()
    };
    //dbg!(path);
    // Open input CSV file with test records.
    let file = File::open(path).unwrap();
    // Create CSV reader with semicolon delimiter.
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';') 
        .from_reader(file);

    // Parse each row and append fish data into the in-memory database.
    for result in rdr.records() {
        Fish::add_to_db(&result.unwrap(), &mut database, false);
    }

    // Persist all collected changes to disk.
    database.save_db();

}

/// Rebuilds the statistical models for every species listed in the config by
/// replaying each species' CSV catch log from disk, then persists the results.
///
/// The application config is loaded from `../data/config.json`; if that file is
/// missing a default [`Config`] is used instead.
pub fn recalculate_stats() {
    let mut database = DatabaseConfig::new(&"../database_data/".to_string());
    let config_path = "../data/config.json".to_string();

    let config: Config = match Config::read(&config_path) {
        Some(d) => d,
        None => Config::new()
    };

    config.fishes.iter()
        .for_each(|(fish_name, _fish)| {
            database.recalculate_fish(fish_name);
        });
    database.save_db(); 
}

/// Re-computes all EXP-related columns in a semicolon-delimited CSV log file
/// and writes the corrected data back to the same file.
///
/// # Processing steps
/// 1. Open and read all records from `path`.
/// 2. Delete the original file.
/// 3. For each record, recalculate `exp_real`, `exp_l`, `exp_happy`, `exp_prem`,
///    `exp_drink`, and `exp_sum` using multipliers from the device preset and rig
///    configuration.
/// 4. Write the updated rows back, creating the file anew with the project header.
///
/// If the file cannot be opened, a warning is printed and the function returns
/// without modifying anything.
///
/// The application config is loaded from `../data/config.json`; if that file is
/// missing a default [`Config`] is used instead.
///
/// # Arguments
/// * `path` – Path to the CSV log file to recalculate in place.
pub fn recalculate_csv(path: &Path){
    // Try to open the source CSV file.
    let file = File::open(path);
    // If the file cannot be opened, print the error and stop processing.
    if let Err(err) = file {
        println!("Error with file [{}]: {}", path.to_str().unwrap(), err);
        return;
    }

    // Load runtime configuration (fallback to default config when missing).
    let config_path = "../data/config.json".to_string();

    let config: Config = match Config::read(&config_path) {
        Some(d) => d,
        None => Config::new()
    };

    // Build CSV reader with semicolon as delimiter.
    let file = file.unwrap();
    let mut rdr = ReaderBuilder::new()
        .delimiter(b';') 
        .from_reader(file);

    // Read all records first, so the file can be safely recreated afterward.
    let records: Vec<Result<StringRecord, Error>> = rdr.records().map(|obj| {
        obj 
    }).collect();

    // Remove original file before writing recalculated rows.
    match std::fs::remove_file(path) {
        Ok(_) => println!("File removed."),
        Err(e) => println!("Error removing file [{}]: {}", path.to_str().unwrap(), e),
    }

    // Temporary in-memory collection of transformed records.
    let mut result_record: Vec<Vec<String>> = Vec::new();

    // Header for the normalized output CSV format.
    let result = format!("mark;timestamp;fish_name;mass;long;test;map;point;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;long_mass_mark;long_mass_err;mass_exp_mark;mass_exp_err;device;exp_real\n");

    // Ensure output file exists and contains the required header.
    let _ = cvs_file_exists(&path, &result);

    // Recalculate EXP-related columns for each input row and write back to file.
    for result in records.into_iter() {  
        let fish = match result {
            Ok(record) => record,
            Err(err) => {
                println!("Error reading a record: {}", err);
                continue;
            }
            
        };
        // Resolve multipliers from device preset and rig configuration.
        let rig_mul = config.rigs.get(&config.device_presets.get(&fish[18]).unwrap().rig).unwrap().exp_mul;
        let light_mul = config.device_presets.get(&fish[18]).unwrap().light_mul;

        // Base real EXP value adjusted by rig multiplier.
        let exp_real = (fish[19].parse::<u64>().unwrap() as f64) / rig_mul;
        //dbg!((fish_name, mass, long, exp));
        //self.add_row_without_old_data(&fish_name, mass, long, exp);

        // Conditional EXP components based on existing marker columns.
        let exp_l = if fish[9].parse::<u64>().unwrap() > 0 {exp_real * light_mul} else {0.0};
        let exp_happy = if fish[10].parse::<u64>().unwrap() > 0 {exp_real * 2.0} else {0.0};
        let exp_prem = if fish[11].parse::<u64>().unwrap() > 0 {exp_real} else {0.0};
        let exp_drink = if fish[13].parse::<u64>().unwrap() > 0 {fish[19].parse::<u64>().unwrap() as f64 / rig_mul} else {0.0};

        // Final EXP sum.
        let exp_all = exp_real + exp_drink + exp_happy + exp_l + exp_prem;

        // Convert row fields into mutable string vector for column replacement.
        let mut res_str: Vec<String> = fish.into_iter()
            .map(|obj| {
                obj.to_string()
            }).collect();

        // Update recalculated EXP columns (rounded to integers).
        res_str[8] = (exp_real.round() as u64).to_string();
        res_str[9] = (exp_l.round() as u64).to_string();
        res_str[10] = (exp_happy.round() as u64).to_string();
        res_str[11] = (exp_prem.round() as u64).to_string();
        res_str[12] = (exp_all.round() as u64).to_string();
        res_str[13] = (exp_drink.round() as u64).to_string();

        // Serialize updated row and append it to the output CSV file.
        let fish_str = format!("{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n", res_str[0], res_str[1], res_str[2], res_str[3], res_str[4], res_str[5], res_str[6], res_str[7], res_str[8], res_str[9], res_str[10], res_str[11], res_str[12], res_str[13], res_str[14], res_str[15], res_str[16], res_str[17], res_str[18], res_str[19]);
        let _ = cvs_file_adding(&path, &fish_str);

        // Keep a copy in memory (currently not used outside this function).
        result_record.push(res_str);
    }
}

/// Simple addition helper used in unit tests.
///
/// # Arguments
/// * `left`  – First operand.
/// * `right` – Second operand.
///
/// # Returns
/// The sum `left + right`.
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Runs [`recalculate_csv`] followed by [`add_test_to_db`] on a sample file.
    /// Expected to fail (`assert_eq!(2, 4)`) — kept as a manual integration check.
    #[test]
    fn it_works() {
        let path = Path::new("../data/result/spin1_64_78.csv");
        recalculate_csv(path);
        add_test_to_db(path);

        let _result = add(2, 2);
        assert_eq!(2, 4);
    }
    
    /// Triggers a full stats recalculation for all species.
    /// Disabled by default — requires live database files.
    //#[test]
    fn it_recalculate() {
        recalculate_stats();

        let _result = add(2, 2);
        assert_eq!(2, 4);
    }
    
    /// Runs [`recalculate_csv`] on a specific sample file.
    /// Disabled by default — requires the file to exist on disk.
    //#[test]
    fn it_recalculate_csv() {
        let path = Path::new("../data/result/worm_126_79.csv");
        recalculate_csv(path);

        let _result = add(2, 2);
        assert_eq!(2, 4);
    }
}


