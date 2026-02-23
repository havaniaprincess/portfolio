use std::{collections::{HashMap, HashSet}, fs::File};
use std::io::{Write, BufWriter, Read};

use tesseract_lib::fs::dir_exists;

pub mod traits;
pub mod impliment;
pub mod algorythms;

// Types of EXP values used in parsed game records.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum ExpType{
    Base,
    LBonus,
    HappyBonus,
    PremBonus,
    DrinkBonus,
    AllExp,
    Real,
}

// Line material types for device presets.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum LineType{
    Flur,
    Pleten,
    Neylon,
    Steel,
}

// EXP descriptor with matching tags for OCR/text recognition.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Exp{
    pub name: ExpType,
    pub tags: Vec<String>,
    pub primary_tags: Vec<String>,
}


// Main application configuration:
// fish dictionary, tags, EXP mappings, device presets, and rigs.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Config{
    pub fishes: HashMap<String, Fish>,
    pub tags: HashSet<String>,
    pub exp_types: HashMap<ExpType, Exp>,
    pub exp_tags: HashSet<String>,
    pub device_presets: HashMap<String, DevicePreset>,
    pub rigs: HashMap<String, Rig>,
}

// Rig parameters affecting EXP conversion.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Rig{
    pub exp_mul: f64
}

// Device configuration preset used during CSV recalculations.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize, PartialEq, PartialOrd)]
pub struct DevicePreset{
    pub id: String,
    pub device_type: String,
    pub rig: String,
    pub bait: String,
    pub line: (LineType, f64, f64),
    pub hook: (String, u16),
    pub light_mul: f64,
    pub distance: u64,
    pub depth: u64,
}

// Fish metadata and tag set used for recognition.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Fish{
    pub name: String,
    pub weights: Vec<u64>,
    pub tags: Vec<String>,
    pub primary_tags: Vec<String>,
}



impl Config {
    // Builds an empty default configuration object.
    pub fn new() -> Self {
        Self { 
            fishes: HashMap::new(),
            tags: HashSet::new(),
            exp_types: HashMap::new(),
            exp_tags: HashSet::new(),
            device_presets: HashMap::new(),
            rigs: HashMap::new(),
        }
    }

    // Reads config JSON from disk and computes derived tag sets.
    // Returns None if the path does not exist.
    pub fn read(path: &String) -> Option<Self> {
        if dir_exists(path, false) != Ok(()) {
            return None;
        }

        // Parse raw JSON into Config.
        let mut file = File::open(path).unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).unwrap();
        let mut conv: Self = serde_json::from_str(&contents).expect("JSON was not well-formatted");

        // Build union of all fish tags.
        conv.tags = conv.fishes.iter()
            .map(|(_, fish)| fish.tags.to_vec()).collect::<Vec<Vec<String>>>().concat().into_iter().collect();

        // Build union of all EXP tags.
        conv.exp_tags = conv.exp_types.iter()
            .map(|(_, exp)| exp.tags.to_vec()).collect::<Vec<Vec<String>>>().concat().into_iter().collect();
        Some(conv)
    }

    // Saves the full configuration as pretty-formatted JSON.
    pub fn save(&self, path: &String) {
        
        let pretty_json = serde_json::to_string_pretty(self).unwrap();

        let data_file = File::create(path).unwrap();
        let mut data_file = BufWriter::new(data_file);
        data_file.write_all(pretty_json.as_bytes()).unwrap();
        data_file.flush().unwrap();
    }
    
    // Converts displayed/total EXP into clear/base EXP using rig multiplier.
    pub fn get_clear_exp(&self, preset: &DevicePreset, exp: u64) -> f64 {
        let rig_mul = self.rigs.get(&preset.rig).unwrap().exp_mul;
        // floor(clear * mul) = exp
        exp as f64 / rig_mul
    }
}