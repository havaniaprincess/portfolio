use std::collections::{BTreeMap, HashMap};
use rand::Rng;
use mmr_libs::{memory::SessionMemory, types::{Leaderboard, LeaderboardRow, LeaderboardV2, UserBattleRow}};

macro_rules! leaderboard_row {
    ($uid:expr, $mmr:expr, $battle:expr) => {
        LeaderboardRow{
            id: $uid as u64, 
            mmr: $mmr,
            battle_score: $battle * 1600,
            battles: $battle,
            victories: (0.5 * $battle as f64) as u32,
            early_quites: 0,
            top_20: 0,
            last_session:0
        }
    };
}

macro_rules! user_battle_row {
    ($uid:expr, $session_id:expr, $team:expr, $battle_score:expr, $victory:expr, $top_20:expr, $early_quit:expr, $faction:expr) => {
        UserBattleRow{
            id: $uid as u64, 
            session_id: $session_id,
            commit_time: 0,
            team: $team,
            battle_score: $battle_score,
            victories: $victory,
            team_score_top_20_percent: $top_20,
            early_quit: $early_quit,
            faction: $faction
        }
    };
}

pub fn generate_v1_leaderboard(players: &[(u64, u32, u32)]) -> Leaderboard {
    let mut leaderboard = Leaderboard{users: HashMap::new(), sets: Vec::new(), battle_score_hash: BTreeMap::new(), battle_faction_hash: HashMap::new()};
    for (uid, mmr, battles) in players.iter() {
        leaderboard.users.insert(*uid, leaderboard_row!(*uid, *mmr, *battles));
        leaderboard.battle_score_hash.insert((1600 as u32, *uid), *mmr);
    }
    leaderboard
}

pub fn generate_v2_leaderboard(players: &[(u64, u32, u32)]) -> LeaderboardV2 {
    let mut leaderboard = LeaderboardV2{users: HashMap::new(), sets: Vec::new(), battle_score_hash: BTreeMap::new(), battle_faction_hash: HashMap::new()};
    for (uid, mmr, battles) in players.iter() {
        leaderboard.users.insert(*uid, leaderboard_row!(*uid, *mmr, *battles));
        leaderboard.battle_score_hash.insert((1600 as u32, *uid), *mmr);
    }
    leaderboard
}

// uid, team, score, victory, top_20, equite, faction
pub fn generate_v1_session(session_id: u64, players: &[(u64, u8, u32, bool, bool, bool, String)]) -> SessionMemory {
    let mut session = SessionMemory{now_session_id: session_id, rows: Vec::new()};
    for (uid, team, score, victory, top_20, early_quite, faction) in players.iter() {
        session.rows.push(user_battle_row!(*uid, session_id, *team, *score, *victory, *top_20, *early_quite, faction.to_string()));
    }
    session
}

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
