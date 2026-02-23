use std::collections::BTreeMap;
use std::fs::File;
use std::io::Read;
use std::path::Path;
use tesseract_lib::fs::dir_exists;

use crate::db_config::StatFunc;



pub fn json_hashmap_load<P: AsRef<Path>>(path: P) -> Option<BTreeMap<u64, StatFunc>>
{
    // Return None if the target file/path does not exist.
        if dir_exists(&path, false) != Ok(()) {
            return None;
        }

    // Open JSON file and read its full contents into a string buffer.
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

    // Deserialize JSON into a sorted map: key -> StatFunc.
        let conv: BTreeMap<u64, StatFunc> = serde_json::from_str(&contents).expect("JSON was not well-formatted");

    // Return parsed structure.
        Some(conv)
    
}
pub fn json_regression_load<P: AsRef<Path>>(path: P) -> Option<Vec<f64>>
{
    // Return None if the target file/path does not exist.
        if dir_exists(&path, false) != Ok(()) {
            return None;
        }

    // Open JSON file and read its full contents into a string buffer.
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();

    // Deserialize JSON into regression coefficients vector.
        let conv: Vec<f64> = serde_json::from_str(&contents).expect("JSON was not well-formatted");

    // Return parsed vector.
        Some(conv)
    
}