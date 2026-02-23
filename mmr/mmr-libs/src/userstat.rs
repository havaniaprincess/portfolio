
use crate::datasets::UserFaction;
use crate::types::UserBattleRow;


impl UserBattleRow {
    pub fn parsing_str(
        line: String, 
        user_team: &std::collections::HashMap<(u64,u64), (u8, bool)>, 
        user_faction: &UserFaction,
    ) -> Option<Self> {
        // Parse one JSON-like line into key/value fields.
        let hash_line: std::collections::BTreeMap<String, String> = line.replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
            let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
            if splited.len() == 2 {
                Some((splited[0].clone(), splited[1].clone()))
            } else {
                None
            }
        }).collect();
    
        let user_id = hash_line.get("user_id");
        let session_id = hash_line.get("session_id");
        let commit_time = hash_line.get("commit_time");
    
        // Required fields for valid row construction.
        if user_id == None || session_id == None || commit_time == None {
            return None;
        }
        let user_id = user_id.unwrap().parse::<u64>().unwrap();
        // session_id is expected in hexadecimal format in this parser path.
        let session_id =  u64::from_str_radix(session_id.unwrap().clone().as_str(), 16).unwrap();
        // Team/victory are sourced from preloaded user_team map.
        let (_team, victory) = match user_team.get(&(user_id, session_id)) {
            Some((t, v)) => (*t, *v),
            None => (0, false)
        };
        // Prefer external faction dataset, fallback to row field when missing.
        let faction = match user_faction.0.get(&(user_id, session_id)) {
            Some(mode) => mode.clone(),
            None => hash_line.get("faction").unwrap().to_string()
        };
        let row = UserBattleRow { 
            user_id: user_id, 
            session_id: session_id, 
            commit_time: commit_time.unwrap().parse::<u64>().unwrap(), 
            battle_score: match hash_line.get("battle_score") {
                Some(data) => data.parse::<u32>().unwrap(),
                None => 0
            }, 
            victories: victory, 
            early_quit: match hash_line.get("early_quit") {
                Some(data) => if data.parse::<u16>().unwrap() == 1 {true} else {false},
                None => false
            }, 
            team_score_top_20_percent: match hash_line.get("team_score_top_20_percent") {
                Some(data) => if data.parse::<u16>().unwrap() == 1 {true} else {false},
                None => false
            },
            team: if faction == "faction_2".to_string() {2} else {1},
            faction: faction
        };
        Some(row)
        //None
    }
    pub fn parsing_row(line: String) -> Option<Self> {
        // Parse already-normalized stored row (session_id is decimal here).
        let hash_line: std::collections::BTreeMap<String, String> = line.replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
            let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
            if splited.len() == 2 {
                Some((splited[0].clone(), splited[1].clone()))
            } else {
                None
            }
        }).collect();
    
        let user_id = hash_line.get("user_id");
        let session_id = hash_line.get("session_id");
        let commit_time = hash_line.get("commit_time");
    
        // Required fields for valid row construction.
        if user_id == None || session_id == None || commit_time == None {
            return None;
        }
        let row = UserBattleRow { 
            user_id: user_id.unwrap().parse::<u64>().unwrap(), 
            session_id: session_id.unwrap().parse::<u64>().unwrap(), 
            commit_time: commit_time.unwrap().parse::<u64>().unwrap(), 
            team: match hash_line.get("team") {
                Some(data) => data.parse::<u8>().unwrap(),
                None => 0
            },
            battle_score: match hash_line.get("battle_score") {
                Some(data) => data.parse::<u32>().unwrap(),
                None => 0
            }, 
            victories: match hash_line.get("victories") {
                Some(data) => if data.parse::<u16>().unwrap() == 1 {true} else {false},
                None => false
            }, 
            early_quit: match hash_line.get("early_quit") {
                Some(data) => if data.parse::<u16>().unwrap() == 1 {true} else {false},
                None => false
            }, 
            team_score_top_20_percent: match hash_line.get("team_score_top_20_percent") {
                Some(data) => if data.parse::<u16>().unwrap() == 1 {true} else {false},
                None => false
            } , 
            faction: match hash_line.get("faction") {
                Some(data) => data.clone(),
                None => panic!()
            } 
        };
        Some(row)
        //None
    }
    pub fn to_string(&self) -> String {
        // Serialize row back to legacy comma-separated key:value format.
        let mut str: String = String::new();
        str = str + "user_id:" + self.user_id.to_string().as_str();
        str = str + ",session_id:" + self.session_id.to_string().as_str();
        str = str + ",commit_time:" + self.commit_time.to_string().as_str();
        str = str + ",battle_score:" + self.battle_score.to_string().as_str();
        str = str + ",team:" + self.team.to_string().as_str();
        str = str + ",faction:" + self.faction.to_string().as_str();
        // Optional flags are included only when true.
        str = str  + if self.victories {",victories:1"} else {""};
        str = str  + if self.early_quit {",early_quit:1"} else {""};
        str = str  + if self.team_score_top_20_percent {",team_score_top_20_percent:1"} else {""};
        str
    }
}