use crate::{types::{self, LeaderboardChangeV1, LeaderboardRow, MMRType, MMRChangeDebug, MMRChangeDebugV2, LeaderboardChangeV2}, memory};

use tokio::{fs::File, io::BufWriter};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::io::AsyncWriteExt;




impl std::string::ToString for types::MMRPair {
    fn to_string(&self) -> String {
        let mmr = match self.1.clone() {
            MMRType::MMR(mmr) => mmr,
            MMRType::NotEnought(_) => 0,
            MMRType::None => 0
        };
        let mmr_type = match self.1.clone() {
            MMRType::MMR(_) => "mmr",
            MMRType::NotEnought(_) => "not_enought",
            MMRType::None => "new"
        };
        "{".to_string() 
            + "\"u\":" + self.0.to_string().as_str() + "," 
            + "\"mt\":\"" + mmr_type + "\","
            + "\"m\":" + mmr.to_string().as_str() + "}"
    }
}

impl std::string::ToString for types::TeamMMR {
    fn to_string(&self) -> String {
        
        "[".to_string() + self.0.clone().into_iter().map(|obj| obj.to_string()).collect::<Vec<String>>().join(",").as_str()
            + "]"
    }
}

impl std::string::ToString for MMRType {
    fn to_string(&self) -> String {
        match self {
            MMRType::MMR(data) => data.to_string(),
            MMRType::NotEnought(data) => data.to_string(),
            _ => 0.to_string()
        }
    }
}

pub async fn write_session(
    session_memory: memory::SessionMemory,
    win_mmr: types::TeamMMR,
    lose_mmr: types::TeamMMR,
    team_1_res: bool,
    team_2_res: bool,
    session_file: Arc<Mutex<BufWriter<File>>>
){
    
    let session_id = session_memory.now_session_id;

    for user_t1 in win_mmr.0.iter() {
        let str: String = session_id.to_string() + ";"
            + user_t1.0.to_string().as_str() + ";"
            + "1;" + team_1_res.to_string().as_str() + ";"
            + match user_t1.1 {
                MMRType::MMR(_) => "mmr",
                MMRType::NotEnought(_) => "not_enought",
                MMRType::None => "new"
            } + ";"
            + (match user_t1.1 {
                MMRType::MMR(mmr) => mmr,
                _ => 0
            }).to_string().as_str() + "\n";
        
        session_file.lock().await.write_all(str.as_bytes()).await.unwrap();
    }
    for user_t2 in lose_mmr.0.iter() {
        let str: String = session_id.to_string() + ";"
            + user_t2.0.to_string().as_str() + ";"
            + "2;" + team_2_res.to_string().as_str() + ";"
            + match user_t2.1 {
                MMRType::MMR(_) => "mmr",
                MMRType::NotEnought(_) => "not_enought",
                MMRType::None => "new"
            } + ";"
            + (match user_t2.1 {
                MMRType::MMR(mmr) => mmr,
                _ => 0
            }).to_string().as_str() + "\n";
        
        session_file.lock().await.write_all(str.as_bytes()).await.unwrap();
    }

}

pub async fn write_change(
    change: LeaderboardChangeV1,
    diff_mmr: i32,
    user_row: LeaderboardRow,
    change_file: &mut BufWriter<File>,
    change_debug: MMRChangeDebug
){
    
    let session_str: String = "{".to_string()
    + "u:" + change.user_id.to_string().as_str()
    + ",v:" + change.victory.to_string().as_str()
    + ",dm:" + diff_mmr.to_string().as_str()
    + ",m:" + user_row.mmr.to_string().as_str()
    + ",o:" + change.top_3.iter().map(|o| o.to_string()).collect::<Vec<String>>().join(",").to_string().as_str()
    + ",t:" + user_row.top_20.to_string().as_str()
    + ",e:" + user_row.early_quites.to_string().as_str()
    + ",bs:" + change.battle_score.to_string().as_str()
    + ",bsm:" + change.battle_score_muld.to_string().as_str()
    + ",de:[" + change_debug.0.to_string().as_str() + ","
    + change_debug.1.to_string().as_str() + ","
    + change_debug.2.to_string().as_str() + ","
    + change_debug.3.to_string().as_str() + ","
    + change_debug.4.to_string().as_str() + ","
    + change_debug.5.to_string().as_str() + "]"
    + "}\n";

    change_file.write_all(session_str.as_bytes()).await.unwrap();
}

pub async fn write_change_v2(
    change: LeaderboardChangeV2,
    diff_mmr: i32,
    user_row: LeaderboardRow,
    change_file: &mut BufWriter<File>,
    change_debug: MMRChangeDebugV2
){
    
    let session_str: String = "{".to_string()
    + "u:" + change.user_id.to_string().as_str()
    + ",v:" + change.victory.to_string().as_str()
    + ",dm:" + diff_mmr.to_string().as_str()
    + ",m:" + user_row.mmr.to_string().as_str()
    + ",o:" + change.top_3.iter().map(|o| (((o.0.get() as f64) * o.1) as u64).to_string()).collect::<Vec<String>>().join("+").to_string().as_str()
    + (if change.top_20 {",t:".to_string() + (1).to_string().as_str() }else {"".to_string()}).as_str() 
    + (if change.early_quite {",e:".to_string() + (1).to_string().as_str() }else {"".to_string()}).as_str() 
    + ",bs:" + change.battle_score.to_string().as_str()
    + ",bsm:" + change.battle_score_muld.to_string().as_str()
     + ",de:[im:" + (((change_debug.0 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",dm:"
    + (((change_debug.1 * 10.0) as u32) as f64 / 10.0).to_string().as_str() + ",po:"
    + (((change_debug.2 * 1.0) as u32) as f64 / 1.0).to_string().as_str() + ",ip:"
    + (((change_debug.3 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",ik"
    + (((change_debug.4 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",dk"
    + (((change_debug.5 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",sk"
    + (((change_debug.6 * 1.0) as u32) as f64 / 1.0).to_string().as_str() + ",bi"
    + (((change_debug.7 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",bd"
    + (((change_debug.8 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",k"
    + (((change_debug.9 * 100.0) as u32) as f64 / 100.0).to_string().as_str() + ",a"
    + (((change_debug.10 * 10.0) as u32) as f64 / 10.0).to_string().as_str() + "]" 
    + "}\n";

    change_file.write_all(session_str.as_bytes()).await.unwrap();
}