use std::{collections::{HashMap, HashSet}, path::Path};

use tokio::{fs::File, io::{AsyncWriteExt, BufWriter}};
use mmr_libs::{datasets::UserCampaigh, memory::read_lines, reader::reader};
use mission_group::{BalanceCat, WinTeam};

mod mission_group;

#[tokio::main]
async fn main() {
    let mut mission_group: HashMap<(String, BalanceCat, WinTeam), u64> = HashMap::new();
    let mut mission_balance_group: HashMap<(String, BalanceCat), u64> = HashMap::new();
    let mut mission_win_group: HashMap<(String, WinTeam), (u64, u64)> = HashMap::new();
    let mut mission_counts: HashMap<String, u64> = HashMap::new();
    let mut session_dau: u64 = 0;

    let mut session_mission: std::collections::HashMap<u64, String> = std::collections::HashMap::new();
    if Path::new("data/session_mission.json").exists() {
        if let Ok(lines) = read_lines("data/session_mission.json") {
            for line in lines.flatten() {
                let hash_line = reader(&line);
            
                let session_id = hash_line.get("session_id_str");
                let mission = hash_line.get("mission");
                
                if session_id == None || mission == None {
                    continue;
                }

                session_mission.insert(session_id.unwrap().parse::<u64>().unwrap(), mission.unwrap().to_string());

                let counts = match mission_counts.get(mission.unwrap()) {
                    Some(&c) => c,
                    None => 0
                };

                mission_counts.insert(mission.unwrap().to_string(), counts + 1);
            }
        }
    }

    
    let user_campaign = UserCampaigh::new();

    
    let nations: HashSet<(u64, String)> = user_campaign.0.iter().map(|obj| (obj.0.1, obj.1.to_string())).collect();

    let mut session_classification: std::collections::HashMap<u64, (BalanceCat, WinTeam)> = std::collections::HashMap::new();
    if Path::new("data/leaderboard_v1/session_classification").exists() {
        if let Ok(lines) = read_lines("data/leaderboard_v1/session_classification") {
            for (idx, line) in lines.flatten().enumerate() {
                let hash_line = reader(&line);
                let session_id = hash_line.get("session_id");
                let team_1 = hash_line.get("team_1");
                let team_2 = hash_line.get("team_2");
                let team_1_v = hash_line.get("team_1_v");
                let team_2_v = hash_line.get("team_2_v");
                
                if session_id == None {
                    continue;
                }
                let session_id = session_id.unwrap().parse::<u64>().unwrap();


                let faction_1_nat = if nations.contains(&(session_id, "faction_1".to_string())) {"faction_1".to_string()} else {"faction_1".to_string()};
                let faction_2_nat = if nations.contains(&(session_id, "faction_2".to_string())) {"faction_2".to_string()} else {"faction_2".to_string()};

                let balance_group = if team_1.unwrap().parse::<bool>().unwrap() {BalanceCat::Faction1(faction_1_nat.clone())} else if team_2.unwrap().parse::<bool>().unwrap() {BalanceCat::Faction2(faction_2_nat.to_string())} else {BalanceCat::Balance};
                let win_group = if team_1_v.unwrap().parse::<bool>().unwrap() {WinTeam::Faction1(faction_1_nat.to_string())} else {WinTeam::Faction2(faction_2_nat.to_string())};
                let lose_group = if team_2_v.unwrap().parse::<bool>().unwrap() {WinTeam::Faction1(faction_1_nat.to_string())} else {WinTeam::Faction2(faction_2_nat.to_string())};

                session_classification.insert(session_id, (balance_group.clone(), win_group.clone()));

                let mission = session_mission.get(&session_id);

                if mission == None {
                    continue;
                }
                if idx % 1000 == 0{
                    println!("{:?} session_classification", mission);
                }
                let mission = mission.unwrap();
                let counts = match mission_group.get(&(mission.to_string(), balance_group.clone(), win_group.clone())) {
                    Some(&c) => c,
                    None => 0
                };

                mission_group.insert((mission.to_string(), balance_group.clone(), win_group.clone()), counts + 1);

                let counts = match mission_balance_group.get(&(mission.to_string(), balance_group.clone())) {
                    Some(&c) => c,
                    None => 0
                };

                mission_balance_group.insert((mission.to_string(), balance_group.clone()), counts + 1);

                let counts = match mission_win_group.get(&(mission.to_string(), win_group.clone())) {
                    Some(&c) => c,
                    None => (0, 0)
                };

                mission_win_group.insert((mission.to_string(), win_group.clone()), (counts.0 + 1, counts.1 + 1));

                let counts = match mission_win_group.get(&(mission.to_string(), lose_group.clone())) {
                    Some(&c) => c,
                    None => (0, 0)
                };

                mission_win_group.insert((mission.to_string(), lose_group.clone()), (counts.0, counts.1 + 1));
                session_dau += 1;

            }
        }
    }

    println!("{:?} session_classification", mission_counts.clone());
    let data_file = File::create("data/mission_stat_v1.csv".to_string()).await.unwrap();
    let mut data_file = BufWriter::new(data_file);

    let str = "mission,balance_group,win_group,cross_count,balance_count,win_count,count,session_dau\n".to_string();
    data_file.write_all(str.as_bytes()).await.unwrap();

    for ((mission, balance_group, win_group), &cross_count) in mission_group.iter() {
        let balance_counts = match mission_balance_group.get(&(mission.to_string(), balance_group.clone())) {
            Some(&d) => d,
            None => 0
        };
        let (win_counts, counts) = match mission_win_group.get(&(mission.to_string(), win_group.clone())) {
            Some(&d) => d,
            None => (0, 0)
        };

        
        let str = mission.to_string()
        + "," + balance_group.to_string().as_str()
        + "," + win_group.to_string().as_str()
        + "," + cross_count.to_string().as_str()
        + "," + balance_counts.to_string().as_str()
        + "," + win_counts.to_string().as_str()
        + "," + counts.to_string().as_str()
        + "," + session_dau.to_string().as_str() + "\n";
        data_file.write_all(str.as_bytes()).await.unwrap();
    }
    data_file.flush().await.unwrap();
  
}
