use std::path::Path;
use std::time::{Duration, Instant};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::memory::read_lines;
use crate::reader::reader;
use crate::types::{Leaderboard, LeaderboardChangeV1, LeaderboardRow, MMRAgg, MMRChangeDebug, MMRPair, MMRType, TeamMMR};
use crate::{math, statistic::{self, Statistic}, memory,datasets};

impl Leaderboard {
    /// Estimates the initial calibrated MMR for a new player who just completed their 6th battle.
    ///
    /// Searches the `battle_score_hash` BTree for players within ±50 of the given average
    /// `score`. If at least 10 neighbors are found their MMR values are averaged; otherwise
    /// the single nearest neighbor is used with a conservative 0.8 downscale factor.
    ///
    /// Returns `None` when there is insufficient history (fewer than 1 000 calibrated users
    /// in the leaderboard or fewer than 10 nearby samples and no nearest neighbor).
    async fn get_mmr_for_new(&self, score: u32) -> Option<u32> {

        // Estimate initial MMR for a player who just reached the calibrated threshold.
        // Uses nearby users by average battle score as reference.

        let mut min_mmr: Option<(u32, u32)> = None;
        let mut sum_100: u32 = 0;
        let mut count_100: u32 = 0;
        let user_count = self.battle_score_hash.len();

        for ((battle_score, _uid), mmr) in self.battle_score_hash.range((math::max((score as i32)-50, 0) as u32, 0)..=(score+50, u64::MAX)) {
            let dist = ((score as i32) - (*battle_score as i32)).abs() as u32;
            if min_mmr == None || dist < min_mmr.unwrap().0 {
                min_mmr = Some((dist, *mmr));
            }
            if dist <= 50 {
                count_100 += 1;
                sum_100 += mmr;
            }

        }
        // Require enough history to avoid noisy estimates.
        if (min_mmr == None && count_100 < 10) || user_count < 1000  {
            return None;
        }
        if count_100 >= 10 {
            // Prefer local average when enough nearby samples are available.
            return Some(sum_100 / (count_100 as u32));
        }
        // Fallback to nearest-neighbor with conservative downscale.
        Some(((min_mmr.unwrap().1 as f64) * 0.8) as u32)
    }
    /// Enmodes a single `LeaderboardChangeV1` into the pending change buffer.
    ///
    /// Changes are buffered and applied in bulk by [`set_changes`] or [`set_changes_lite`].
    pub fn add_change(&mut self, change: LeaderboardChangeV1) {
        self.sets.push(change);
    }

    /// Applies all buffered `LeaderboardChangeV1` entries to the in-memory leaderboard.
    ///
    /// For each change the calibration phase is resolved:
    /// - battles 1–5: provisional MMR is a running average of accumulated battle score.
    /// - battle 6: MMR is bootstrapped from historical neighbors via [`get_mmr_for_new`].
    /// - battles 7+: classic diff-based ELO update via `math::diff_mmr`.
    ///
    /// The `battle_score_hash` index is kept consistent for fully calibrated users.
    /// When `setting_change` is `true` the pending buffer is cleared after processing.
    pub async fn set_changes(&mut self, _cl_id: u16,
        _sender_tasks: flume::Sender<(LeaderboardChangeV1, i32, LeaderboardRow, MMRChangeDebug, u16)>,
        setting_change: bool
    ) {
        // Apply buffered per-user changes produced from processed sessions.
        let changes = self.sets.clone();

        for change in changes.iter() {
            let (diff_mmr, _change_debug) = math::diff_mmr(change.victory, change.battle_score_muld as i32, change.top_3.clone(), change.mmr.clone(), change.early_quite, change.top_20);
            match self.battle_faction_hash.get_mut(&(change.user_id, change.faction.clone())) {
                Some(bc) => {
                    *bc = *bc + 1;
                },
                None => {self.battle_faction_hash.insert((change.user_id, change.faction.clone()), 1);}
            }
            match self.users.get(&change.user_id) {
                Some(user_row) => {
                    // Calibration model:
                    // - up to 5 battles: direct score-based provisional MMR
                    // - at 6th battle: bootstrap against historical neighbors
                    // - after 6 battles: classic diff-based MMR updates
                    let new_mmr = if user_row.battles + 1 == 6 {
                        match self.get_mmr_for_new((user_row.battle_score + change.battle_score) / (user_row.battles + 1)).await {
                            Some(mmr) => mmr,
                            None => (user_row.battle_score + change.battle_score) / (user_row.battles + 1)                           
                        }

                    } else if user_row.battles + 1 > 6 {
                        self.battle_score_hash.remove(&(user_row.battle_score/user_row.battles, change.user_id));
                        (math::max(user_row.mmr as i32 + diff_mmr, 0)) as u32
                    } else {
                        (user_row.battle_score + change.battle_score) / (user_row.battles + 1)
                    };
                    
                    // Keep lookup index only for fully calibrated users.
                    if user_row.battles > 5 {
                        self.battle_score_hash.insert(((user_row.battle_score + change.battle_score)/(user_row.battles + 1), change.user_id), new_mmr);
                    }
                    /* let _ = sender_tasks
                        .send((
                            change.clone(),
                            diff_mmr,
                            user_row.clone(),
                            change_debug,
                            cl_id
                        )); */  

                    self.users.insert(change.user_id, LeaderboardRow{
                        user_id: change.user_id,
                        mmr: new_mmr,
                        battles: user_row.battles + 1,
                        victories: user_row.victories + if change.victory {1} else {0},
                        early_quites: user_row.early_quites + if change.early_quite {1} else {0},
                        top_20: user_row.top_20 + if change.top_20 {1} else {0},
                        battle_score: user_row.battle_score + change.battle_score,
                        last_session: change.last_session
                    });
                },
                None => {
                    // First appearance of a user: initialize from battle score.

                    self.users.insert(change.user_id, LeaderboardRow{
                        user_id: change.user_id,
                        mmr: change.battle_score as u32,
                        battles: 1,
                        victories: if change.victory {1} else {0},
                        early_quites: if change.early_quite {1} else {0},
                        top_20: if change.top_20 {1} else {0},
                        battle_score: change.battle_score,
                        last_session: change.last_session
                    });
                }
            };
        }
        if setting_change {
            // Clear pending buffer only when requested by caller.
            self.sets = Vec::new();
        }
    }

    /// Applies all buffered changes and returns the applied set (debug/inspection variant).
    ///
    /// Identical calibration logic to [`set_changes`] but additionally prints each change
    /// and its computed `diff_mmr` to stdout. Always clears the pending buffer and returns
    /// the full list of applied `LeaderboardChangeV1` records for caller inspection.
    pub async fn set_changes_lite(&mut self) -> Vec<LeaderboardChangeV1> {
        // Lite mode: same updates as set_changes + debug output + returned applied set.
        let changes = self.sets.clone();

        for change in changes.iter() {
            let (diff_mmr, change_debug) = math::diff_mmr(change.victory, change.battle_score_muld as i32, change.top_3.clone(), change.mmr.clone(), change.early_quite, change.top_20);
            println!("{:?} -- {}\n{:?}", change, diff_mmr, change_debug);
            match self.battle_faction_hash.get_mut(&(change.user_id, change.faction.clone())) {
                Some(bc) => {
                    *bc = *bc + 1;
                },
                None => {self.battle_faction_hash.insert((change.user_id, change.faction.clone()), 1);}
            }
            match self.users.get(&change.user_id) {
                Some(user_row) => {
                    let new_mmr = if user_row.battles + 1 == 6 {
                        match self.get_mmr_for_new((user_row.battle_score + change.battle_score) / (user_row.battles + 1)).await {
                            Some(mmr) => mmr,
                            None => (user_row.battle_score + change.battle_score) / (user_row.battles + 1)                           
                        }

                    } else if user_row.battles + 1 > 6 {
                        self.battle_score_hash.remove(&(user_row.battle_score/user_row.battles, change.user_id));
                        (math::max(user_row.mmr as i32 + diff_mmr, 0)) as u32
                    } else {
                        (user_row.battle_score + change.battle_score) / (user_row.battles + 1)
                    };
                    
                    if user_row.battles > 5 {
                        self.battle_score_hash.insert(((user_row.battle_score + change.battle_score)/(user_row.battles + 1), change.user_id), new_mmr);
                    } 

                    self.users.insert(change.user_id, LeaderboardRow{
                        user_id: change.user_id,
                        mmr: new_mmr,
                        battles: user_row.battles + 1,
                        victories: user_row.victories + if change.victory {1} else {0},
                        early_quites: user_row.early_quites + if change.early_quite {1} else {0},
                        top_20: user_row.top_20 + if change.top_20 {1} else {0},
                        battle_score: user_row.battle_score + change.battle_score,
                        last_session: change.last_session
                    });
                },
                None => {
                    

                    self.users.insert(change.user_id, LeaderboardRow{
                        user_id: change.user_id,
                        mmr: change.battle_score as u32,
                        battles: 1,
                        victories: if change.victory {1} else {0},
                        early_quites: if change.early_quite {1} else {0},
                        top_20: if change.top_20 {1} else {0},
                        battle_score: change.battle_score,
                        last_session: change.last_session
                    });
                }
            };
        }
        let res = self.sets.clone();
        self.sets = Vec::new();
        res
    }

    /// Constructs a `Leaderboard` by restoring persisted state from disk.
    ///
    /// Reads two files when they exist:
    /// - `data/leaderboard_v1/base` — core leaderboard rows (`LeaderboardRow` per user).
    /// - `data/leaderboard_v1/battle_faction` — per-user battle counts grouped by faction.
    ///
    /// Players with fewer than 6 battles are loaded but excluded from the `battle_score_hash`
    /// lookup index used by [`get_mmr_for_new`]. Returns an empty leaderboard when the files
    /// are absent.
    pub fn new() -> Self {
        // Restore leaderboard state from persisted files when available.
        let mut users: std::collections::HashMap<u64, LeaderboardRow> = std::collections::HashMap::new();
        let mut battle_scores: std::collections::BTreeMap<(u32, u64), u32> =  std::collections::BTreeMap::new();
        if Path::new("data/leaderboard_v1/base").exists() {
            if let Ok(lines) = read_lines("data/leaderboard_v1/base") {
                for line in lines.flatten() {
                    match LeaderboardRow::parse_file(line.replace("\"", "")) {
                        Some(row) => {
                            users.insert(row.user_id, row.clone());
                            // Index only calibrated users for bootstrap estimates.
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
        // Restore per-user battles grouped by faction/mode.
        if Path::new("data/leaderboard_v1/battle_faction").exists() {
            if let Ok(lines) = read_lines("data/leaderboard_v1/battle_faction") {
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
    /// - `data/leaderboard_v1/base` — one serialized `LeaderboardRow` per line.
    /// - `data/leaderboard_v1/battle_faction` — per-user faction battle counters in
    ///   `user_id:<id>,faction:<name>,battles:<n>` format.
    pub async fn write(&self){

        // Persist core leaderboard rows.

        let data_file = tokio::fs::File::create("data/leaderboard_v1/base".to_string()).await.unwrap();
        let mut data_file = BufWriter::new(data_file);
        // Write contents to the file

        for (_user_id, row) in self.users.clone().iter() {
            data_file.write_all((row.to_string() + "\n").as_bytes()).await.unwrap();
        }
        data_file.flush().await.unwrap();

        
        // Persist per-user faction battle counters.
        let data_file = tokio::fs::File::create("data/leaderboard_v1/battle_faction".to_string()).await.unwrap();
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
    /// - `MMRType::MMR(mmr)`          — fully calibrated (6+ battles).
    /// - `MMRType::NotEnought(mmr)`   — provisional value (1–5 battles).
    /// - `MMRType::None`              — user has never appeared in a session.
    pub fn get_mmr(&self, user_id: u64) -> MMRType {
        // NotEnought = provisional MMR before calibration threshold.
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

    /// Processes a completed session and updates the leaderboard.
    ///
    /// Executes the full v1 session pipeline in five timed stages:
    /// 1. **Prepare** — splits `session_memory` into team_1 / team_2 snapshots and ranks each
    ///    team by current MMR. Sessions with fewer than 5 players per side or belonging to
    ///    the `newbie_common` mode are skipped (`None` is returned).
    /// 2. **Write statistics** — when `cl_id > 0`, emits `Statistic` payloads for the
    ///    `common` board and all mode-specific boards via `sender`, and forwards
    ///    `(win_avg_mmr, lose_avg_mmr)` to `sender_check`. Also computes top-3 MMR averages
    ///    and sends team-disbalance flags to `sender_session_class`.
    /// 3. **Prepare changes** — builds `LeaderboardChangeV1` records for every player using
    ///    the opposing team's top-3 as reference context.
    /// 4. **Apply changes** — calls [`set_changes`] to update in-memory MMR values.
    ///
    /// Returns four `(Instant, Duration)` tuples — one per stage — for caller-side profiling,
    /// or `None` if the session was skipped.
    pub async fn make_session(
        &mut self,
        session_memory: memory::SessionMemory,
        cl_id: u16,
        sender: flume::Sender<(String, Statistic)>,
        session_mode: &datasets::SessionMode,
        registrations: &datasets::Registrations,
        sender_tasks: flume::Sender<(LeaderboardChangeV1, i32, LeaderboardRow, MMRChangeDebug, u16)>, 
        sender_check: flume::Sender<(u32, u32)>, 
        sender_session_class: flume::Sender<(u64, bool, bool, bool, bool)>,
        setting_change: bool
    ) -> Option<((Instant, Duration), (Instant, Duration), (Instant, Duration), (Instant, Duration))> 
    {
        // 1) Build per-session team snapshots and rank players by known MMR.
        let prepear_session = Instant::now();
    
        let common_score = session_memory.rows.iter().fold(
            0 as u64, 
            |fold_obj, other| {
                fold_obj + other.battle_score as u64
            }
        );
    
        let team_1: memory::SessionMemory = memory::SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_1".to_string() {Some(item)} else {None}).collect() 
        };
        let team_2: memory::SessionMemory = memory::SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_2".to_string() {Some(item)} else {None}).collect() 
        };
    
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
    
        let mode_type = match session_mode.0.get(&session_memory.now_session_id) {
            Some((_mode_0, mode_1, _mode_2)) => {
                mode_1.clone()
            },
            None => "newbie_common".to_string()
        };
    
        //println!("{} {}", team_1.rows.len(), team_2.rows.len());
        // Ignore incomplete matches and newbie mode for leaderboard updates.
        if team_1.rows.len() < 5 || team_2.rows.len() < 5 || mode_type == "newbie_common".to_string() {
            return None;
        }
        
        let prepear_session_time = prepear_session.elapsed();
        let team_1_res = team_1.rows.iter().fold(false, |a,b| if a == true || b.victories == true {true} else {false});
        let team_2_res = team_2.rows.iter().fold(false, |a,b| if a == true || b.victories == true {true} else {false});
    
        let write_session = Instant::now();
        if cl_id > 0 { 
            // 2) Emit statistics for common and mode-specific boards.
            sender
                .send(statistic::proc_statistic(
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
                        .send(statistic::proc_statistic(
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
                        .send(statistic::proc_statistic(
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
                        .send(statistic::proc_statistic(
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
            // 3) Compute simple team disbalance flags from top-3 MMR averages.
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
            
            // count team 2 disbalance sessions
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
        }
    
        let write_session_time = write_session.elapsed();
    
        let prepear_change = Instant::now();
        // 4) Build per-user change payloads against opposite team context.
        let mut top_3_lose = team_2_mmr.0.clone();
        top_3_lose.sort_unstable_by_key(|o| match o.1 {
            MMRType::MMR(data) => data,
            _ => 0
        });
        top_3_lose.reverse();
        let top_3_lose = top_3_lose.into_iter().filter_map(|o| Some(o.1)).collect::<Vec<MMRType>>();
        for win_user in team_1.rows.iter() {
            self.add_change(LeaderboardChangeV1 { user_id: win_user.user_id, mmr: self.get_mmr(win_user.user_id), top_3: top_3_lose.clone(), victory: team_1_res, early_quite: win_user.early_quit, top_20: win_user.team_score_top_20_percent, battle_score: win_user.battle_score, battle_score_muld: ((1600.0 * (team_2.rows.len() + team_1.rows.len()) as f64) / (common_score as f64) * (win_user.battle_score as f64)) as u32, faction: win_user.faction.clone(), last_session: win_user.commit_time })
        }
        
        let mut top_3_win = team_1_mmr.0.clone();
        top_3_win.sort_unstable_by_key(|o| match o.1 {
            MMRType::MMR(data) => data,
            _ => 0
        });
        top_3_win.reverse();
        let top_3_win = top_3_win.into_iter().filter_map(|o| Some(o.1)).collect::<Vec<MMRType>>();
        for lose_user in team_2.rows.iter() {
            self.add_change(LeaderboardChangeV1 { user_id: lose_user.user_id, mmr: self.get_mmr(lose_user.user_id), top_3: top_3_win.clone(), victory: team_2_res, early_quite: lose_user.early_quit, top_20: lose_user.team_score_top_20_percent, battle_score: lose_user.battle_score, battle_score_muld: ((1600.0 * (team_2.rows.len() + team_1.rows.len()) as f64) / (common_score as f64) * (lose_user.battle_score as f64)) as u32, faction: lose_user.faction.clone(), last_session: lose_user.commit_time })
        }
        let prepear_change_time = prepear_change.elapsed();
    
        let set_change = Instant::now();
        // 5) Apply prepared changes to in-memory leaderboard state.
        self.set_changes(cl_id, sender_tasks, setting_change).await;
        let set_change_time = set_change.elapsed();
        Some(((prepear_session, prepear_session_time), (write_session, write_session_time), (prepear_change, prepear_change_time), (prepear_change, set_change_time)))
    
    }

    /// Processes a completed session without emitting statistics (debug/inspection variant).
    ///
    /// Mirrors the team-split, MMR-ranking and change-building logic of [`make_session`] but
    /// skips all statistic and session-classification side effects. Uses a hardcoded
    /// `mode_type` of `"low_br"` so no session is filtered as `newbie_common`.
    ///
    /// Calls [`set_changes_lite`] to apply and return the full list of `LeaderboardChangeV1`
    /// records, or `None` when the session has fewer than 5 players per team.
    pub async fn make_session_lite(
        &mut self,
        session_memory: memory::SessionMemory,
    ) -> Option<Vec<LeaderboardChangeV1>> {
        // Lite variant: produces and applies change set without statistic side effects.
        let prepear_session = Instant::now();
    
        let common_score = session_memory.rows.iter().fold(
            0 as u64, 
            |fold_obj, other| {
                fold_obj + other.battle_score as u64
            }
        );
    
        let team_1: memory::SessionMemory = memory::SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_1".to_string() {Some(item)} else {None}).collect() 
        };
        let team_2: memory::SessionMemory = memory::SessionMemory { 
            now_session_id: session_memory.now_session_id, 
            rows: session_memory.rows.clone().into_iter().filter_map(|item| if item.faction == "faction_2".to_string() {Some(item)} else {None}).collect() 
        };
    
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
    
        let mode_type = "low_br".to_string();
    
        if team_1.rows.len() < 5 || team_2.rows.len() < 5 || mode_type == "newbie_common".to_string() {
            return None;
        }
        
        let _prepear_session_time = prepear_session.elapsed();
        let team_1_res = team_1.rows[0].victories;
        let team_2_res = team_2.rows[0].victories;
    
        let prepear_change = Instant::now();
        let mut top_3_lose = team_2_mmr.0.clone();
        top_3_lose.sort_unstable_by_key(|o| match o.1 {
            MMRType::MMR(data) => data,
            _ => 0
        });
        top_3_lose.reverse();
        let top_3_lose = top_3_lose.into_iter().filter_map(|o| Some(o.1)).collect::<Vec<MMRType>>();
        for win_user in team_1.rows.iter() {
            self.add_change(LeaderboardChangeV1 { user_id: win_user.user_id, mmr: self.get_mmr(win_user.user_id), top_3: top_3_lose.clone(), victory: team_1_res, early_quite: win_user.early_quit, top_20: win_user.team_score_top_20_percent, battle_score: win_user.battle_score, battle_score_muld: ((1600.0 * (team_2.rows.len() + team_1.rows.len()) as f64) / (common_score as f64) * (win_user.battle_score as f64)) as u32, faction: win_user.faction.clone(), last_session: win_user.commit_time })
        }
        
        let mut top_3_win = team_1_mmr.0.clone();
        top_3_win.sort_unstable_by_key(|o| match o.1 {
            MMRType::MMR(data) => data,
            _ => 0
        });
        top_3_win.reverse();
        let top_3_win = top_3_win.into_iter().filter_map(|o| Some(o.1)).collect::<Vec<MMRType>>();
        for lose_user in team_2.rows.iter() {
            self.add_change(LeaderboardChangeV1 { user_id: lose_user.user_id, mmr: self.get_mmr(lose_user.user_id), top_3: top_3_win.clone(), victory: team_2_res, early_quite: lose_user.early_quit, top_20: lose_user.team_score_top_20_percent, battle_score: lose_user.battle_score, battle_score_muld: ((1600.0 * (team_2.rows.len() + team_1.rows.len()) as f64) / (common_score as f64) * (lose_user.battle_score as f64)) as u32, faction: lose_user.faction.clone(), last_session: lose_user.commit_time })
        }
        let _prepear_change_time = prepear_change.elapsed();
    
        let set_change = Instant::now();
        let sets = self.set_changes_lite().await;
        let _set_change_time = set_change.elapsed();
        Some(sets)
    
    }
}