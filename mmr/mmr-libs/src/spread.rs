use crate::types::{LeaderboardMark, LeaderboardRow};

/// Builds an MMR distribution bucketed by dominant faction and MMR range.
///
/// Iterates over `users_rows`, filters by `filter_battle` (minimum total battles) and
/// `filter_time` (minimum `last_session` timestamp), then classifies each user into one of
/// three faction categories:
/// - `"faction_1"` — played ≥ 65 % of battles as faction 1
/// - `"faction_2"` — played ≥ 65 % of battles as faction 2
/// - `"mixed"`     — no dominant faction
///
/// Players are grouped into fixed-width MMR buckets of size `mmr_dist`
/// (bucket key = `(mmr / mmr_dist) * mmr_dist`).
///
/// Returns a `HashMap<(faction, mmr_bucket), (player_count, total_mmr_sum)>`.
pub fn mmr_spread<T>(
    leaderboard: &T,
    users_rows: &std::collections::HashMap<u64, LeaderboardRow>,
    mmr_dist: u32,
    filter_battle: u32,
    filter_time: u64
) -> std::collections::HashMap<(String, u32), (u64, u64)>
where 
    T: LeaderboardMark
{
    let mut spread: std::collections::HashMap<(String, u32), (u64, u64)> = std::collections::HashMap::new();

    // Filter users by minimum battles and recent activity.
    for (&user_id, user_row) in users_rows.iter().filter(|obj| if obj.1.battles >= filter_battle && obj.1.last_session >= filter_time {true} else {false}) {
        let faction_1_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_1".to_string())) {
            Some(&battles) => battles,
            None => 0
        };
        let faction_2_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_2".to_string())) {
            Some(&battles) => battles,
            None => 0
        };

        // Determine dominant faction for the user (>=65% of battles), otherwise mark as mixed.
        let campain_main = if ((faction_1_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_1".to_string()
        } else if ((faction_2_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_2".to_string()
        } else {
            "mixed".to_string()
        };
        // Snap to the lower bound of the fixed-width MMR bucket.
        let mmr_group = (user_row.mmr / mmr_dist) * mmr_dist;
        let mut spread_row = match spread.get(&(campain_main.clone(), mmr_group)) {
            Some(dd) => *dd,
            None => (0, 0 as u64)
        };
        // Accumulate player count and total MMR for this bucket.
        spread_row.0 += 1;
        spread_row.1 += user_row.mmr as u64;

        spread.insert((campain_main, mmr_group), spread_row);
    }
    spread
}


/// Builds a battle-count distribution bucketed by dominant faction and battles range.
///
/// Same filtering and faction-classification logic as [`mmr_spread`], but groups players
/// into fixed-width battle-count buckets of size `battle_dist`
/// (bucket key = `(battles / battle_dist) * battle_dist`).
///
/// Returns a `HashMap<(faction, battles_bucket), (player_count, total_mmr_sum)>`.
pub fn battle_spread<T>(
    leaderboard: &T,
    users_rows: &std::collections::HashMap<u64, LeaderboardRow>,
    battle_dist: u32,
    filter_battle: u32,
    filter_time: u64
) -> std::collections::HashMap<(String, u32), (u64, u64)>
where 
    T: LeaderboardMark
{
    let mut spread: std::collections::HashMap<(String, u32), (u64, u64)> = std::collections::HashMap::new();

    // Filter users by minimum battles and recent activity.
    for (&user_id, user_row) in users_rows.iter().filter(|obj| if obj.1.battles >= filter_battle && obj.1.last_session >= filter_time {true} else {false}) {
        let faction_1_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_1".to_string())) {
            Some(&battles) => battles,
            None => 0
        };
        let faction_2_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_2".to_string())) {
            Some(&battles) => battles,
            None => 0
        };

        // Determine dominant faction for the user (>=65% of battles), otherwise mark as mixed.
        let campain_main = if ((faction_1_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_1".to_string()
        } else if ((faction_2_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_2".to_string()
        } else {
            "mixed".to_string()
        };
        // Snap to the lower bound of the fixed-width battles bucket.
        let battle_group = (user_row.battles / battle_dist) * battle_dist;
        let mut spread_row = match spread.get(&(campain_main.clone(), battle_group)) {
            Some(dd) => *dd,
            None => (0, 0 as u64)
        };
        // Accumulate player count and total MMR for this bucket.
        spread_row.0 += 1;
        spread_row.1 += user_row.mmr as u64;
        spread.insert((campain_main, battle_group), spread_row);
    }
    spread
}


/// Builds a country-aware MMR distribution bucketed by dominant faction, country and MMR range.
///
/// Extends [`mmr_spread`] with a per-country dimension. Applies the same `filter_battle` /
/// `filter_time` filters and faction-classification logic, then further partitions each
/// bucket by the player's country from `user_country`. Players with no known country entry
/// are silently skipped.
///
/// Returns a `HashMap<(faction, country, mmr_bucket), (player_count, total_mmr_sum)>`.
pub fn country_spread<T>(
    leaderboard: &T,
    users_rows: &std::collections::HashMap<u64, LeaderboardRow>,
    user_country: &std::collections::HashMap<u64, String>,
    mmr_dist: u32,
    filter_battle: u32,
    filter_time: u64
) -> std::collections::HashMap<(String, String, u32), (u64, u64)>
where 
    T: LeaderboardMark
{
    let mut spread: std::collections::HashMap<(String, String, u32), (u64, u64)> = std::collections::HashMap::new();

    // Filter users by minimum battles and recent activity.
    for (&user_id, user_row) in users_rows.iter().filter(|obj| if obj.1.battles >= filter_battle && obj.1.last_session >= filter_time {true} else {false}) {
        let faction_1_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_1".to_string())) {
            Some(&battles) => battles,
            None => 0
        };
        let faction_2_battles = match leaderboard.get_battle_faction_hash().get(&(user_id, "faction_2".to_string())) {
            Some(&battles) => battles,
            None => 0
        };

        // Determine dominant faction for the user (>=65% of battles), otherwise mark as mixed.
        let campain_main = if ((faction_1_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_1".to_string()
        } else if ((faction_2_battles) as f64) / (user_row.battles as f64) >= 0.65 {
            "faction_2".to_string()
        } else {
            "mixed".to_string()
        };
        // Snap to the lower bound of the fixed-width MMR bucket and resolve country.
        let mmr_group = (user_row.mmr / mmr_dist) * mmr_dist;
        let country = user_country.get(&user_id);
        // Skip users with no known country.
        if country == None {
            continue;
        }
        let mut spread_row = match spread.get(&(campain_main.clone(), country.unwrap().to_string(), mmr_group)) {
            Some(dd) => *dd,
            None => (0, 0 as u64)
        };
        // Accumulate player count and total MMR for this bucket.
        spread_row.0 += 1;
        spread_row.1 += user_row.mmr as u64;
        spread.insert((campain_main, country.unwrap().to_string(), mmr_group), spread_row);
    }
    spread
}