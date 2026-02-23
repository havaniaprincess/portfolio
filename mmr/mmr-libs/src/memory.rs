use crate::types::UserBattleRow;

use std::fs;
use std::path::Path;
use std::io::{self, BufRead};
use std::io::prelude::*;

#[derive(Clone, Debug)]
pub struct SessionMemory{
    pub now_session_id: u64,
    pub rows: Vec<UserBattleRow>
}

impl SessionMemory {
    pub fn new() -> Self {
        if Path::new("data/memory").exists() {
            if let Ok(lines) = read_lines("data/memory") {
                let mut session_id: u64 = 0;
                let mut rows: Vec<UserBattleRow> = Vec::new();
                for (idx, line) in lines.flatten().enumerate() {
                    if idx == 0 {
                        session_id = line.clone().parse::<u64>().unwrap();
                    }
                    match UserBattleRow::parsing_row(line.replace("\"", "")) {
                        Some(row) => {
                            rows.push(row);
                        },
                        None => {
                            
                        }
                    };
                }
                if session_id > 0 { 
                    return Self {
                        now_session_id: session_id,
                        rows: rows.clone()
                    };
                }
            }
        }

        Self {
            now_session_id: 0,
            rows: Vec::new()
        }
    }

    pub fn write(&self){

        let mut data_file = fs::File::create("data/memory").expect("creation failed");

        // Write contents to the file
        data_file.write((self.now_session_id.to_string() + "\n").as_bytes()).expect("write failed");
        let mut file = fs::File::options().append(true).create(true).open("data/memory").unwrap();

        for row in self.rows.clone().iter() {

            file.write((row.to_string() + "\n").as_bytes()).unwrap();
        }
    }
}


pub fn read_lines<P>(filename: P) -> io::Result<io::Lines<io::BufReader<fs::File>>>
where P: AsRef<Path>, {
    let file = fs::File::open(filename)?;
    Ok(io::BufReader::new(file).lines())
} 