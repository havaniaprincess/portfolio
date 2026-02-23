use std::path::Path;
use tokio::io::AsyncWriteExt;

use std::time::{Duration, Instant};

use tokio::io::BufWriter;

use crate::datasets::{Registrations, SessionMode};
use crate::math::{divide_or_0, max, maxf, minf, sigmoid};
use crate::memory::{read_lines, SessionMemory};
use crate::reader::reader;
use crate::statistic::proc_statistic;
use crate::types::{LeaderboardChangeV2, LeaderboardRow, LeaderboardV2, MMRAgg, MMRChangeDebugV2, MMRPair, MMRType, TeamMMR, TeamMMRV2, UserBattleRow};
use crate::{math, statistic::Statistic};




impl LeaderboardV2 {
    
    /// Constructs a `LeaderboardV2` by restoring persisted state from disk.
    ///
    /// Reads two files when they exist:
    /// - `data/leaderboard_v2/base` — core leaderboard rows (`LeaderboardRow` per user).
    /// - `data/leaderboard_v2/battle_faction` — per-user battle counts grouped by faction.
    ///
    /// Players with fewer than 6 battles are loaded but excluded from the `battle_score_hash`
    /// lookup index. Returns an empty leaderboard when the files are absent.
    pub fn new() -> Self {
        // Restore persisted leaderboard state (users + faction counters).
        let mut users: std::collections::HashMap<u64, LeaderboardRow> = std::collections::HashMap::new();
        let mut battle_scores: std::collections::BTreeMap<(u32, u64), u32> =  std::collections::BTreeMap::new();
        if Path::new("data/leaderboard_v2/base").exists() {
            if let Ok(lines) = read_lines("data/leaderboard_v2/base") {
                for line in lines.flatten() {
                    match LeaderboardRow::parse_file(line.replace("\"", "")) {
                        Some(row) => {
                            users.insert(row.user_id, row.clone());
                            // Keep index only for calibrated players.
                            if row.battles >= 6 {
                                battle_scores.insert((row.battle_score / row.battles, row.user_id), row.mmr);
                            }
                        },
                        None => {
                            
                        }
                    };
                }
            }
        }
        let mut battle_faction: std::collections::HashMap<(u64, String), u64> = std::collections::HashMap::new();
        if Path::new("data/leaderboard_v2/battle_faction").exists() {
            if let Ok(lines) = read_lines("data/leaderboard_v2/battle_faction") {
                for line in lines.flatten() {
                    let hash_line = reader(&line);
                
                    let user_id = hash_line.get("user_id");
                    let faction = hash_line.get("faction");
                    let battles = hash_line.get("battles");
                    
                    if user_id == None || faction == None || battles == None {
                        continue;
                    }

                    battle_faction.insert((user_id.unwrap().parse::<u64>().unwrap(), faction.unwrap().to_string()), battles.unwrap().parse::<u64>().unwrap());
                }
            }
        }

        return Self {
            users: users,
            sets: Vec::new(),
            battle_score_hash: battle_scores,
            battle_faction_hash: battle_faction
        };
    }

    /// Persists the current in-memory leaderboard state to disk.
    ///
    /// Writes two files:
    /// - `data/leaderboard_v2/base` — one serialized `LeaderboardRow` per line.
    /// - `data/leaderboard_v2/battle_faction` — per-user faction battle counters in
    ///   `user_id:<id>,faction:<name>,battles:<n>` format.
    pub async fn write(&self){

        // Persist leaderboard rows.

        let data_file = tokio::fs::File::create("data/leaderboard_v2/base".to_string()).await.unwrap();
        let mut data_file = BufWriter::new(data_file);
        // Write contents to the file

        for (_user_id, row) in self.users.clone().iter() {
            data_file.write_all((row.to_string() + "\n").as_bytes()).await.unwrap();
        }
        data_file.flush().await.unwrap();

        
        // Persist per-user battle counters split by faction/mode.
        let data_file = tokio::fs::File::create("data/leaderboard_v2/battle_faction".to_string()).await.unwrap();
        let mut data_file = BufWriter::new(data_file);
        // Write contents to the file

        for (user_id, row) in self.battle_faction_hash.iter() {
            let str = "user_id:".to_string() + user_id.0.to_string().as_str()
                + ",faction:" + user_id.1.as_str()
                + ",battles:" + row.to_string().as_str();
            data_file.write_all((str + "\n").as_bytes()).await.unwrap();
        }
        data_file.flush().await.unwrap();
    }

    /// Returns the current `MMRType` for `user_id`.
    ///
    /// - `MMRType::MMR(mmr)`         — fully calibrated (6+ battles).
    /// - `MMRType::NotEnought(mmr)`  — provisional value (1–5 battles).
    /// - `MMRType::None`             — user has never appeared in a session.
    pub fn get_mmr(&self, user_id: u64) -> MMRType {
        // NotEnought means provisional (insufficient battles for calibrated MMR).
        match self.users.get(&user_id) {
            Some(user_row) => {
                if user_row.battles <= 5 {
                    MMRType::NotEnought(user_row.mmr)
                } else {
                    MMRType::MMR(user_row.mmr)
                }
            },
            None => {
                MMRType::None
            }
        }
    }

    /// Returns the total battle count for `user_id`, or `0` if unknown.
    ///
    /// Used to compute the confidence coefficient $k = (\sqrt{2})^{\min(0,\,b-6)}$
    /// that scales a player's contribution to the MMR pool.
    pub fn get_battles(&self, user_id: u64) -> u32 {
        // Convenience helper used in confidence/scaling terms.
        match self.users.get(&user_id) {
            Some(user_row) => {
                user_row.battles
            },
            None => {
                0
            }
        }
    }

    /// Processes a completed session and updates the leaderboard using the v2 pool algorithm.
    ///
    /// Executes the full pipeline in four timed stages:
    /// 1. **Prepare** — splits `session_memory` into team_1 / team_2 snapshots and builds
    ///    `teams_common_mmr` with per-player confidence coefficients
    ///    $k = (\sqrt{2})^{\min(0,\,battles-6)}$. Sessions with fewer than 5 players per
    ///    side or in the `newbie_common` mode are skipped (`None` returned).
    /// 2. **Write statistics** — when `cl_id > 0`, computes top-3 MMR averages, emits
    ///    team-disbalance flags to `sender_session_class`, and sends `Statistic` payloads
    ///    for the `common` board and all mode-specific boards via `sender`. Forwards
    ///    `(win_avg_mmr, lose_avg_mmr)` to `sender_check`.
    /// 3. **Prepare changes** — computes the shared MMR pool, per-player increase/decrease
    ///    coefficients, and `bank_give`/`bank_get` sigmoid redistribution terms.
    /// 4. **Apply changes** — calls [`set_change`] for every player to apply the computed
    ///    delta: `inc_mmr - dec_mmr`.
    ///
    /// Returns four `(Instant, Duration)` tuples — one per stage — for caller-side profiling,
    /// or `None` if the session was skipped.
    pub async fn proc_session(
        &mut self,
        session_memory: SessionMemory,
        cl_id: u16,
        sender: flume::Sender<(String, Statistic)>,
        session_mode: &SessionMode,
        registrations: &Registrations,
        sender_tasks: flume::Sender<(LeaderboardChangeV2, i32, LeaderboardRow, MMRChangeDebugV2, u16)>, 
        sender_check: flume::Sender<(u32, u32)>,
        sender_session_class: flume::Sender<(u64, bool, bool, bool, bool)>,        
    ) -> Option<((Instant, Duration), (Instant, Duration), (Instant, Duration), (Instant, Duration))> {

        // 1) Build session teams and weighted player descriptors.
        let prepear_session = Instant::now();

        let team_1: SessionMemory = SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_1".to_string() {Some(item)} else {None}).collect() 
        };
        let team_2: SessionMemory = SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_2".to_string() {Some(item)} else {None}).collect() 
        };
        
        // The 4th tuple item is a confidence-like coefficient based on battles count.
        let mut teams_common_mmr: TeamMMRV2 = TeamMMRV2(session_memory.rows.clone().into_iter().map(|user| (user.user_id, self.get_mmr(user.user_id), user.clone(), (2.0 as f64).sqrt().powf(minf(0.0, self.get_battles(user.user_id) as f64 - 6.0)))).collect());
        teams_common_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        teams_common_mmr.0.reverse();

        
        let mode_type = match session_mode.0.get(&session_memory.now_session_id) {
            Some((_mode_0, mode_1, _mode_2)) => {
                mode_1.clone()
            },
            None => "newbie_common".to_string()
        };

        // Skip incomplete sessions and newbie mode for MMR updates.
        if team_1.rows.len() < 5 || team_2.rows.len() < 5 || mode_type == "newbie_common".to_string() {
            return None;
        }
        let prepear_session_time = prepear_session.elapsed();

        let team_1_res = team_1.rows[0].victories;
        let team_2_res = team_2.rows[0].victories;
        let mut team_1_mmr: TeamMMR = TeamMMR(team_1.rows.clone().into_iter().map(|user| MMRPair(user.user_id, self.get_mmr(user.user_id), user.clone())).collect());
        team_1_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        team_1_mmr.0.reverse();
        let mut team_2_mmr: TeamMMR = TeamMMR(team_2.rows.clone().into_iter().map(|user| MMRPair(user.user_id, self.get_mmr(user.user_id), user.clone())).collect());
        team_2_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        team_2_mmr.0.reverse();
        let write_session = Instant::now();
        if cl_id > 0 {
            // 2) Emit statistics + session disbalance classification.
            let team_1_top3d: MMRAgg = team_1_mmr.0.clone().into_iter().enumerate().filter_map(|(idx, obj)| if idx < 3 {Some(obj.1)} else {None}).fold(
                MMRAgg(0, 0),
                |left, right| {
                    let mut res = left.clone();
                    match right {
                        MMRType::MMR(mmr) => {
                            res.0 += 1;
                            res.1 += mmr as i64;
                            res
                        },
                        _ => res
                    }
                }
            );
            let team_1_top3 = if team_1_top3d.0 == 0 {
                None
            } else {
                Some((team_1_top3d.1 / team_1_top3d.0) as u32)
            };
            let team_2_top3d: MMRAgg = team_2_mmr.0.clone().into_iter().enumerate().filter_map(|(idx, obj)| if idx < 3 {Some(obj.1)} else {None}).fold(
                MMRAgg(0, 0),
                |left, right| {
                    let mut res = left.clone();
                    match right {
                        MMRType::MMR(mmr) => {
                            res.0 += 1;
                            res.1 += mmr as i64;
                            res
                        },
                        _ => res
                    }
                }
            );
            let team_2_top3 = if team_2_top3d.0 == 0 {
                None
            } else {
                Some((team_2_top3d.1 / team_2_top3d.0) as u32)
            };
            // Flag sessions where team 1 top-3 average MMR greatly exceeds team 2.
            let disbalance_team_1 = match team_1_top3 {
                Some(team_1) => {
                    match team_2_top3 {
                        Some(team_2) => {
                            if (team_1 as i32) - (team_2 as i32) > 800 {
                                true
                            } else {
                                false
                            }
                        },
                        _ => false
                    }
                },
                _ => {
                    false
                }
            };
            
            // Flag sessions where team 2 top-3 average MMR is much higher.
            let disbalance_team_2 = match team_1_top3 {
                Some(team_1) => {
                    match team_2_top3 {
                        Some(team_2) => {
                            if (team_2 as i32) - (team_1 as i32) > 800 {
                                true
                            } else {
                                false
                            }
                        },
                        _ => false
                    }
                },
                _ => {
                    false
                }
            };
            sender_session_class
                .send((session_memory.now_session_id, disbalance_team_1, disbalance_team_2, team_1_res, team_2_res))
                .unwrap(); 
            sender
                .send(proc_statistic(
                    "common".to_string(),
                    &team_1_mmr,
                    &team_2_mmr,
                    team_1_res,
                    team_2_res,
                    &registrations,
                    Some(sender_check)
                ).await)
                .unwrap();  
    
            match session_mode.0.get(&session_memory.now_session_id) {
                Some((mode_0, mode_1, mode_2)) => {
                    sender
                        .send(proc_statistic(
                            mode_0.to_string(),
                            &team_1_mmr,
                            &team_2_mmr,
                            team_1_res,
                            team_2_res,
                            &registrations,
                            None
                        ).await)
                        .unwrap();
                    sender
                        .send(proc_statistic(
                            mode_1.to_string(),
                            &team_1_mmr,
                            &team_2_mmr,
                            team_1_res,
                            team_2_res,
                            &registrations,
                            None
                        ).await)
                        .unwrap();
                    sender
                        .send(proc_statistic(
                            mode_2.to_string(),
                            &team_1_mmr,
                            &team_2_mmr,
                            team_1_res,
                            team_2_res,
                            &registrations,
                            None
                        ).await)
                        .unwrap();
                },
                None => {}
            } 
        }
    
        let write_session_time = write_session.elapsed();


        let prepear_change = Instant::now();
        // 3) Build redistribution inputs for MMR pool calculation.
        let avg_mmr = teams_common_mmr.0.iter().fold(
            (0.0 as f64, 0.0 as f64, 0.0 as f64), 
            |base, other| {
                let mmr = match other.1 {
                    MMRType::None => 0,
                    MMRType::NotEnought(m) => m,
                    MMRType::MMR(m) => m
                };


                (base.0 + (mmr as f64) * other.3, base.1 + other.3, base.2 + max(other.2.battle_score as i64 + if other.2.victories {(other.2.battle_score as i64) / 4} else {0} + if other.2.team_score_top_20_percent {(other.2.battle_score as i64) / 4} else {0} + (if other.2.early_quit {-(other.2.battle_score as i64) / 2} else {0} as i64), 0) as f64)
            }
        );
        let (avg_mmr, sum_score) = (avg_mmr.0 / avg_mmr.1, avg_mmr.2);
        // user_id -> (mmr, weighted_score, k, decrease_k, increase_k, bank_get, bank_give)
        let mmr_poll_diff: std::collections::BTreeMap<u64, (u32, u32, f64, f64, f64, f64, f64)> = teams_common_mmr.0.iter().map(|(user_id, mmr_player, user, k)| {
            let mmr = match mmr_player {
                MMRType::None => 0,
                MMRType::NotEnought(m) => *m,
                MMRType::MMR(m) => *m
            };
            let bank_give = sigmoid(mmr as f64, 0.01, 500.0, -1.0, 1.0) * 1.0 + 0.2 * (1.0 - k);
            let bank_get = sigmoid(mmr as f64, 0.01, 9500.0, 1.0, 0.0) * 0.05;

            (
                *user_id, 
                (
                    mmr, 
                    (
                        max(
                            user.battle_score as i64 
                                + if user.victories {
                                    (user.battle_score as i64) / 4
                                } else {0} 
                                + if user.team_score_top_20_percent {
                                    (user.battle_score as i64) / 4
                                } else {0} 
                                + (if user.early_quit {-(user.battle_score as i64) / 2} else {0} as i64), 
                            0)
                    ) as u32, 
                    *k, 
                    0.5 / (8.0 * divide_or_0(avg_mmr, mmr as f64).powf(0.35) + 1.0), 
                    0.5 / (4.0 * divide_or_0(mmr as f64, avg_mmr) + 1.0), 
                    bank_get, 
                    bank_give
                )
            )
            
        }).collect();

        // Aggregate global pool/increase normalization terms for all players in session.
        let (mmr_pool, mmr_inc, normalization_k) = mmr_poll_diff.iter().fold((0.0, 0.0, 0.0), |base, (_user_id, (mmr, battle_score, k, dec_k, inc_k, bank_get, bank_give))| {
            (base.0 + (dec_k + bank_get) * (*mmr as f64) * k +  bank_give * maxf(avg_mmr, 500.0) - bank_get * (*mmr as f64), base.1 + inc_k / k, base.2 + (inc_k / k) * (*battle_score as f64) )
        });
        let prepear_change_time = prepear_change.elapsed();
    
        let set_change = Instant::now();
        // 4) Compute per-user delta and apply to persistent leaderboard state.
        for (user_id, _mmr, user, _k) in teams_common_mmr.0.iter() {
            let (mmr, score, k, dec_k, inc_k, bd, bi) = match mmr_poll_diff.get(user_id) {
                Some(mdiff) => mdiff,
                None => panic!()
            };
            let inc_mmr = (mmr_pool) * ((*score as f64) / sum_score) * ((inc_k) / mmr_inc) * 1.0 / (normalization_k / (sum_score * mmr_inc)) / k;
            let dec_mmr = (*mmr as f64) * (dec_k + bd) / k;
            let change = LeaderboardChangeV2{
                user_id: *user_id,
                mmr: self.get_mmr(*user_id),
                top_3: teams_common_mmr.0.iter().map(|obj| (obj.1.clone(), obj.3)).collect::<Vec<(MMRType, f64)>>(),
                victory: user.victories,
                early_quite: user.early_quit,
                top_20: user.team_score_top_20_percent,
                battle_score: user.battle_score,
                battle_score_muld: *score,
                faction: user.faction.clone(),
                last_session: user.commit_time
            };
            self.set_change(user_id, user, ((inc_mmr - dec_mmr)) as i32, sender_tasks.clone(), cl_id, change, MMRChangeDebugV2(inc_mmr, dec_mmr, mmr_pool, mmr_inc, *inc_k, *dec_k, sum_score, *bi, *bd, *k, avg_mmr));
        }
        let set_change_time = set_change.elapsed();
        Some(((prepear_session, prepear_session_time), (write_session, write_session_time), (prepear_change, prepear_change_time), (prepear_change, set_change_time)))

        
    }

    /// Processes a completed session without emitting statistics (debug/inspection variant).
    ///
    /// Mirrors the full pool-based MMR calculation of [`proc_session`] but skips all
    /// statistic and session-classification side effects. Uses a hardcoded `mode_type`
    /// of `"low_br"` so no session is filtered as `newbie_common`. Calls
    /// [`set_change_lite`] for each player, which prints detailed debug output to stdout.
    ///
    /// Always returns `None` — timing data is tracked internally but not exposed.
    pub async fn proc_session_lite(
        &mut self,
        session_memory: SessionMemory,
        
    ) -> Option<((Instant, Duration), (Instant, Duration), (Instant, Duration), (Instant, Duration))> {

        // Lite variant: same MMR math without external statistic/classification side effects.
        let prepear_session = Instant::now();

        let team_1: SessionMemory = SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_1".to_string() {Some(item)} else {None}).collect() 
        };
        let team_2: SessionMemory = SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_2".to_string() {Some(item)} else {None}).collect() 
        };
        
        let mut teams_common_mmr: TeamMMRV2 = TeamMMRV2(session_memory.rows.clone().into_iter().map(|user| (user.user_id, self.get_mmr(user.user_id), user.clone(), (2.0 as f64).sqrt().powf(minf(0.0, self.get_battles(user.user_id) as f64 - 6.0)))).collect());
        teams_common_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        teams_common_mmr.0.reverse();

        
        // Fixed mode marker in lite mode.
        let mode_type = "low_br".to_string();

        if team_1.rows.len() < 5 || team_2.rows.len() < 5 || mode_type == "newbie_common".to_string() {
            return None;
        }
        let _prepear_session_time = prepear_session.elapsed();

        let _team_1_res = team_1.rows[0].victories;
        let _team_2_res = team_2.rows[0].victories;
        let mut team_1_mmr: TeamMMR = TeamMMR(team_1.rows.clone().into_iter().map(|user| MMRPair(user.user_id, self.get_mmr(user.user_id), user.clone())).collect());
        team_1_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        team_1_mmr.0.reverse();
        let mut team_2_mmr: TeamMMR = TeamMMR(team_2.rows.clone().into_iter().map(|user| MMRPair(user.user_id, self.get_mmr(user.user_id), user.clone())).collect());
        team_2_mmr.0.sort_unstable_by_key(|obj| match obj.1 {
            MMRType::MMR(mmr) => (2, mmr),
            _ => (1, 0)
        });
        team_2_mmr.0.reverse();


        let prepear_change = Instant::now();
        // Recompute pool terms and apply deltas in-place.
        let avg_mmr = teams_common_mmr.0.iter().fold(
            (0.0 as f64, 0.0 as f64, 0.0 as f64), 
            |base, other| {
                let mmr = match other.1 {
                    MMRType::None => 0,
                    MMRType::NotEnought(m) => m,
                    MMRType::MMR(m) => m
                };


                (base.0 + (mmr as f64) * other.3, base.1 + other.3, base.2 + max(other.2.battle_score as i64 + if other.2.victories {(other.2.battle_score as i64) / 4} else {0} + if other.2.team_score_top_20_percent {(other.2.battle_score as i64) / 4} else {0} + (if other.2.early_quit {-(other.2.battle_score as i64) / 2} else {0} as i64), 0) as f64)
            }
        );
        let (avg_mmr, sum_score) = (avg_mmr.0 / avg_mmr.1, avg_mmr.2);
        // user_id -> (mmr, weighted_score, k, decrease_k, increase_k, bank_get, bank_give)
        let mmr_poll_diff: std::collections::BTreeMap<u64, (u32, u32, f64, f64, f64, f64, f64)> = teams_common_mmr.0.iter().map(|(user_id, mmr_player, user, k)| {
            let mmr = match mmr_player {
                MMRType::None => 0,
                MMRType::NotEnought(m) => *m,
                MMRType::MMR(m) => *m
            };
            let bank_give = sigmoid(mmr as f64, 0.01, 500.0, -1.0, 1.0) * 1.0 + 0.2 * (1.0 - k);
            let bank_get = sigmoid(mmr as f64, 0.01, 9500.0, 1.0, 0.0) * 0.05;

            (*user_id, (mmr, (max(user.battle_score as i64 + if user.victories {(user.battle_score as i64) / 4} else {0} + if user.team_score_top_20_percent {(user.battle_score as i64) / 4} else {0} + (if user.early_quit {-(user.battle_score as i64) / 2} else {0} as i64), 0)) as u32, *k, 0.5 / (8.0 * divide_or_0(avg_mmr, mmr as f64).powf(0.35) + 1.0), 0.5 / (4.0 * divide_or_0(mmr as f64, avg_mmr) + 1.0), bank_get, bank_give))
            
        }).collect();

        let (mmr_pool, mmr_inc, normalization_k) = mmr_poll_diff.iter().fold((0.0, 0.0, 0.0), |base, (_user_id, (mmr, battle_score, k, dec_k, inc_k, bank_get, bank_give))| {
            (base.0 + (dec_k + bank_get) * (*mmr as f64) * k +  bank_give * maxf(avg_mmr, 500.0) - bank_get * (*mmr as f64), base.1 + inc_k / k, base.2 + (inc_k / k) * (*battle_score as f64) )
        });
        let _prepear_change_time = prepear_change.elapsed();
    
        let set_change = Instant::now();
        // user_id, (mmr, inc_mmr, dec_mmr, common_mmr)
        for (user_id, _mmr, user, _k) in teams_common_mmr.0.iter() {
            let (mmr, score, k, dec_k, inc_k, bd, bi) = match mmr_poll_diff.get(user_id) {
                Some(mdiff) => mdiff,
                None => panic!()
            };
            let inc_mmr = (mmr_pool) * ((*score as f64) / sum_score) * ((inc_k) / mmr_inc) * 1.0 / (normalization_k / (sum_score * mmr_inc)) / k;
            let dec_mmr = (*mmr as f64) * (dec_k + bd) / k;
            let change = LeaderboardChangeV2{
                user_id: *user_id,
                mmr: self.get_mmr(*user_id),
                top_3: teams_common_mmr.0.iter().map(|obj| (obj.1.clone(), obj.3)).collect::<Vec<(MMRType, f64)>>(),
                victory: user.victories,
                early_quite: user.early_quit,
                top_20: user.team_score_top_20_percent,
                battle_score: user.battle_score,
                battle_score_muld: *score,
                faction: user.faction.clone(),
                last_session: user.commit_time
            };
            self.set_change_lite(user_id, user, ((inc_mmr - dec_mmr)) as i32, change, MMRChangeDebugV2(inc_mmr, dec_mmr, mmr_pool, mmr_inc, *inc_k, *dec_k, sum_score, *bi, *bd, *k, avg_mmr));
        }
        let _set_change_time = set_change.elapsed();
        None

        
    }
    /// Applies a pre-computed MMR delta for a single player to the in-memory leaderboard.
    ///
    /// Updates `battle_faction_hash` for the player's faction, then either:
    /// - **existing user** — applies `diff_mmr` to current MMR (clamped to 0) and
    ///   increments all counters (battles, victories, early-quits, top-20, battle score).
    /// - **new user** — initializes a fresh `LeaderboardRow` from the session contribution.
    fn set_change(&mut self, user_id: &u64, userstat_row: &UserBattleRow, diff_mmr: i32,
        _sender_tasks: flume::Sender<(LeaderboardChangeV2, i32, LeaderboardRow, MMRChangeDebugV2, u16)>, 
        _cl_id: u16, change: LeaderboardChangeV2, _debug: MMRChangeDebugV2
    ){
        // Update faction battle counters for this user.
        match self.battle_faction_hash.get_mut(&(change.user_id, change.faction.clone())) {
            Some(bc) => {
                *bc = *bc + 1;
            },
            None => {self.battle_faction_hash.insert((change.user_id, change.faction.clone()), 1);}
        }
        match self.users.get(user_id) {
            Some(user) => {
                // Existing user: apply MMR delta and accumulate counters.

                self.users.insert(*user_id, LeaderboardRow{
                    user_id: *user_id,
                    mmr: (math::max(user.mmr as i32 + diff_mmr, 0)) as u32,
                    battles: user.battles + 1,
                    victories: user.victories + if userstat_row.victories {1} else {0},
                    early_quites: user.early_quites + if userstat_row.early_quit {1} else {0},
                    top_20: user.top_20 + if userstat_row.team_score_top_20_percent {1} else {0},
                    battle_score: user.battle_score + userstat_row.battle_score,
                    last_session: change.last_session
                });
            },
            None => {
                // New user: initialize row from current session contribution.
                let row = LeaderboardRow{
                    user_id: *user_id,
                    mmr: (math::max(diff_mmr, 0)) as u32,
                    battles: 1,
                    victories: if userstat_row.victories {1} else {0},
                    early_quites: if userstat_row.early_quit {1} else {0},
                    top_20: if userstat_row.team_score_top_20_percent {1} else {0},
                    battle_score: userstat_row.battle_score,
                    last_session: change.last_session
                };
                self.users.insert(*user_id, row);
            }
        }
    }
    /// Debug variant of [`set_change`] that prints the full change payload before applying it.
    ///
    /// Identical update logic to [`set_change`] but prints `userstat_row`, `change`, `debug`
    /// and `diff_mmr` to stdout for inspection. Dead-code sender calls are retained as
    /// commented-out blocks for future use.
    fn set_change_lite(&mut self, user_id: &u64, userstat_row: &UserBattleRow, diff_mmr: i32, change: LeaderboardChangeV2, debug: MMRChangeDebugV2
    ){
        // Lite mode prints detailed debug payload for inspection.
        println!("{:?}\n{:?} == {:?}\n{}", userstat_row, change, debug, diff_mmr);
        match self.battle_faction_hash.get_mut(&(change.user_id, change.faction.clone())) {
            Some(bc) => {
                *bc = *bc + 1;
            },
            None => {self.battle_faction_hash.insert((change.user_id, change.faction.clone()), 1);}
        }
        match self.users.get(user_id) {
            Some(user) => {

                /* let _ = sender_tasks
                .send((
                    change,
                    diff_mmr,
                    user.clone(),
                    debug,
                    cl_id
                )); */
                self.users.insert(*user_id, LeaderboardRow{
                    user_id: *user_id,
                    mmr: (math::max(user.mmr as i32 + diff_mmr, 0)) as u32,
                    battles: user.battles + 1,
                    victories: user.victories + if userstat_row.victories {1} else {0},
                    early_quites: user.early_quites + if userstat_row.early_quit {1} else {0},
                    top_20: user.top_20 + if userstat_row.team_score_top_20_percent {1} else {0},
                    battle_score: user.battle_score + userstat_row.battle_score,
                    last_session: change.last_session
                });
            },
            None => {
                let row = LeaderboardRow{
                    user_id: *user_id,
                    mmr: (math::max(diff_mmr, 0)) as u32,
                    battles: 1,
                    victories: if userstat_row.victories {1} else {0},
                    early_quites: if userstat_row.early_quit {1} else {0},
                    top_20: if userstat_row.team_score_top_20_percent {1} else {0},
                    battle_score: userstat_row.battle_score,
                    last_session: change.last_session
                };
                /* let _ = sender_tasks
                .send((
                    change,
                    diff_mmr,
                    row.clone(),
                    debug,
                    cl_id
                )); */
                self.users.insert(*user_id, row);
            }
        }
    }
}