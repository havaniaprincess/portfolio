
use std::collections::BTreeMap;
use std::fs::{self, File, FileType};
use std::{collections::HashMap, path::Path};
use std::{fs::OpenOptions, io::{Write, BufWriter}};
use csv::{Reader, ReaderBuilder};
use chrono::{NaiveDateTime, TimeZone, Utc};
use tesseract_lib::fs::{cvs_file_adding, cvs_file_exists};

/// Thin wrapper over the database root path used as the entry point for
/// statistic generation operations.
///
/// The inner `String` must be a path ending with `'/'` that points to the root
/// of the `database_data/` directory (e.g. `"../database_data/"`).
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
// Wrapper over database root path used for statistic generation.
pub struct DBStatistic(pub String);

impl DBStatistic {
    /// Returns the total number of catch records in a specific map-point CSV file
    /// that belong to the given `test` session.
    ///
    /// Opens `<db_root>/maps/<map>/<point_id>.csv`, iterates all rows, and counts
    /// those whose `test` column (index 1) matches `test`.
    /// Returns `0` if the file cannot be opened.
    ///
    /// # Arguments
    /// * `map`      – Map directory name.
    /// * `point_id` – Point file stem (without `.csv`).
    /// * `test`     – Session/test name to filter by.
    // Counts how many rows exist for a specific (map, point, test) combination.
    fn get_point_count(&self, map: &String, point_id: &String, test: &String) -> u64 {
        
        let path = self.0.to_string() + "maps/" + map.as_str() + "/" + point_id.as_str() + ".csv";
        let file = File::open(path);
        if let Err(err) = file {
            println!("Error with map {} {}: {}", map, point_id, err);
            return 0;
        }
        let file = file.unwrap();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(file);

        // Count only records that belong to the requested test.
        rdr.records().fold(0, |acc, rec| {
            let record = rec.unwrap()[1].to_string();
            if &record == test {
                acc + 1
            } else {
                acc
            }
             
        })
    }

    /// Builds a per-point catch summary for one fish species and writes the result
    /// to a new CSV file.
    ///
    /// For each `(point, map, test)` combination found in the species log the method
    /// accumulates total mass and catch count, then looks up how many total attempts
    /// were made at that point/test via [`get_point_count`](Self::get_point_count).
    ///
    /// The output file is recreated from scratch with the header:
    /// `map;point;test;avg_mass;count;point_rate`
    ///
    /// If the optional `maps` filter is provided, only records whose `map` column
    /// matches one of the listed names are included.
    ///
    /// # Arguments
    /// * `fish_name` – Species name (used as file stem under `fishes/`).
    /// * `out_name`  – Destination CSV file path.
    /// * `maps`      – Optional whitelist of map names to include.
    // Builds per-point fish summary and writes it to output CSV.
    // Optional `maps` filter limits processing to selected map names.
    pub fn get_point_info_fish(&self, fish_name: &String, out_name: &String, maps: &Option<Vec<String>>){
        
        let path = self.0.to_string() + "fishes/" + fish_name.as_str() + ".csv";
        let file = File::open(path);
        if let Err(err) = file {
            println!("Error with fish {}: {}", fish_name, err);
            return;
        }
        let file = file.unwrap();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(file);
        
        // (point_id, map, test) -> (total_mass, fish_count, total_point_attempts)
        let mut point_stats: HashMap<(String, String, String), (u64, u64, u64)> = HashMap::new();
        
        for result in rdr.records() {  
            let fish = result.unwrap();      
            
            let map = fish[2].to_string();

            // Skip records outside requested map filter.
            if let Some(filter) = maps {
                if !filter.contains(&map) {
                    continue;
                }
            }
            let point_id = fish[3].to_string();
            let mass = fish[7].parse::<u64>().unwrap();
            let test = fish[1].to_string();
            let point_count = self.get_point_count(&map, &point_id, &test);
            let mut point = match point_stats.get(&(point_id.to_string(), map.to_string(), test.to_string())) {
                Some(data) => *data,
                None => (0, 0, point_count)
            };

            point.0 += mass;
            point.1 += 1;
            point.2 = point_count;

            point_stats.insert((point_id.to_string(), map.to_string(), test.to_string()), point);
        }

        // Recreate output file and write header.
        let result = format!("map;point;test;avg_mass;count;point_rate\n");
        let _ = fs::remove_file(&out_name);
        let _ = cvs_file_exists(&out_name, &result);

        // Persist per-point stats and also compute normalized metrics.
        let point_stats: BTreeMap<(String, String, String), (f64, u64, f64, u64)> = point_stats.into_iter()
            .map(|((point_id, map, test), (mass, count, point_count))| {
                
                let fish_str = format!("{};{};{};{};{};{};\n", &map, &point_id, &test, mass, count, point_count);
                let _ = cvs_file_adding(&out_name, &fish_str);

                ((point_id, map, test), (mass as f64 / count as f64, count, count as f64 / point_count as f64, point_count))
            }).collect();
        dbg!(&point_stats);

    }

    /// Reads one map-point CSV file and appends all catch records whose timestamp
    /// is greater than or equal to `timestamp` into the aggregated output file.
    ///
    /// Timestamps in the source file are stored as formatted strings
    /// (`"%Y%m%d_%H%M%S"`). They are parsed, converted to UTC, and then shifted
    /// by a fixed −2 h offset that compensates for the timezone used when the
    /// original data was recorded.
    ///
    /// Rows older than `timestamp` are silently skipped.
    ///
    /// # Arguments
    /// * `out_name`  – Path to the aggregated output CSV file.
    /// * `path`      – Path to the source map-point CSV file to read.
    /// * `timestamp` – Lower-bound Unix timestamp in milliseconds (inclusive).
    // Appends records from one map-point CSV file into aggregated output,
    // keeping only events newer than or equal to `timestamp` threshold.
    fn add_file(&self, out_name: &String, path: &String, timestamp: u128){
        
        let file = File::open(path);
        if let Err(err) = file {
            println!("Error with fish {}: {}", path, err);
            return;
        }
        let file = file.unwrap();
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(file);

        // Parse each row and convert textual timestamp into unix milliseconds.
        for result in rdr.records() { 
            let fish = result.unwrap();
            let format = "%Y%m%d_%H%M%S";
            let naive = NaiveDateTime::parse_from_str(&fish[4].to_string(), format)
                .expect("Invalid date format");
            let datetime = Utc.from_utc_datetime(&naive);
            // Apply current timezone offset correction used by project data.
            let fish_time: u128 = datetime.timestamp_millis() as u128 - 2*60*60*1000;
            let name = fish[0].to_string();
            let test = fish[1].to_string();
            let map = fish[2].to_string();
            let point = fish[3].to_string();
                        dbg!(&(fish_time, timestamp, fish_time < timestamp));
            // Skip old events.
            if fish_time < timestamp {
                continue;
            }

            // Write normalized row to aggregated CSV.
            let fish_str = format!("{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}\n", &name, &test, &map, &point, fish[4].to_string(), fish[5].parse::<u64>().unwrap(), fish[6].parse::<u64>().unwrap(),fish[7].parse::<u64>().unwrap_or(0),fish[8].parse::<u64>().unwrap_or(0),fish[9].parse::<u64>().unwrap_or(0),fish[10].parse::<u64>().unwrap_or(0),fish[11].parse::<u64>().unwrap_or(0),fish[12].parse::<u64>().unwrap_or(0),&fish[13],fish[14].parse::<u64>().unwrap_or(0));
            let _ = cvs_file_adding(&out_name, &fish_str);

        }
    }

    /// Builds a merged catch statistics CSV from all map-point source files.
    ///
    /// The output file is recreated with the full project column header:
    /// `name;test;map;point;timestamp;mass;long;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;device;exp_real`
    ///
    /// If `maps` is `Some`, only those map directories are processed.
    /// If `maps` is `None`, all subdirectories of `<db_root>/maps/` are scanned
    /// automatically.
    ///
    /// For each map directory every point CSV file is passed through
    /// [`add_file`](Self::add_file), which filters rows by `timestamp` and appends
    /// the qualifying ones to `out_name`.
    ///
    /// # Arguments
    /// * `out_name`  – Destination CSV file path.
    /// * `maps`      – Optional whitelist of map directory names.
    /// * `timestamp` – Lower-bound Unix timestamp in milliseconds (inclusive).
    ///
    /// # Panics
    /// Panics if any required directory cannot be read.
    // Creates a merged statistics CSV from map/point source files.
    // If `maps` is None, all map directories are scanned automatically.
    pub fn make_statistic(&self, out_name: &String, maps: &Option<Vec<String>>, timestamp: u128) {
        // Recreate output file and write header.
        let result = format!("name;test;map;point;timestamp;mass;long;exp;exp_l;exp_happy;exp_prem;exp_sum;exp_drink;device;exp_real\n");
        let _ = fs::remove_file(&out_name);
        let _ = cvs_file_exists(&out_name, &result);

        // Determine map list either from filter or from filesystem scan.
        let maps_available: Vec<String> = if let Some(ms) = maps {
            ms.clone()
        } else {
            match fs::read_dir(self.0.to_string() + "maps") {
                Ok(entries) => {
                    entries.filter_map(|entry|{
                        let entry = entry.unwrap();
                        if !entry.file_type().unwrap().is_dir() {
                            return None;
                        }
                        Some(entry.file_name().into_string().unwrap())
                    }).collect()
                }
                Err(e) => panic!("Error: {}", e),
            }
        };
        dbg!(&maps_available);

        // Iterate maps and all point files, then append filtered rows.
        maps_available.iter().for_each(|map| {
            match fs::read_dir(self.0.to_string() + "maps/" + map.as_str()) {
                Ok(entries) => {
                    let points: Vec<String> = entries.filter_map(|entry|{
                        let entry = entry.unwrap();
                        if entry.file_type().unwrap().is_dir() {
                            return None;
                        }
                        Some(entry.file_name().into_string().unwrap())
                    }).collect();
                        dbg!(&points);
                    points.iter().for_each(|point| {
                        let path = self.0.to_string() + "maps/" + map.as_str() + "/" + point;
                        dbg!(&path);

                        // Aggregate one point file into final output.
                        self.add_file(out_name, &path, timestamp);
                    });
                }
                Err(e) => panic!("Error: {}", e),
            }

        });

    }
}

