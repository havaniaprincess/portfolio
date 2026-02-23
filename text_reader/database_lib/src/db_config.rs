use std::collections::BTreeMap;
use std::fs::File;
use std::collections::HashMap;
use std::io::{Write, BufWriter};
use csv::ReaderBuilder;

use csv::StringRecord;
use tesseract_lib::fs::{cvs_file_adding, cvs_file_exists, dir_exists};

use crate::config::Config;
use crate::fs::json_hashmap_load;

/// Classifies how likely an observed input value is correct compared to historical statistics.
///
/// Each variant (except [`MistakeProb::NotEnoughData`]) carries a `f64` representing
/// the absolute deviation between the observed value and the historical average.
#[derive(Clone, Debug, PartialEq, PartialOrd)]
pub enum MistakeProb{
    /// The value is within 5 % of the historical average — considered accurate.
    FullAccuracy(f64),
    /// The value deviates more than 5 % but may still be valid.
    MaybeMistake(f64),
    /// The value differs significantly from the historical average.
    FullyNotSure(f64),
    /// Not enough historical records exist to make a comparison.
    NotEnoughData,
}

impl MistakeProb {
    /// Converts the variant into a human-readable pair `(label, value_string)`.
    ///
    /// The numeric payload is formatted to two decimal places.
    /// For [`MistakeProb::NotEnoughData`] the value string is empty.
    ///
    /// # Returns
    /// `(status_label, numeric_value_as_string)`
    pub fn get_text(&self) -> (String, String) {
        match self {
            Self::FullAccuracy(d) => {
                let d_s = format!("{:.2}", d);
                ("FullAccuracy".to_string(), d_s)
            },
            Self::MaybeMistake(d) => {
                let d_s = format!("{:.2}", d);
                ("MaybeMistake".to_string(), d_s)
            },
            Self::FullyNotSure(d) => {
                let d_s = format!("{:.2}", d);
                ("FullyNotSure".to_string(), d_s)
            },
            Self::NotEnoughData => {
                let d_s = format!("");
                ("NotEnoughData".to_string(), d_s)
            },
        }
    }
}

/// Represents a single fish catch record with all decomposed EXP components.
///
/// Fields follow the column layout of the project's main CSV log format.
pub struct Fish {
    pub timestamp: String,
    pub name: String,
    /// Catch weight in grams.
    pub mass: u64,
    /// Catch length in millimetres.
    pub long: u64,
    pub exp_base: u64,
    pub exp_light: u64,
    pub exp_happy: u64,
    pub exp_prem: u64,
    pub exp_drink: u64,
    pub exp_sum: u64,

}

impl Fish {
    /// Processes one raw CSV record, updates the in-memory database aggregates,
    /// and appends a normalized row to three per-scope output files:
    /// - `database_data/fishes/<name>.csv`
    /// - `database_data/maps/<map>/<point>.csv`
    /// - `database_data/tests/<test>.csv`
    ///
    /// # Arguments
    /// * `fish`        – Raw CSV record from the source log (column indices are fixed).
    /// * `database`    – Mutable reference to the in-memory database to update.
    /// * `without_old` – When `true`, stats are accumulated without loading previous
    ///                   data from disk first (useful for batch recalculation).
    ///
    /// # Panics
    /// Panics if a required directory cannot be created or if numeric fields
    /// in the record cannot be parsed.
    pub fn add_to_db(fish: &StringRecord, database: &mut DatabaseConfig, without_old: bool) {
        //dbg!(&fish[0]);
        let name = fish[2].to_string();
        let test = fish[5].to_string();
        let map = fish[6].to_string();
        let point = fish[7].to_string();
        let fish_file_path = "../database_data/fishes/".to_string() + name.as_str() + ".csv";
        let map_dir_path = "../database_data/maps/".to_string() + map.as_str() + "/";
        let point_file_path = "../database_data/maps/".to_string() + map.as_str() + "/" + point.as_str() + ".csv";
        let test_file_path = "../database_data/tests/".to_string() + test.as_str() + ".csv";

        let mass = fish[3].parse::<u64>().unwrap();
        let long = fish[4].parse::<u64>().unwrap();
        let exp = fish[8].parse::<u64>().unwrap();

        if without_old {
            database.add_row_without_old_data(&name, mass, long, exp);
        } else {
            database.add_row(&name, mass, long, exp);
        }

        
        // Ensure destination map directory exists.
        match dir_exists(&map_dir_path, true) {
            Ok(_) => {},
            Err(err) => {
                let mess = format!("{}: {}", map_dir_path.clone(), err);
                panic!("{}", mess)
            }
        }

        // Header for normalized output rows.
        let result = format!("name;test;map;point;timestamp;mass;long;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;device;exp_real\n");
        let _ = cvs_file_exists(&fish_file_path, &result);
        let _ = cvs_file_exists(&point_file_path, &result);
        let _ = cvs_file_exists(&test_file_path, &result);
        
        // Build a serialized CSV row and append it to all target files.
        let fish_str = format!("{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n", &name, &test, &map, &point, fish[1].to_string(), fish[3].parse::<u64>().unwrap(), fish[4].parse::<u64>().unwrap(),fish[8].parse::<u64>().unwrap_or(0),fish[9].parse::<u64>().unwrap_or(0),fish[10].parse::<u64>().unwrap_or(0),fish[11].parse::<u64>().unwrap_or(0),fish[12].parse::<u64>().unwrap_or(0),fish[13].parse::<u64>().unwrap_or(0),&fish[18],fish[19].parse::<u64>().unwrap_or(0));
        let _ = cvs_file_adding(&fish_file_path, &fish_str);
        let _ = cvs_file_adding(&point_file_path, &fish_str);
        let _ = cvs_file_adding(&test_file_path, &fish_str);


    }
}

/// Aggregated metric bucket stored as `(sum, count)`.
///
/// The average value for a bucket is `sum / count`.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct StatFunc(pub u64, pub u64);

/// Per-species statistical model built from historical catch records.
///
/// Maintains two sorted maps used for cross-metric validation:
/// - `long_mass`: length (mm) → [`StatFunc`] — predicts mass from length.
/// - `mass_exp`:  mass  (g)  → [`StatFunc`] — predicts EXP from mass.
///
/// Both maps use [`BTreeMap`] so buckets are always iterated in ascending key order,
/// which is required by the linear-interpolation logic.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct FishDatabaseConfig {
    long_mass: BTreeMap<u64, StatFunc>,
    mass_exp: BTreeMap<u64, StatFunc>,
}

impl FishDatabaseConfig {
    /// Incorporates one new observation `(mass, long, exp)` into both aggregate maps.
    ///
    /// If a bucket for the given key already exists its sum and count are incremented;
    /// otherwise a new bucket is created with `sum = value` and `count = 1`.
    ///
    /// # Arguments
    /// * `mass` – Catch weight in grams.
    /// * `long` – Catch length in millimetres.
    /// * `exp`  – Experience points awarded for the catch.
    pub fn add_row(&mut self, mass: u64, long: u64, exp: u64) {
        match self.long_mass.get_mut(&long) {
            Some(ms) => {
                ms.0 += mass;
                ms.1 += 1;
            },
            None => {
                self.long_mass.insert(long, StatFunc(mass, 1));
            }
        }
        match self.mass_exp.get_mut(&mass) {
            Some(es) => {
                es.0 += exp;
                es.1 += 1;
            },
            None => {
                self.mass_exp.insert(mass, StatFunc(exp, 1));
            }
        }
    }

    /// Serializes both aggregate tables to JSON files on disk.
    ///
    /// Files are written to:
    /// - `<db_root>/stats/fishes/<name>/long_mass.json`
    /// - `<db_root>/stats/fishes/<name>/mass_exp.json`
    ///
    /// The target directory is created automatically if it does not exist.
    ///
    /// # Arguments
    /// * `name` – Species name used as the directory name under `stats/fishes/`.
    ///
    /// # Panics
    /// Panics if the directory cannot be created or if file I/O fails.
    pub fn save_db(&self, name: &String) {
        let fish_stat_path = "../database_data/stats/fishes/".to_string() + name.as_str() + "/";
        match dir_exists(&fish_stat_path, true) {
            Ok(_) => {},
            Err(err) => {
                let mess = format!("{}: {}", fish_stat_path.clone(), err);
                panic!("{}", mess)
            }
        }
        // Save long->mass aggregate table.
        let long_mass_path = fish_stat_path.to_string() + "long_mass" + ".json";
        
        let pretty_json = serde_json::to_string_pretty(&self.long_mass).unwrap();

        
        let data_file = File::create(&long_mass_path).unwrap();
        let mut data_file = BufWriter::new(data_file);
        data_file.write_all(pretty_json.as_bytes()).unwrap();
        data_file.flush().unwrap();
        // Save mass->exp aggregate table.
        let mass_exp_path = fish_stat_path.to_string() + "mass_exp" + ".json";
        
        let pretty_json = serde_json::to_string_pretty(&self.mass_exp).unwrap();

        
        let data_file = File::create(&mass_exp_path).unwrap();
        let mut data_file = BufWriter::new(data_file);
        data_file.write_all(pretty_json.as_bytes()).unwrap();
        data_file.flush().unwrap();

    }

    /// Estimates whether `(mass, exp)` is consistent with historical mass → exp data.
    ///
    /// If the exact `mass` bucket exists the observed `exp` is compared against the
    /// bucket average. Otherwise a linear model `y = a*x + b` is fitted through the
    /// two nearest buckets and the predicted value is used instead.
    ///
    /// Threshold: relative deviation ≤ 5 % → [`MistakeProb::FullAccuracy`];
    /// higher → [`MistakeProb::FullyNotSure`].
    /// [`MistakeProb::NotEnoughData`] is returned when no lower-bound bucket exists.
    ///
    /// # Arguments
    /// * `mass` – Observed catch weight in grams.
    /// * `exp`  – Observed EXP value to validate.
    fn get_error_me_a(&self, mass: u64, exp: u64) -> MistakeProb {
        //dbg!((mass, exp));
        //dbg!(&self.mass_exp);
        let (avg_mass, avg_exp, last_mass, last_avg_exp) = self.mass_exp.iter()
            .fold((0, 0, 0, 0), |acc, (mass_s, exp_s)| {
                if *mass_s <= mass {
                    return (*mass_s, exp_s.0 / exp_s.1, acc.0, acc.1);
                }
                acc
            });
        //dbg!((avg_mass, avg_exp, last_mass, last_avg_exp));
        if avg_mass == mass {
            let diff = exp.abs_diff(avg_exp) as f64;
            if (diff) / (avg_exp as f64) <= 0.05 {
                return MistakeProb::FullAccuracy(diff);
            } else {
                return MistakeProb::FullyNotSure(diff);
            }
        }
        if last_mass == 0 {
            return MistakeProb::NotEnoughData;
        }
        // Linear model:
        // y = a*x + b
        let a = ((avg_exp as f64 - last_avg_exp as f64) as f64) / ((avg_mass as f64 - last_mass as f64) as f64); 
        let b = (last_avg_exp as f64) - a * (last_mass as f64);
        let avg_y = ((mass as f64) * a + b) as u64;
        let diff = (exp as f64 - avg_y as f64).abs() as f64;
        //dbg!((a, b, avg_y, diff));
        if (diff) / (avg_y as f64) <= 0.05 {
            return MistakeProb::FullAccuracy(diff);
        }
        MistakeProb::FullyNotSure(diff)
    }

    /// Estimates whether `(long, mass)` is consistent with historical length → mass data.
    ///
    /// Mirrors the logic of [`get_error_me_a`](Self::get_error_me_a): exact bucket
    /// lookup first, then linear interpolation between the two nearest buckets.
    ///
    /// Threshold: relative deviation ≤ 5 % → [`MistakeProb::FullAccuracy`];
    /// higher → [`MistakeProb::FullyNotSure`].
    /// [`MistakeProb::NotEnoughData`] when no lower-bound bucket exists.
    ///
    /// # Arguments
    /// * `long` – Observed catch length in millimetres.
    /// * `mass` – Observed catch weight in grams to validate.
    fn get_error_lm_a(&self, long: u64, mass: u64) -> MistakeProb {
        //dbg!((long, mass));
        //dbg!(&self.long_mass);
        let (avg_long, avg_mass, last_long, last_avg_mass) = self.long_mass.iter()
            .fold((0, 0, 0, 0), |acc, (long_s, mass_s)| {
                if *long_s <= long {
                    return (*long_s, mass_s.0 / mass_s.1, acc.0, acc.1);
                }
                acc
            });
        //dbg!((avg_long, avg_mass, last_long, last_avg_mass));
        if avg_long == long {
            let diff = mass.abs_diff(avg_mass) as f64;
            if (diff) / (avg_mass as f64) <= 0.05 {
                return MistakeProb::FullAccuracy(diff);
            } else {
                return MistakeProb::FullyNotSure(diff);
            }
        }
        if last_long == 0 {
            return MistakeProb::NotEnoughData;
        }
        // Linear model:
        // y = a*x + b
        let a = ((avg_mass as f64 - last_avg_mass as f64) as f64) / ((avg_long as f64 - last_long as f64) as f64); 
        let b = (last_avg_mass as f64) - a * (last_long as f64);
        let avg_y = ((long as f64) * a + b) as u64;
        let diff = (mass as f64 - avg_y as f64).abs() as f64;
        //dbg!((a, b, avg_y, diff));
        if (diff) / (avg_y as f64) <= 0.05 {
            return MistakeProb::FullAccuracy(diff);
        }
        MistakeProb::FullyNotSure(diff)
    }

    /// Runs both `(long, mass)` and `(mass, exp)` validation checks and returns
    /// their results as a tuple.
    ///
    /// # Arguments
    /// * `_config` – Application config (reserved for future threshold overrides).
    /// * `mass`    – Catch weight in grams.
    /// * `long`    – Catch length in millimetres.
    /// * `exp`     – EXP value to validate.
    ///
    /// # Returns
    /// `(length_mass_check, mass_exp_check)` — independent [`MistakeProb`] values.
    pub fn check_value(&self, _config: &Config, mass: u64, long: u64, exp: u64) -> (MistakeProb, MistakeProb) {
        let long_mass_type_a = self.get_error_lm_a(long, mass);
        let mass_exp_type_a = self.get_error_me_a(mass, exp);
        
        (long_mass_type_a, mass_exp_type_a)
    }
}

/// Top-level database manager that coordinates loading, aggregating, validating,
/// and persisting per-species statistics.
///
/// Fish statistics are loaded lazily from disk the first time they are needed and
/// cached in the `fishes` map for the lifetime of this struct.
pub struct DatabaseConfig {
    /// Root directory of the database (e.g. `"../database_data/"`).
    path: String,
    /// In-memory cache of per-species statistical models.
    fishes: HashMap<String, FishDatabaseConfig>,    
}

impl DatabaseConfig {
    /// Creates a new `DatabaseConfig` pointing at `path` with an empty in-memory cache.
    ///
    /// # Arguments
    /// * `path` – Root path of the database directory. Must end with `'/'`.
    pub fn new(path: &String) -> Self {
        dbg!(path);
        Self { fishes: HashMap::new(), path: path.to_string() }
    }

    /// Lazily loads the statistical model for one species from disk into memory.
    ///
    /// If the species is already cached this is a no-op. Otherwise the two JSON
    /// aggregate files are read and inserted into the cache. Empty [`BTreeMap`]s
    /// are used as defaults if the files do not yet exist.
    ///
    /// The required directory is created automatically if missing.
    ///
    /// # Arguments
    /// * `name` – Species name (used as both directory and file stem).
    ///
    /// # Panics
    /// Panics if the statistics directory cannot be created.
    fn read_stat_fish(&mut self, name: &String) {
        if self.fishes.contains_key(name) {
            return;
        }
        let fish_stat_path = self.path.to_string() + "stats/fishes/" + name.as_str() + "/";
        match dir_exists(&fish_stat_path, true) {
            Ok(_) => {},
            Err(err) => {
                let mess = format!("{}: {}", fish_stat_path.clone(), err);
                panic!("{}", mess)
            }
        }
        
        // Load long->mass stats.
        let metric_path = fish_stat_path.to_string() + "long_mass" + ".json";
        let long_mass = match json_hashmap_load(&metric_path) {
            Some(hm) => hm,
            None => BTreeMap::new()
        };
        // Load mass->exp stats.
        let metric_path = fish_stat_path.to_string() + "mass_exp" + ".json";
        let mass_exp = match json_hashmap_load(&metric_path) {
            Some(hm) => hm,
            None => BTreeMap::new()
        };
        self.fishes.insert(name.to_string(), FishDatabaseConfig { long_mass: long_mass, mass_exp: mass_exp });
    }

    /// Adds one catch observation to the species model, loading historical data
    /// from disk first if it has not been loaded yet.
    ///
    /// # Arguments
    /// * `name` – Species name.
    /// * `mass` – Catch weight in grams.
    /// * `long` – Catch length in millimetres.
    /// * `exp`  – EXP awarded for the catch.
    pub fn add_row(&mut self, name: &String, mass: u64, long: u64, exp: u64) {

        self.read_stat_fish(name);
        let fish = self.fishes.get_mut(name).unwrap();
        fish.add_row(mass, long, exp);

    }

    /// Adds one catch observation without preloading historical data from disk.
    ///
    /// Useful during batch recalculation when all data is fed from CSV files and
    /// an empty in-memory model is intentional. If no model exists for the species
    /// yet, an empty one is created automatically.
    ///
    /// # Arguments
    /// * `name` – Species name.
    /// * `mass` – Catch weight in grams.
    /// * `long` – Catch length in millimetres.
    /// * `exp`  – EXP awarded for the catch.
    pub fn add_row_without_old_data(&mut self, name: &String, mass: u64, long: u64, exp: u64) {

        //self.read_stat_fish(name);
        let fish = self.fishes.get_mut(name);
        let fish = match fish {
            Some(data) => data,
            None => {
                let fish = FishDatabaseConfig {
                    long_mass: BTreeMap::new(),
                    mass_exp: BTreeMap::new()
                };
                self.fishes.insert(name.clone(), fish);
                self.fishes.get_mut(name).unwrap()
            }
        };
        fish.add_row(mass, long, exp);

    }

    /// Persists all currently cached species models to disk.
    ///
    /// Delegates to [`FishDatabaseConfig::save_db`] for each species in the cache.
    pub fn save_db(&self) {
        self.fishes.iter().for_each(|(name, fish)| {
            fish.save_db(name);
        });
    }

    /// Validates whether `(long, mass)` is consistent with historical data for the
    /// given species, loading stats from disk if necessary.
    ///
    /// # Arguments
    /// * `name` – Species name.
    /// * `mass` – Catch weight in grams.
    /// * `long` – Catch length in millimetres.
    ///
    /// # Returns
    /// A [`MistakeProb`] describing the quality of the length–mass relationship.
    pub fn check_value_long_mass(&mut self, name: &String, mass: u64, long: u64) -> MistakeProb {

        self.read_stat_fish(name);
        let fish = self.fishes.get(name).unwrap();
        fish.get_error_lm_a( long, mass)

    }

    /// Validates whether `(mass, exp)` is consistent with historical data for the
    /// given species, loading stats from disk if necessary.
    ///
    /// # Arguments
    /// * `name` – Species name.
    /// * `mass` – Catch weight in grams.
    /// * `exp`  – EXP value to validate.
    ///
    /// # Returns
    /// A [`MistakeProb`] describing the quality of the mass–EXP relationship.
    pub fn check_value_mass_exp(&mut self, name: &String, mass: u64, exp: u64) -> MistakeProb {
        //dbg!(self.fishes.keys());
        self.read_stat_fish(name);
        let fish = self.fishes.get(name).unwrap();
        fish.get_error_me_a( mass, exp)

    }

    /// Rebuilds the in-memory statistical model for one species by replaying its
    /// entire CSV catch log from disk.
    ///
    /// The existing in-memory model is not cleared before replaying, so this method
    /// accumulates on top of whatever is already loaded. If the CSV file does not
    /// exist, a warning is printed and the method returns without modifying the model.
    ///
    /// # Arguments
    /// * `fish_name` – Species name (also the CSV file stem under `fishes/`).
    pub fn recalculate_fish(&mut self, fish_name: &String) {
        
        let path = self.path.to_string() + "fishes/" + fish_name.as_str() + ".csv";
        let file = File::open(path);
        if let Err(err) = file {
            println!("Error with fish {}: {}", fish_name, err);
            return;
        }
        let file = file.unwrap();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';') // Delimiter used in project CSV files
            .from_reader(file);

        // Parse each row and feed aggregated statistics.
        for result in rdr.records() {  
            let fish = result.unwrap();      
            let mass = fish[5].parse::<u64>().unwrap();
            let long = fish[6].parse::<u64>().unwrap();
            let exp = fish[7].parse::<u64>().unwrap();
            //dbg!((fish_name, mass, long, exp));
            self.add_row_without_old_data(&fish_name, mass, long, exp);
        }


    }
}


#[cfg(test)]
mod tests {
    use super::*;

    /// Integration smoke-test for [`DatabaseConfig::check_value_mass_exp`].
    /// Disabled by default (`#[test]` commented out) as it requires live database files.
    //#[test]
    fn test_db() {
        //let path = Path::new("../data/result/hour_1.csv");
        //add_test_to_db(path);

        //let result = add(2, 2);

        let mut db = DatabaseConfig::new(&"../database_data/".to_string());

        db.check_value_mass_exp(&"окунь большеротый".to_string(), 1025, 2984);

        assert_eq!(2, 4);
    }
}
