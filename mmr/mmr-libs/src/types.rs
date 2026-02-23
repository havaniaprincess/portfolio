#[derive(Clone, Debug)]
pub struct MMRPair(pub u64, pub MMRType, pub UserBattleRow);

#[derive(Clone, Debug)]
pub struct MMRAgg(pub i64, pub i64);



#[derive(Clone, Debug)]
pub struct TeamMMR(pub Vec<MMRPair>);

#[derive(Clone, Debug)]
pub struct MMRChangeDebug(pub i32, pub i32, pub i32, pub i32, pub f64, pub f64);

#[derive(Clone, Debug)]
pub struct MMRChangeDebugV2(pub f64, pub f64, pub f64, pub f64, pub f64, pub f64, pub f64, pub f64, pub f64, pub f64, pub f64);



#[derive(Clone, Debug)]
pub struct TeamMMRV2(pub Vec<(u64, MMRType, UserBattleRow, f64)>);


#[derive(Clone, Debug)]
pub enum MMRType{
    MMR(u32),
    NotEnought(u32),
    None
}

impl MMRType {
    pub fn get(&self) -> u32 {
        match self {
            MMRType::MMR(m) => *m,
            MMRType::NotEnought(m) => *m,
            MMRType::None => 0
        }
    }
}


#[derive(Clone, Debug)]
pub struct Leaderboard{
    pub users: std::collections::HashMap<u64, LeaderboardRow>,
    pub sets: Vec<LeaderboardChangeV1>,
    pub battle_score_hash: std::collections::BTreeMap<(u32, u64), u32>,
    pub battle_faction_hash: std::collections::HashMap<(u64, String), u64>
}


#[derive(Clone, Debug)]
pub struct LeaderboardChangeV1 {
    pub user_id: u64,
    pub mmr: MMRType,
    pub top_3: Vec<MMRType>,
    pub victory: bool,
    pub early_quite: bool,
    pub top_20: bool,
    pub battle_score: u32,
    pub battle_score_muld: u32,
    pub faction: String,
    pub last_session: u64
}

#[derive(Clone, PartialEq, Debug)]
pub struct LeaderboardRow {
    pub user_id: u64,
    pub mmr: u32,
    pub battles: u32,
    pub victories: u32,
    pub early_quites: u32,
    pub top_20: u32,
    pub battle_score: u32,
    pub last_session: u64
}

#[derive(Clone, Debug)]
pub struct UserBattleRow{
    pub user_id: u64,
    pub session_id: u64,
    pub commit_time: u64,
    pub team: u8,
    pub battle_score: u32,
    pub victories: bool,
    pub early_quit: bool,
    pub team_score_top_20_percent: bool,
    pub faction: String
}


#[derive(Clone)]
pub struct LeaderboardV2{
    pub users: std::collections::HashMap<u64, LeaderboardRow>,
    pub sets: Vec<LeaderboardChangeV2>,
    pub battle_score_hash: std::collections::BTreeMap<(u32, u64), u32>,
    pub battle_faction_hash: std::collections::HashMap<(u64, String), u64>
}


#[derive(Clone, Debug)]
pub struct LeaderboardChangeV2 {
    pub user_id: u64,
    pub mmr: MMRType,
    pub top_3: Vec<(MMRType, f64)>,
    pub victory: bool,
    pub early_quite: bool,
    pub top_20: bool,
    pub battle_score: u32,
    pub battle_score_muld: u32,
    pub faction: String,
    pub last_session: u64
}

pub trait LeaderboardMark {
    fn get_battle_faction_hash(&self) -> &std::collections::HashMap<(u64, String), u64>;
}

impl LeaderboardMark for Leaderboard {
    fn get_battle_faction_hash(&self) -> &std::collections::HashMap<(u64, String), u64> {
        &self.battle_faction_hash
    }
}
impl LeaderboardMark for LeaderboardV2 {
    fn get_battle_faction_hash(&self) -> &std::collections::HashMap<(u64, String), u64> {
        &self.battle_faction_hash
    }
}