use std::path::Path;

use crate::memory;

// session_id -> (raw_mode, normalized_common_mode, normalized_specific_mode)
#[derive(Clone)]
pub struct SessionMode(pub std::collections::HashMap<u64, (String, String, String)>);

impl SessionMode {
    pub fn new(path: &str) -> Self {
        // Load session mode mapping from a JSON-like line file.
        if Path::new(path).exists() {
            let mut session_mode: std::collections::HashMap<u64, (String, String, String)> = std::collections::HashMap::new();
            if let Ok(lines) = memory::read_lines(path) { 
                
                for line in lines.flatten() {
                    // Lightweight key:value parser for expected input rows.
                    let hash_line: std::collections::BTreeMap<String, String> = line.replace("\"", "").replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
                        let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
                        if splited.len() == 2 {
                            Some((splited[0].clone(), splited[1].clone()))
                        } else {
                            None
                        }
                    }).collect();

                    let session_id = hash_line.get("session_id");
                    // Skip malformed rows without session id.
                    if session_id == None {
                        continue;
                    }
                    let mode = match hash_line.get("mode") {
                        Some(data) => data.clone(),
                        None => "".to_string()
                    };

                    let low_position = mode.find("low_teir");
                    let high_position = mode.find("high_teir");
                    let newbie_position = mode.find("newbie");

                    // Build normalized mode names for grouped and specific reporting.
                    session_mode.insert(
                        session_id.unwrap().parse::<u64>().unwrap(), 
                        (mode.clone(),
                        match newbie_position {
                            Some(_pos) => "newbie_common".to_string(),
                            None => {
                                match low_position {
                                    Some(_pos) => "low_teir_common".to_string(),
                                    None => {
                                        match high_position {
                                            Some(_pos) => "high_teir_common".to_string(),
                                            None => "lobbie_common".to_string()
                                        }
                                    }
                                }
                            }
                        },
                        match newbie_position {
                            Some(_pos) => "newbie".to_string(),
                            None => {
                                match low_position {
                                    Some(pos) => mode.clone()[pos..].to_string(),
                                    None => {
                                        match high_position {
                                            Some(pos) => mode.clone()[pos..].to_string(),
                                            None => "lobbie".to_string()
                                        }
                                    }
                                }
                            }
                        })
                    );
                }
            }
            return Self(session_mode);
        }
        // Return empty mapping when source file is missing.
        Self(std::collections::HashMap::new())
    }
}

// user_id -> registration_timestamp
#[derive(Clone)]
pub struct Registrations(pub std::collections::HashMap<u64, u64>);

impl Registrations {
    pub fn new() -> Self {
        // Loads user registrations from data/regs.json when available.
        if Path::new("data/regs.json").exists() {
            let mut regs: std::collections::HashMap<u64, u64> = std::collections::HashMap::new();
            if let Ok(lines) = memory::read_lines("data/regs.json") { 
                
                for line in lines.flatten() {
                    // Lightweight parser for JSON-like line entries.
                    let hash_line: std::collections::BTreeMap<String, String> = line.replace("\"", "").replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
                        let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
                        if splited.len() == 2 {
                            Some((splited[0].clone(), splited[1].clone()))
                        } else {
                            None
                        }
                    }).collect();

                    let user_id = hash_line.get("user_id");
                    // Skip malformed rows without user id.
                    if user_id == None {
                        continue;
                    }
                    let registered_time = match hash_line.get("registered_time") {
                        Some(data) => data.clone(),
                        None => "0".to_string()
                    };

                    regs.insert(
                        user_id.unwrap().parse::<u64>().unwrap(), 
                        registered_time.parse::<u64>().unwrap()
                    );
                }
            }
            return Self(regs);
        }
        // Return empty mapping when source file is missing.
        Self(std::collections::HashMap::new())
    }
}


// (user_id, session_id) -> mode/faction label
#[derive(Clone)]
pub struct UserFaction(pub std::collections::BTreeMap<(u64, u64), String>);

impl UserFaction {
    pub fn new(path: &str) -> Self {
        // Load user faction/mode by (user_id, session_id).
        if Path::new(path).exists() {
            let mut camps: std::collections::BTreeMap<(u64, u64), String> = std::collections::BTreeMap::new();
            if let Ok(lines) = memory::read_lines(path) { 
                
                for line in lines.flatten() {
                    // Lightweight parser for JSON-like line entries.
                    let hash_line: std::collections::BTreeMap<String, String> = line.replace("\"", "").replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
                        let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
                        if splited.len() == 2 {
                            Some((splited[0].clone(), splited[1].clone()))
                        } else {
                            None
                        }
                    }).collect();

                    let user_id = hash_line.get("user_id");
                    let session_id = hash_line.get("session_id");
                    // Skip malformed rows missing both keys.
                    if user_id == None && session_id == None {
                        continue;
                    }
                    let mode = match hash_line.get("mode") {
                        Some(data) => data.clone(),
                        None => "mixed".to_string()
                    };

                    camps.insert(
                        // sessionId is expected in hexadecimal format.
                        (user_id.unwrap().parse::<u64>().unwrap(), u64::from_str_radix(session_id.unwrap().clone().as_str(), 16).unwrap()), 
                        mode
                    );
                }
            }
            return Self(camps);
        }
        // Return empty mapping when source file is missing.
        Self(std::collections::BTreeMap::new())
    }
}