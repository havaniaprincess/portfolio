use crate::{datasets, types::{self, MMRAgg, MMRType}};

#[derive(Clone,Debug)]
pub struct Statistic {
    // key: (team1_top3_avg, team2_top3_avg), value: (team1_wins, total_sessions)
    pub top_3_disbalance_spread: std::collections::BTreeMap<(u32, u32), (u64, u64)>,
    // Total processed sessions for this statboard.
    pub battles: u64,
    // Sessions where top-3 team1 avg MMR exceeds team2 by threshold.
    pub disbalance_team_1: u64,
    // Disbalanced sessions above where team1 also won.
    pub disbalance_team_1_victory: u64,
    // Sessions where top-3 team2 avg MMR exceeds team1 by threshold.
    pub disbalance_team_2: u64,
    // Disbalanced sessions above where team2 also won.
    pub disbalance_team_2_victory: u64,
    // Newbie battles played in disbalanced sessions.
    pub new_user_disbalance_battles: u64,
    // Newbie battles where the newbie was on the stronger side.
    pub new_user_disbalance_allie_battles: u64,
    // Newbie battles on stronger side that ended in victory.
    pub new_user_disbalance_battles_victory: u64,
    // Total battles with newbies.
    pub new_user_battles: u64,
    // Sessions that included at least one newbie.
    pub new_users_session: u64,
    // Sessions with strong newbie-ratio skew toward team1.
    pub newbie_count_disbalance_team_1: u64,
    // Sessions with strong newbie-ratio skew toward team2.
    pub newbie_count_disbalance_team_2: u64,
    // Newbie sessions that were also MMR-disbalanced.
    pub new_user_disbalance_session: u64
}


impl Statistic {
    pub fn to_string(&self, statboard: &String) -> String {
        // Serialize statistic into legacy line format consumed by downstream scripts.
        println!("{}", statboard.clone());
        let mut str = String::new();
        str = str + statboard.as_str() + "__top_3_disbalance_spread:{";
        let mut spread_btree: std::collections::BTreeMap<i32, (u64, u64)> = std::collections::BTreeMap::new();

        // Re-bucket top3 delta into 200-MMR ranges for compact output.
        for item in self.top_3_disbalance_spread.iter() {
            match spread_btree.get_mut(&(((((item.0.0 as i32) - (item.0.1 as i32)) / 200) * 200))) {
                Some(btree) => {
                    btree.0 += item.1.0;
                    btree.1 += item.1.1; 
                },
                None => {
                    spread_btree.insert((((item.0.0 as i32) - (item.0.1 as i32)) / 200) * 200, (item.1.0, item.1.1));
                }
            }
        }

        for spread in spread_btree.iter() {
            str = str + "\"" + spread.0.to_string().as_str() + "\":[" + ((spread.1.0 as f64) / (spread.1.1 as f64)).to_string().as_str() + "," + spread.1.0.to_string().as_str() + "," + spread.1.1.to_string().as_str() + "],"
        }
        str = str + "};\n";
        str = str + statboard.as_str() + "__battles:" + self.battles.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__disbalance_team_1:" + self.disbalance_team_1.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__disbalance_team_1_victory:" + self.disbalance_team_1_victory.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__disbalance_team_2:" + self.disbalance_team_2.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__disbalance_team_2_victory:" + self.disbalance_team_2_victory.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_user_disbalance_battles:" + self.new_user_disbalance_battles.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_user_disbalance_allie_battles:" + self.new_user_disbalance_allie_battles.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_user_disbalance_battles_victory:" + self.new_user_disbalance_battles_victory.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_user_battles:" + self.new_user_battles.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_users_session:" + self.new_users_session.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__newbie_count_disbalance_team_1:" + self.newbie_count_disbalance_team_1.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__newbie_count_disbalance_team_2:" + self.newbie_count_disbalance_team_2.to_string().as_str() + ";\n";
        str = str + statboard.as_str() + "__new_user_disbalance_session:" + self.new_user_disbalance_session.to_string().as_str() + ";\n";
        str
    }
    pub fn add_statistic(&mut self, other: &Statistic){
        // Merge counters from another partial statistic payload.
        self.battles += other.battles;
        self.disbalance_team_1 += other.disbalance_team_1;
        self.disbalance_team_1_victory += other.disbalance_team_1_victory;
        self.disbalance_team_2 += other.disbalance_team_2;
        self.disbalance_team_2_victory += other.disbalance_team_2_victory;
        self.new_user_disbalance_battles += other.new_user_disbalance_battles;
        self.new_user_disbalance_allie_battles += other.new_user_disbalance_allie_battles;
        self.new_user_disbalance_battles_victory += other.new_user_disbalance_battles_victory;
        self.new_user_battles += other.new_user_battles;
        self.new_users_session += other.new_users_session;
        self.newbie_count_disbalance_team_1 += other.newbie_count_disbalance_team_1;
        self.newbie_count_disbalance_team_2 += other.newbie_count_disbalance_team_2;
        self.new_user_disbalance_session += other.new_user_disbalance_session;

        for (spread_idx, spread_res) in other.top_3_disbalance_spread.iter() {
            match self.top_3_disbalance_spread.get_mut(&spread_idx) {
                Some(item) => {
                    item.0 += spread_res.0;
                    item.1 += spread_res.1;
                },
                None => {self.top_3_disbalance_spread.insert(*spread_idx, (spread_res.0, spread_res.1));}
            };
        }
    }
}

pub async fn proc_statistic(
    statboard: String,
    team_1_mmr: &types::TeamMMR,
    team_2_mmr: &types::TeamMMR,
    team_1_res: bool,
    team_2_res: bool,
    registrations: &datasets::Registrations,
    sender_check: Option<flume::Sender<(u32, u32)>>,
) -> (String, Statistic) {
    // Build per-session statistic record to be later aggregated by statboard key.
    let mut statistic = Statistic{
        top_3_disbalance_spread: std::collections::BTreeMap::new(),
        battles: 0,
        disbalance_team_1: 0,
        disbalance_team_1_victory: 0,
        disbalance_team_2: 0,
        disbalance_team_2_victory: 0,
        new_user_disbalance_battles: 0,
        new_user_disbalance_battles_victory: 0,
        new_user_disbalance_allie_battles: 0,
        new_user_battles: 0,
        new_users_session: 0,  
        newbie_count_disbalance_team_1: 0,
        newbie_count_disbalance_team_2: 0,
        new_user_disbalance_session: 0
      };
    statistic.battles += 1;
        // Compute average top-3 calibrated MMR for each team.
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
    /* if team_2_top3.0 == 0 {
        println!("team {:?}", team_2_mmr.clone());
        println!("res {:?}", team_2_top3);
    } */
    let team_2_top3 = if team_2_top3d.0 == 0 {
        None
    } else {
        Some((team_2_top3d.1 / team_2_top3d.0) as u32)
    };

    // Optional sanity stream for win-probability checks.
    if team_1_top3 != None && team_2_top3 != None {
        if ((team_1_top3.unwrap() as i64) - (team_2_top3.unwrap() as i64)).abs() > 30000 {
            println!("team {:?}", team_1_mmr.clone());
            println!("team {:?}", team_2_mmr.clone());
            println!("team {:?}", team_1_top3d.clone());
            println!("team {:?}", team_1_top3.clone());
            println!("team {:?}", team_2_top3d.clone());
            println!("team {:?}", team_2_top3.clone());

        }
        match sender_check {
            Some(sender) => {
                sender
                    .send((if team_1_res {team_1_top3.unwrap()} else {team_2_top3.unwrap()}, if team_1_res {team_2_top3.unwrap()} else {team_1_top3.unwrap()}))
                    .unwrap();
            },
            None => {}
        }
    }
    // Track win-rate conditioned on exact (top3_team1, top3_team2) pair.
    if team_1_top3 != None && team_2_top3 != None {
        match statistic.top_3_disbalance_spread.get_mut(&(team_1_top3.unwrap(), team_2_top3.unwrap())) {
            Some(data) => {
                data.0 = data.0.clone() + if team_1_res {1} else {0};
                data.1 = data.1.clone() + 1;
            },
            None => {
                statistic.top_3_disbalance_spread.insert((team_1_top3.unwrap(), team_2_top3.unwrap()), (if team_1_res {1} else {0}, 1));
            }
        }
    }
    // Count sessions where team1 had significant top-3 MMR advantage.
    statistic.disbalance_team_1 += match team_1_top3 {
        Some(team_1) => {
            match team_2_top3 {
                Some(team_2) => {
                    if (team_1 as i32) - (team_2 as i32) > 800 {
                        1
                    } else {
                        0
                    }
                },
                _ => 0
            }
        },
        _ => {
            0
        }
    };
    
    // Count sessions where team2 had significant top-3 MMR advantage.
    statistic.disbalance_team_2 += match team_1_top3 {
        Some(team_1) => {
            match team_2_top3 {
                Some(team_2) => {
                    if (team_2 as i32) - (team_1 as i32) > 800 {
                        1
                    } else {
                        0
                    }
                },
                _ => 0
            }
        },
        _ => {
            0
        }
    };
    
    // Count wins in team1-advantaged sessions.
    statistic.disbalance_team_1_victory += match team_1_top3 {
        Some(team_1) => {
            match team_2_top3 {
                Some(team_2) => {
                    if (team_1 as i32) - (team_2 as i32) > 800 && team_1_res{
                        1
                    } else {
                        0
                    }
                },
                _ => 0
            }
        },
        _ => {
            0
        }
    };
    // Count wins in team2-advantaged sessions.
    statistic.disbalance_team_2_victory += match team_1_top3 {
        Some(team_1) => {
            match team_2_top3 {
                Some(team_2) => {
                    if (team_2 as i32) - (team_1 as i32) > 800 && team_2_res{
                        1
                    } else {
                        0
                    }
                },
                _ => 0
            }
        },
        _ => {
            0
        }
    };

    let mut newbie_fl = false;
    let mut team_1_newbie_count: u16 = 0;
    // Newbie window: first 24h from registration timestamp.
    for user in team_1_mmr.0.iter() {
        let user_id = user.0;
        let newbie_status = match registrations.0.get(&user_id) {
            Some(time) => {
                if user.2.commit_time > *time && user.2.commit_time - time < 24*60*60*1000 {
                    team_1_newbie_count += 1;
                    true
                } else {
                    false
                }
            },
            None => {
                false
            }
        };
        if !newbie_status {
            continue;
        }
        // Per-user newbie battle counter.
        if newbie_status {
            statistic.new_user_battles += 1;
        }
        // Per-session newbie counters (increment once per session).
        if newbie_status && !newbie_fl {
            statistic.new_users_session += 1;
            newbie_fl = true;
            statistic.new_user_disbalance_session += match team_1_top3 {
                Some(team_1) => {
                    match team_2_top3 {
                        Some(team_2) => {
                            if ((team_2 as i32) - (team_1 as i32) > 800) || ((team_1 as i32) - (team_2 as i32) > 800) {
                                1
                            } else {
                                0
                            }
                        },
                        _ => 0
                    }
                },
                _ => {
                    0
                }
            };
        }

        // Newbie-in-disbalanced-session counters.
        statistic.new_user_disbalance_battles += match team_1_top3 {
            Some(team_1) => {
                match team_2_top3 {
                    Some(team_2) => {
                        if ((team_2 as i32) - (team_1 as i32)).abs() > 800 {
                            1
                        } else {
                            0
                        }
                    },
                    _ => 0
                }
            },
            _ => {
                0
            }
        };
        statistic.new_user_disbalance_allie_battles += match team_1_top3 {
            Some(team_1) => {
                match team_2_top3 {
                    Some(team_2) => {
                        if ((team_2 as i32) - (team_1 as i32) > 800 && user.2.team == 2) || ((team_1 as i32) - (team_2 as i32) > 800 && user.2.team == 1) {
                            1
                        } else {
                            0
                        }
                    },
                    _ => 0
                }
            },
            _ => {
                0
            }
        };
    
        statistic.new_user_disbalance_battles_victory += match team_1_top3 {
            Some(team_1) => {
                match team_2_top3 {
                    Some(team_2) => {
                        if (((team_2 as i32) - (team_1 as i32) > 800 && user.2.team == 2) || ((team_1 as i32) - (team_2 as i32) > 800 && user.2.team == 1)) && user.2.victories {
                            1
                        } else {
                            0
                        }
                    },
                    _ => 0
                }
            },
            _ => {
                0
            }
        };
    }
    let mut team_2_newbie_count: u16 = 0;
    // Count newbie users on team2 for team-level newbie ratio imbalance.
    for user in team_2_mmr.0.iter() {
        let user_id = user.0;
        match registrations.0.get(&user_id) {
            Some(time) => {
                if user.2.commit_time > *time && user.2.commit_time - time < 24*60*60*1000 {
                    team_2_newbie_count += 1;
                }
            },
            None => {}
        };
    }

    // Team-level newbie skew: one team has high newbie ratio while the other is low.
    statistic.newbie_count_disbalance_team_1 += if team_1_newbie_count as f64 / (team_1_mmr.0.len() as f64) >= 0.69 && team_2_newbie_count as f64 / (team_2_mmr.0.len() as f64) < 0.31 {
        1
    } else {
        0
    };

    statistic.newbie_count_disbalance_team_2 += if team_2_newbie_count as f64 / (team_2_mmr.0.len() as f64) >= 0.69 && team_1_newbie_count as f64 / (team_1_mmr.0.len() as f64) < 0.31 {
        1
    } else {
        0
    };

    //println!("here! {:?}", write_session_time);
    (statboard, statistic)
}