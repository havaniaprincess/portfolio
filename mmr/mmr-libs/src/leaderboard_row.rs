use crate::types::LeaderboardRow;

impl LeaderboardRow {
    pub fn parse_file(line: String) -> Option<Self> {
        // Parse one flat JSON-like line into key/value pairs.
        let hash_line: std::collections::BTreeMap<String, String> = line.replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
            let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
            if splited.len() == 2 {
                Some((splited[0].clone(), splited[1].clone()))
            } else {
                None
            }
        }).collect();
    
        let user_id = hash_line.get("user_id");

        // user_id is required; skip malformed lines.
        if user_id == None {
            return None;
        }

        
        // Build row, using zero defaults for missing optional fields.
        Some(
            LeaderboardRow { 
                user_id: user_id.unwrap().parse::<u64>().unwrap(), 
                battles: match hash_line.get("battles") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                },  
                mmr: match hash_line.get("mmr") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                },  
                victories: match hash_line.get("victories") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                },  
                early_quites: match hash_line.get("early_quites") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                },  
                top_20: match hash_line.get("top_20") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                }, 
                battle_score: match hash_line.get("battle_score") {
                    Some(data) => data.parse::<u32>().unwrap(),
                    None => 0
                },
                last_session: match hash_line.get("last_session") {
                    Some(data) => data.parse::<u64>().unwrap(),
                    None => 0
                },  

            }
        )


    }
    pub fn to_string(&self) -> String {
        // Serialize row back to the legacy comma-separated key:value format.
        let mut str: String = String::new();
        str = str + "user_id:" + self.user_id.to_string().as_str();
        str = str + ",battles:" + self.battles.to_string().as_str();
        str = str + ",battle_score:" + self.battle_score.to_string().as_str();
        str = str + ",mmr:" + self.mmr.to_string().as_str();
        str = str + ",victories:" + self.victories.to_string().as_str();
        str = str + ",early_quites:" + self.early_quites.to_string().as_str();
        str = str + ",top_20:" + self.top_20.to_string().as_str();
        str = str + ",last_session:" + self.last_session.to_string().as_str();
        str
    }
}