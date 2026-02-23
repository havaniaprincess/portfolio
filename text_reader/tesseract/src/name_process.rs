//! OCR text post-processing for fish detection.
//!
//! Provides helpers for:
//! - Computing tag-based Levenshtein distances to identify fish names.
//! - Parsing raw OCR output to extract (mass, length) pairs.
//! - Parsing EXP snippet files, scoring candidates against the statistics
//!   database, and reconstructing the full EXP breakdown.

use std::{collections::HashMap, sync::{Arc, Mutex}, u64};

use database_lib::{config::{algorythms::get_config_item, Config, DevicePreset, ExpType}, db_config::{DatabaseConfig, MistakeProb}};
use regex::Regex;
use strsim::levenshtein;
use rayon::prelude::*;
use tesseract_lib::commands::get_text_tesseract;


/// Legacy / experimental result type for fish parameter recognition.
///
/// Represents three confidence levels for a detected `(mass, length, score)`
/// triple. Currently unused in the main pipeline but kept for reference.
#[derive(Clone, Debug)]
pub enum _FishParamRes {
    None,
    Likely((u64, u64, f64)),
    Badly((u64, u64, f64)),
}


/// Legacy fish-name detector based on aggregated tag distances.
///
/// For each OCR candidate string in `names`, computes Levenshtein distances
/// to every known tag (via [`get_fish_tags`]), then scores each configured
/// fish by the sum of its tag distances divided by the fish name length.
/// Returns the fish with the lowest (best) score together with that score.
///
/// # Arguments
/// * `config` – runtime configuration containing tag and fish definitions.
/// * `names`  – list of raw OCR strings to search.
///
/// # Returns
/// `(fish_name, score)` for the best-matching fish.
pub fn _get_fish_name(config: &Config, names: &Vec<String>) -> (String, f64) {
    // Build map: tag -> minimal distance found in OCR candidates.
    let tags: HashMap<String, usize> = names.par_iter()
        .map(|name| {
            get_fish_tags(config, name)
        }).collect::<Vec<Vec<(String, usize)>>>().concat().into_iter().collect();

    // Score each fish by total tag distance and choose the smallest score.
    let mut fishes: Vec<(String, f64)> = config.fishes.par_iter()
        .map(|(name, fish)| {
            let dist = fish.tags.iter()
                .fold(0, |acc, tag| {
                    acc + match tags.get(tag) {
                        Some(d) => *d,
                        None => 0
                    }
                });
            (name.to_string(), (dist as f64) / (name.chars().count() as f64))
        }).collect();
    fishes.sort_unstable_by(|left ,right| {
        left.1.partial_cmp(&right.1).unwrap()
    });
    fishes[0].clone()
}

/// Computes the minimum sliding-window Levenshtein distance from the OCR
/// string `look_name` to every known tag in the config.
///
/// For each tag a window of the same character length is slid over `look_name`
/// and the smallest distance across all positions is recorded. Newlines in
/// the input are replaced with spaces before matching.
///
/// # Arguments
/// * `config`    – runtime configuration that holds the tag list.
/// * `look_name` – the raw OCR string to search within.
///
/// # Returns
/// A vector of `(tag, min_distance)` pairs — one entry per known tag.
pub fn get_fish_tags(config: &Config, look_name: &String) -> Vec<(String, usize)> {
    

    let tags: Vec<(String, usize)> = config.tags.par_iter()
        .filter_map(|tag| {
            let str = look_name.replace("\n", " ");
            let chars: Vec<char> = str.chars().collect();
            let dist = chars.windows(tag.chars().count())
            .fold( usize::MAX,|acc, word| {
                let dist = levenshtein(tag, &word.iter().collect::<String>());
                if acc > dist {dist} else {acc}
            });
            Some((tag.to_string(), dist))
        }).collect();
    tags
}

/// Extracts fish mass and length from a list of OCR candidate strings and
/// returns the best-validated `(mass, length, error)` triple.
///
/// Each string in `names` is processed in parallel by [`get_fish_mass`].
/// Results are sorted by validation error (ascending) and the best candidate
/// is returned. Falls back to `(0, 0, NotEnoughData)` when no candidates
/// are found.
///
/// # Arguments
/// * `config`    – runtime configuration.
/// * `db`        – mutable reference to the statistics database used for
///                 cross-validation.
/// * `names`     – raw OCR strings from different language models.
/// * `fish_name` – optionally identified fish `(name, score)`; used to
///                 narrow cross-validation.
///
/// # Returns
/// `(mass_g, length_mm, validation_error)`.
pub fn get_fish_param(config: &Config, db: &mut DatabaseConfig, names: &Vec<String>, fish_name: &Option<(String, f64)>) -> (u64, u64, MistakeProb) {
    // Shared DB guard for parallel OCR candidate processing.
    let db_arc = Arc::new(Mutex::new(db));
    let mut data: Vec<(u64, u64, MistakeProb)> = names.par_iter()
        .filter_map(|name| {
            get_fish_mass(config, &mut db_arc.lock().unwrap(), name, fish_name)            
        }).collect::<Vec<(u64, u64, MistakeProb)>>();

    // Sort by validation error (lower is better).
    data.sort_by(| left, right | {
        let left_acc_0 = match left.2 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        let right_acc_0 = match right.2 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        left_acc_0.partial_cmp(&right_acc_0).unwrap()
    });
    if data.len() == 1 {
        return data[0].clone();
    }
    if data.len() > 0 {
        return data[0].clone();
    }
    (0, 0, MistakeProb::NotEnoughData)
}


/// Parses a raw OCR string line-by-line to extract `(mass, length)` candidates.
///
/// Each line (skipping the first) is searched for numeric tokens with a regex.
/// When at least two numbers are found the first two are taken as `(mass, length)`
/// and validated against the statistics database when a fish name is known.
/// The candidate with the smallest validation error is returned.
///
/// # Arguments
/// * `_config`   – runtime configuration (reserved for future use).
/// * `db`        – mutable reference to the statistics database.
/// * `str`       – raw OCR string to parse.
/// * `fish_name` – optionally identified fish used for DB cross-validation.
///
/// # Returns
/// `Some((mass, length, error))` for the best candidate, or `None` if no
/// valid pair was found in the text.
pub fn get_fish_mass(_config: &Config, db: &mut DatabaseConfig, str: &String, fish_name: &Option<(String, f64)>) -> Option<(u64, u64, MistakeProb)> {
    dbg!(fish_name);
    let re = Regex::new(r"[-+]?\d*\.?\d+").unwrap();
    let mut lines: Vec<(Vec<u64>, MistakeProb)> = str.split("\n").enumerate()
        .filter_map(|(idx, line)| {
            if idx == 0 {
                return None;
            }
            let res_line = line.replace(".", "");
            let numbers: Vec<u64> = re.find_iter(&res_line).filter_map(|n| {
                match n.as_str().parse::<u64>() {
                    Ok(d) => Some(d),
                    Err(_) => None
                }
            }).collect();
            if numbers.len() >= 2 {
                let pairs = numbers.into_iter().take(2).collect::<Vec<u64>>();
                let mass = pairs[0];
                let long = pairs[1];
                //dbg!((fish_name, &pairs, &res_line));

                // Validate pair against fish statistics when fish is known.
                let accuracy = match fish_name {
                    Some((f_name, _)) => db.check_value_long_mass(f_name, mass, long),
                    None => MistakeProb::NotEnoughData
                };
                
                

                Some((pairs, accuracy))
            } else {
                None
            }
        }).collect();
    //dbg!(&lines);

    // Keep candidate with minimal validation error.
    lines.sort_by(| left, right | {
        let left_acc_0 = match left.1 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        let right_acc_0 = match right.1 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        left_acc_0.partial_cmp(&right_acc_0).unwrap()
    });
    if lines.len() > 0 {
        return Some((lines[0].0[0], lines[0].0[1], lines[0].1.clone()))
    }
    None
}



/// Recognises EXP fragments from a set of OCR snippet files and reconstructs
/// the full EXP breakdown for one catch event.
///
/// For each snippet file the function:
/// 1. Runs Tesseract OCR with every language model in `langs`.
/// 2. Identifies the EXP category via tag matching ([`get_config_item`]).
/// 3. Parses all numeric tokens and validates each against the DB.
/// 4. Converts raw in-game values to the "clear" (rig-independent) base using
///    the effective device preset's light multiplier.
/// 5. Delegates to [`find_base_exp`] to select the most reliable base EXP value.
/// 6. Derives all bonus components (light, happy-hour, premium, drink) and
///    the rig-adjusted real EXP from the chosen base.
///
/// # Arguments
/// * `config`        – runtime configuration (tags, fishes, device presets, rigs).
/// * `db`            – mutable reference to the statistics database.
/// * `files`         – paths to preprocessed EXP snippet PNG files.
/// * `langs`         – Tesseract language model names to try (e.g. `["rus_hp2", "rus"]`).
/// * `fish_name`     – optionally identified fish used for DB validation.
/// * `mass`          – detected fish mass used for DB validation.
/// * `device_preset` – active device preset; inferred from EXP data when `None`.
///
/// # Returns
/// A `HashMap<ExpType, (value, error)>` with entries for `Base`, `LBonus`,
/// `HappyBonus`, `PremBonus`, `DrinkBonus`, `AllExp`, and `Real`.
/// Returns an empty map if no reliable base EXP can be determined.
pub fn get_exp(config: &Config, db: &mut DatabaseConfig, files: &Vec<String>, langs: &Vec<String>, fish_name: &Option<(String, f64)>, mass: u64, device_preset: &Option<&DevicePreset>) -> HashMap<ExpType, (u64, MistakeProb)> {
    //dbg!(fish_name);
    // For each OCR snippet file:
    // 1) run OCR with multiple languages,
    // 2) detect EXP type by tags,
    // 3) parse candidate numbers and score them.
    let exps: HashMap<ExpType, Vec<(u64, MistakeProb)>> = files.iter()
        .filter_map(|file| {
            let res_texts: Vec<String> = langs.iter()
                .map(|lang| {
                    get_text_tesseract(file, lang).to_lowercase()
                }).collect();
            //dbg!(&res_texts);
            let exp_res = get_config_item(&res_texts, &config.exp_tags, &config.exp_types);
            //dbg!(&exp_res);
            let re = Regex::new(r"[-+]?\d*\.?\d+").unwrap();
            let mut exps = res_texts.iter()
                .map(|text| {
                    let res_line = text.replace(" ", "");
                    let numbers: Vec<u64> = re.find_iter(&res_line).filter_map(|n| {
                        match n.as_str().parse::<u64>() {
                            Ok(d) => Some(d),
                            Err(_) => None
                        }
                    }).collect();
                    numbers.iter()
                        .map(|num| {
                            let acc = match fish_name {
                                Some((f_name, _)) => db.check_value_mass_exp(f_name, mass, *num),
                                None => MistakeProb::NotEnoughData
                            };
                            (*num, acc)
                        }).collect::<Vec<(u64, MistakeProb)>>()
                    
                }).collect::<Vec<Vec<(u64, MistakeProb)>>>().concat();
            sort_exp_array(&mut exps);
            //dbg!(&exps);
            if exps.len() == 0 || exp_res == None {
                return None;
            }   
            Some((exp_res.unwrap().0, exps))
        }).collect();

    // Resolve effective device preset when not supplied explicitly.
    let device_final = if device_preset.clone() == None {
        if exps.get(&ExpType::LBonus) == None {
            config.device_presets.get(&"mah_losinoe_default".to_string()).unwrap()
        } else {
            config.device_presets.get(&"match_losinoe_default".to_string()).unwrap()
        }
    } else {
        device_preset.unwrap()
    };

    // Convert OCR EXP values to clear/base space using rig multiplier.
    let exps = exps.into_iter()
        .map(|(etype, ex_vec)| {
            (etype, ex_vec.into_iter().map(|(exp, prob)| (config.get_clear_exp(device_final, exp), prob.clone())).collect())
        }).collect();

    // Derive base EXP and rebuild all dependent EXP components.
    let base = find_base_exp(db, &exps, fish_name, mass, device_final.light_mul);
    //dbg!(&base);
    if base == None {
        return HashMap::new();
    }
    let base_exp = base.clone().unwrap().0;
    let base_err = base.clone().unwrap().1;
    let there_is_happy = exps.get(&ExpType::HappyBonus) != None;
    let there_is_premium = exps.get(&ExpType::PremBonus) != None;
    let there_is_l = exps.get(&ExpType::LBonus) != None;
    let there_is_drink = exps.get(&ExpType::DrinkBonus) != None;

    let l_exp = if there_is_l {device_final.light_mul * base_exp} else {0.0};
    let happy_exp = if there_is_happy {base_exp * 2.0} else {0.0};
    let prem_exp = if there_is_premium {base_exp} else {0.0};
    let drink_exp = if there_is_drink {exps.get(&ExpType::DrinkBonus).unwrap()[0].0} else {0.0};
    let all_exp = base_exp + happy_exp + l_exp + prem_exp + drink_exp;
    let rig_mul = config.rigs.get(&device_final.rig).unwrap().exp_mul;

    // Return normalized EXP map with validation status.
    vec![
        (ExpType::Base, (base_exp.round() as u64, base_err.clone())),
        (ExpType::LBonus, (l_exp.round() as u64, base_err.clone())),
        (ExpType::HappyBonus, (happy_exp.round() as u64, base_err.clone())),
        (ExpType::PremBonus, (prem_exp.round() as u64, base_err.clone())),
        (ExpType::DrinkBonus, (drink_exp.round() as u64, base_err.clone())),
        (ExpType::AllExp, (all_exp.round() as u64, base_err.clone())),
        (ExpType::Real, (((base_exp) * rig_mul).round() as u64, base_err.clone())),
    ].into_iter().collect()
}

/// Sorts a `(value, MistakeProb)` vector in ascending order of validation error.
///
/// `FullAccuracy` and `MaybeMistake` use their inner `f64` score directly;
/// `FullyNotSure` likewise; `NotEnoughData` is treated as `f64::MAX` so it
/// always sorts to the end.
fn sort_exp_array<T>(array: &mut Vec<(T, MistakeProb)>) {
    array.sort_by(| left, right | {
        let left_acc_0 = match left.1 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        let right_acc_0 = match right.1 {
            MistakeProb::FullAccuracy(d) => d,
            MistakeProb::FullyNotSure(d) => d,
            MistakeProb::MaybeMistake(d) => d,
            MistakeProb::NotEnoughData => f64::MAX,
        };
        left_acc_0.partial_cmp(&right_acc_0).unwrap()
    });
}

/// Legacy helper: selects the light-bonus candidate from `exps` that is
/// numerically closest to one of the expected fractions of `base`
/// (`base/4`, `base/2`, `base`).
///
/// Returns the best-matching base-equivalent value, or `None` if `exps`
/// is empty. Currently unused in the main pipeline.
fn _find_l_bonus(db: &mut DatabaseConfig, exps: &Vec<(u64, MistakeProb)>, base: u64) -> Option<u64> {
    let k_set: Vec<u64> = vec![base / 4, base / 2, base];
    let mut variants = exps.iter()
        .map(|(exp, _)| {
            k_set.iter().map(|base_k| {
                (*base_k, base_k.abs_diff(*exp)) 
            }).collect::<Vec<(u64, u64)>>()
        }).collect::<Vec<Vec<(u64, u64)>>>().concat();
    if variants.len() == 0 {
        return None;
    }
    variants.sort_by(| left, right | {
        left.1.partial_cmp(&right.1).unwrap()
    });
    Some(variants[0].0)
}

/// Determines the most reliable base EXP value from the available EXP categories.
///
/// Each present category (`Base`, `HappyBonus`, `PremBonus`, `LBonus`) is
/// projected back to base-EXP space:
/// - `Base`       → used as-is.
/// - `HappyBonus` → divided by 2 (happy hour doubles base).
/// - `PremBonus`  → used as-is (premium equals base).
/// - `LBonus`     → divided by `light_mul`.
///
/// Projected candidates are re-validated against the DB and sorted by error.
/// The candidate with the smallest error across all categories is returned.
///
/// # Arguments
/// * `db`         – mutable reference to the statistics database.
/// * `exps`       – map of detected EXP categories with their scored candidates
///                  already converted to clear (rig-independent) floats.
/// * `fish_name`  – optionally identified fish used for DB validation.
/// * `mass`       – detected fish mass used for DB validation.
/// * `light_mul`  – light-bonus multiplier from the active device preset.
///
/// # Returns
/// `Some((base_exp, error))` for the best candidate, or `None` if no EXP
/// categories are present.
fn find_base_exp(db: &mut DatabaseConfig, exps: &HashMap<ExpType, Vec<(f64, MistakeProb)>>, fish_name: &Option<(String, f64)>, mass: u64, light_mul: f64) -> Option<(f64, MistakeProb)> {

        let there_is_base = exps.get(&ExpType::Base) != None;
        let there_is_happy = exps.get(&ExpType::HappyBonus) != None;
        let there_is_premium = exps.get(&ExpType::PremBonus) != None;
        let there_is_l = exps.get(&ExpType::LBonus) != None;
        let base_base = if there_is_base {
            Some(exps.get(&ExpType::Base).unwrap()[0].clone())
        } else {
            None
        };
        let base_happy = if there_is_happy {
            let mut happies_exp = exps.get(&ExpType::HappyBonus).unwrap()
                .iter().map(|obj| {
                    let happy_base = obj.0 / 2.0;
                    let acc = match fish_name {
                        Some((f_name, _)) => db.check_value_mass_exp(f_name, mass, happy_base as u64),
                        None => MistakeProb::NotEnoughData
                    };
                    (happy_base, acc)
                }).collect::<Vec<(f64, MistakeProb)>>();
            sort_exp_array(&mut happies_exp);
            Some(happies_exp[0].clone())
        } else {
            None
        };
        let base_prem = if there_is_premium {
            let mut prems_exp = exps.get(&ExpType::PremBonus).unwrap()
                .iter().map(|obj| {
                    let prem_base = obj.0;
                    let acc = match fish_name {
                        Some((f_name, _)) => db.check_value_mass_exp(f_name, mass, prem_base as u64),
                        None => MistakeProb::NotEnoughData
                    };
                    (prem_base, acc)
                }).collect::<Vec<(f64, MistakeProb)>>();
            sort_exp_array(&mut prems_exp);
            Some(prems_exp[0].clone())
        } else {
            None
        };
        let _base_l = if there_is_l {
            let mut l_exp = exps.get(&ExpType::LBonus).unwrap()
                .iter().map(|obj| {
                    let l_base = obj.0 / light_mul;
                    let acc = match fish_name {
                        Some((f_name, _)) => db.check_value_mass_exp(f_name, mass, l_base as u64),
                        None => MistakeProb::NotEnoughData
                    };
                    (l_base, acc)
                }).collect::<Vec<(f64, MistakeProb)>>();
            sort_exp_array(&mut l_exp);
            Some(l_exp[0].clone())
        } else {
            None
        };
        //dbg!((&base_base, &base_happy, &base_prem));
        let mut bases: Vec<(f64, MistakeProb)> = vec![base_base, base_happy, base_prem].into_iter().filter_map(|obj| obj).collect();
        if bases.len() == 0 {
            return None;
        }
        sort_exp_array(&mut bases);
        let (base_exp, base_prob) = bases[0].clone();
        Some((base_exp, base_prob))
}