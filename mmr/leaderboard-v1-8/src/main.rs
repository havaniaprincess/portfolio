use std::time::{Duration, Instant};
use mmr_libs::datasets::{Registrations, SessionMode, UserFaction};
use mmr_libs::memory::{read_lines, SessionMemory};
use mmr_libs::types::{Leaderboard, LeaderboardChangeV1, LeaderboardRow, MMRChangeDebug, UserBattleRow};
use mmr_libs::statistic::Statistic;
use mmr_libs::writer;
use tokio::io::AsyncWriteExt;
use std::sync::{Arc};
use tokio::sync::Mutex;
use tokio::io::BufWriter;
use clap::Parser;

// flume = "0.11.0" — multi-producer multi-consumer channel crate

use flume::{Receiver, RecvError};

/// Command-line arguments for the leaderboard v1 pipeline.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the file containing (user_id, session_id) -> (team, victory) mappings.
    #[arg(long)]
    pub user_team: String,
    /// Path to the file mapping session_id to mode name.
    #[arg(long)]
    pub session_mode: String,
    /// Path to the file mapping user_id to faction.
    #[arg(long)]
    pub user_faction: String,
    /// Path to the main userstat dataset file.
    #[arg(long)]
    pub data: String,
    /// Path to the existing leaderboard snapshot (used as a starting state).
    #[arg(long)]
    pub leaderboard: String,
}


/// Entry point for the leaderboard v1 pipeline.
///
/// Orchestrates the full MMR recalculation pass:
/// 1. Parses CLI arguments pointing to the required input datasets.
/// 2. Loads the `user_team` mapping `(user_id, session_id) -> (team, victory)` from disk.
/// 3. Initialises auxiliary datasets: session modes, player registrations and faction assignments.
/// 4. Spawns four background workers via Tokio tasks:
///    - `statistic_aggregate` — merges per-session `Statistic` payloads into a single board map.
///    - `statistic_check`     — accumulates win-rate sanity counters bucketed by MMR delta.
///    - `write_change`        — streams per-user MMR change records to output files.
///    - `session_class_aggreg`— persists team-composition classification flags per session.
/// 5. Drives `async_main`, which reads the userstat dataset line-by-line, groups rows by
///    `session_id`, and calls `Leaderboard::make_session` for each completed session.
/// 6. After the streaming pass, awaits all background workers and writes the aggregated
///    statistics to `data/statistic_v1_8`.
/// 7. Flushes the in-memory session state and the final leaderboard snapshot to disk.
#[tokio::main]
async fn main() {
  // Parse CLI arguments and initialize run timer.
  let args: Args = Args::parse();
  let start = Instant::now();


  let elapsed = start.elapsed();

  println!("Start: {:?}", elapsed);

  // (user_id, session_id) -> (team_id, victory_flag)
  let mut user_team: std::collections::HashMap<(u64,u64), (u8, bool)> = std::collections::HashMap::new();

  // Read data with information about team and result for each user in each session. Format of line: {"user_id":123,"session_id":123,"team":1,"victory":true}
  // Result format is HashMap with key (user_id, session_id) and value (team, victory). Team is 1 or 2, victory is true if team won and false if team lost. If line has wrong format, it will be skipped. If line has no victory field, it will be considered as defeat.
  if let Ok(lines) = read_lines(&args.user_team) {
    for line in lines.flatten() {
      // Very lightweight line parser for key:value JSON-like rows.
      let hash_line: std::collections::BTreeMap<String, String> = line.replace("\"", "").replace("{", "").replace("}", "").split(",").filter_map(|item: &str| {
        let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
        if splited.len() == 2 {
          Some((splited[0].clone(), splited[1].clone()))
        } else {
          None
        }
      }).collect();
      let user_id = hash_line.get("user_id");
      let session_id = hash_line.get("session_id");
      let team = hash_line.get("team");

      
      // Skip malformed rows and unknown teams.
      if user_id == None || session_id == None || (team != Some(&"1".to_string()) && team != Some(&"2".to_string())) {
        continue;
      }  

      
      // Parse the validated string fields into their concrete numeric types.
      let user_id = user_id.unwrap().parse::<u64>().unwrap(); 
      let session_id = session_id.unwrap().parse::<u64>().unwrap();
      let team = team.unwrap().parse::<u8>().unwrap();
      // Keep historical behavior: if `victory` key exists => true, else false.
      let victory = match hash_line.get("victory") {
        Some(_) => true,
        None => false
      };

      user_team.insert((user_id, session_id), (team, victory));
    }    
  }
  println!("USER TEAM ENDE"); // signals successful load of the user-team dataset
  
  let elapsed = start.elapsed();

  println!("Load user_time: {:?}", elapsed);
  
  // Create memory of sessions
  let mut record_memory: SessionMemory = SessionMemory::new();
  // Create leaderboard
  let mut leaderboard = Leaderboard::new();
  // Create session mode information about which session was played in which mode. Format of line: {"session_id":123,"mode":"ranked"}
  let session_mode = SessionMode::new(&args.session_mode);
  // Create registrations with information about when user was registered. Format of line: {"user_id":123,"registered_time":123}
  let registrations = Registrations::new();
  // Create user faction with information about which user was in which faction. Format of line: {"user_id":123,"faction":"newbie"}
  let user_faction = UserFaction::new(&args.user_faction);

  // Create channels for statistic aggregate, statistic check, changes writer and session classification
  let (sender, receiver) = flume::unbounded();
  
  // Background worker: aggregate statistics by board key.
  let stat_map = tokio::task::spawn(statistic_aggregate(
    receiver.clone()
  ));
  
  let (sender_check, receiver_check) = flume::unbounded();
  
  // Background worker: collect expected win-rate sanity buckets.
  let stat_check = tokio::task::spawn(statistic_check(
    receiver_check.clone()
  ));
  
  
  let (sender_tasks, receiver_tasks) = flume::unbounded();
  
  // Background worker: persist per-user MMR changes.
  let change_writer = tokio::task::spawn(write_change(
    receiver_tasks.clone()
  ));
  
  
  let (sender_session_class, receiver_session_class) = flume::unbounded();
  
  // Background worker: persist session team composition flags.
  let session_class_join = tokio::task::spawn(session_class_aggreg(
    receiver_session_class.clone()
  ));
  
  // Process sessions and calculate leaderboard
  async_main(&args, sender, &user_team, &mut record_memory, &mut leaderboard, &session_mode, &registrations, sender_tasks, sender_check, sender_session_class, &user_faction).await;

  // Wait for statistic aggregate, statistic check, changes writer and session classification to finish
  let stat_map = match stat_map.await {
    Ok(data) => data,
    _ => std::collections::HashMap::new()
  };
  
  // Convert (wins, games) into a ratio map for quick inspection.
  let _stat_check: std::collections::BTreeMap<i32, f64> = match stat_check.await {
    Ok(data) => data.into_iter().map(|i| (i.0, (i.1.0 as f64) / (i.1.1 as f64))).collect(),
    _ => std::collections::BTreeMap::new()
  };


  // Write per-board statistics to a flat file for post-processing.
  let statistic_file = tokio::fs::File::create("data/statistic_v1_8".to_string()).await.unwrap();
  let mut statistic_file = BufWriter::new(statistic_file);

  for (statboard, statist) in stat_map.iter() {
    //println!("{:?}\n{:?}", statboard, statist);
    statistic_file.write_all(statist.to_string(statboard).as_bytes()).await.unwrap();
    println!("{}", statboard);
  }
  statistic_file.flush().await.unwrap();

  // Await background workers so their output files are fully flushed before exit.
  match change_writer.await {
    Ok(_) => {},
    _ => {}
  };
  match session_class_join.await {
    Ok(_) => {},
    _ => {}
  };

  // Persist in-memory session state and final leaderboard snapshot.
  record_memory.write();
  leaderboard.write().await;
}

/// Background task that accumulates win-rate statistics bucketed by MMR delta.
///
/// Each message contains the average MMR of the winning team and the losing team.
/// Results are stored as (wins, total_games) per 200-MMR bucket so callers can
/// compute empirical win-rates and verify they match theoretical ELO expectations.
async fn statistic_check(
  receiver: Receiver<(u32, u32)>
) -> std::collections::HashMap<i32, (u64, u64)> {
  // key: MMR delta bucket (step 200), value: (wins, total_games)
  let mut state_check_hash: std::collections::HashMap<i32, (u64, u64)> = std::collections::HashMap::new();
  #[allow(clippy::while_let_loop)] 
  loop {
    match receiver.recv_async().await {
      Ok((win_team, lose_team)) => {
          // Record a win (+1 wins, +1 games) for the winning side's MMR-delta bucket.
          match state_check_hash.get_mut(&((((win_team as i32) - (lose_team as i32)) / 200) * 200)) {
            Some(stat_base) => {
              stat_base.0 += 1;
              stat_base.1 += 1;
            },
            None => {
              state_check_hash.insert((((win_team as i32) - (lose_team as i32)) / 200) * 200, (1 , 1));
            }
          };
          // Record only a game (+1 games) for the losing side's MMR-delta bucket.
          match state_check_hash.get_mut(&((((lose_team as i32) - (win_team as i32)) / 200) * 200)) {
            Some(stat_base) => {
              stat_base.1 += 1;
            },
            None => {
              state_check_hash.insert((((lose_team as i32) - (win_team as i32)) / 200) * 200, (0 , 1));
            }
          };
      }
      Err(RecvError::Disconnected) => break,
    }
  }

  state_check_hash
}

/// Background task that merges incremental `Statistic` payloads into a single map.
///
/// Multiple sessions send `(board_key, Statistic)` pairs concurrently; this worker
/// folds them all into one aggregate `HashMap` keyed by the logical board name.
async fn statistic_aggregate(
  receiver: Receiver<(String, Statistic)>
) -> std::collections::HashMap<String, Statistic> {
  // Merge partial statistic payloads by logical statboard key.
  let mut stat_map: std::collections::HashMap<String, Statistic> = std::collections::HashMap::new();
  #[allow(clippy::while_let_loop)] 
  loop {
    match receiver.recv_async().await {
      Ok((statboard, statistic)) => {
          match stat_map.get_mut(&statboard) {
            Some(stat_base) => {
              // Accumulate into the existing entry.
              stat_base.add_statistic(&statistic);
            },
            None => {
              // First observation for this board key — insert directly.
              stat_map.insert(statboard.clone(), statistic);
            }
          };
      }
      Err(RecvError::Disconnected) => break,
    }
  }

  stat_map
}

/// Background task that writes per-user MMR change records to disk.
///
/// Receives tuples of `(change, mmr_diff, user_row, debug_info, classifier_id)` and
/// routes each record to the output file that matches the classifier id.
async fn write_change(
  receiver: Receiver<(LeaderboardChangeV1, i32, LeaderboardRow, MMRChangeDebug, u16)>
) {
  // Create one output file per classifier id (currently 0 and 1).
  let mut change_files: Vec<BufWriter<tokio::fs::File>> = Vec::new();
  
  for cl_id in 0..2 {
    let change_file = tokio::fs::File::create("data/changes/".to_string() + cl_id.to_string().as_str()).await.unwrap();
    change_files.push(BufWriter::new(change_file));
  }
  #[allow(clippy::while_let_loop)] 
  loop {
    match receiver.recv_async().await {
      Ok((change, mmr_diff, user_row, degub, cl_id)) => {
          // Delegate the actual serialization to the shared writer helper.
          writer::write_change(change, mmr_diff, user_row, &mut change_files[cl_id as usize], degub).await;
      }
      Err(RecvError::Disconnected) => break,
    }
  }
}

/// Background task that records per-session team composition flags.
///
/// Each record written to disk has the format:
/// `session_id:<id>,team_1:<flag>,team_2:<flag>,team_1_v:<flag>,team_2_v:<flag>`
/// where the boolean flags indicate e.g. whether a team consisted of veteran players.
async fn session_class_aggreg(
  receiver: Receiver<(u64, bool, bool, bool, bool)>
) {
  // Persist simple session-level classification markers.
  let data_file = tokio::fs::File::create("data/leaderboard_v1/session_classification_8".to_string()).await.unwrap();
  let mut data_file = BufWriter::new(data_file);
  
  #[allow(clippy::while_let_loop)] 
  loop {
    match receiver.recv_async().await {
      Ok((session_id, team_1, team_2, team_1_v, team_2_v)) => {
        // Serialize the record as a flat key:value line.
        let str = "session_id:".to_string() + session_id.to_string().as_str()
            + ",team_1:" + team_1.to_string().as_str()
            + ",team_2:" + team_2.to_string().as_str()
            + ",team_1_v:" + team_1_v.to_string().as_str()
            + ",team_2_v:" + team_2_v.to_string().as_str();
        data_file.write_all((str + "\n").as_bytes()).await.unwrap();
      }
      Err(RecvError::Disconnected) => break,
    }
  }

  data_file.flush().await.unwrap();
}


/// Core streaming loop for the leaderboard v1 pipeline.
///
/// Reads the userstat dataset line-by-line, groups rows by `session_id`, and invokes
/// `Leaderboard::make_session` for each completed session boundary. Results are
/// broadcast to the background workers through the provided flume senders:
///
/// - `sender`              — forwards `(board_key, Statistic)` payloads to `statistic_aggregate`.
/// - `sender_tasks`        — forwards `(change, mmr_diff, row, debug, cl_id)` to `write_change`.
/// - `sender_check`        — forwards `(win_team_mmr, lose_team_mmr)` to `statistic_check`.
/// - `sender_session_class`— forwards team-composition flags to `session_class_aggreg`.
///
/// Additionally writes a CSV debug dump (`data/csv/0.csv`) of every processed row and
/// prints aggregate timing diagnostics (total wall time, per-stage breakdowns) at the end.
async fn async_main(
  args: &Args,
  sender: flume::Sender<(String, Statistic)>, 
  user_team: &std::collections::HashMap<(u64,u64), (u8, bool)>,
  record_memory: &mut SessionMemory,
  leaderboard: &mut Leaderboard,
  session_mode: &SessionMode,
  registrations: &Registrations,
  sender_tasks: flume::Sender<(LeaderboardChangeV1, i32, LeaderboardRow, MMRChangeDebug, u16)>, 
  sender_check: flume::Sender<(u32, u32)>, 
  sender_session_class: flume::Sender<(u64, bool, bool, bool, bool)>,
  user_faction: &UserFaction
) {
  let _start = Instant::now();
    // CSV dump used for debugging/inspection of processed sessions.
    let session_file = tokio::fs::File::create("data/csv/".to_string() + 0.to_string().as_str() + ".csv").await.unwrap();
    let session_file = Arc::new(Mutex::new(BufWriter::new(session_file)));
    // Write the CSV header row.
    let str: String = "session_id".to_string() + ";"
      + "user_id" + ";"
      + "team;" + "victory" + ";"
      + "mmr_type" + ";"
      + "mmr" + "\n";
    
    session_file.lock().await.write_all(str.as_bytes()).await.unwrap();

      // Timing accumulators used to profile each processing stage.
      let session_common_dt = Instant::now(); // wall-clock anchor for the whole streaming loop
      let mut session_dt: Duration = Duration::new(0, 0);         // total time spent in make_session
      let mut prepear_session_dt: Duration = Duration::new(0, 0); // stage 1: prepare session data
      let mut write_session_dt: Duration = Duration::new(0, 0);   // stage 2: write session rows
      let mut prepear_change_dt: Duration = Duration::new(0, 0);  // stage 3: prepare MMR changes
      let mut set_change_dt: Duration = Duration::new(0, 0);      // stage 4: apply MMR changes

      let mut count = 0;
      // Stream input rows and group them by session_id.
      if let Ok(lines) = read_lines(&args.data) {
        for line in lines.flatten() {
          // Parse raw text line into a typed UserBattleRow; skip unparseable lines.
          match UserBattleRow::parsing_str(line.replace("\"", ""), &user_team, &user_faction) {
            Some(row) => {
              if row.session_id != record_memory.now_session_id {
                // Finalize the previous session when we detect a session switch.
                let start_makesession = Instant::now();
                // Process accumulated rows for the completed session and broadcast results.
                let timings = leaderboard.make_session(record_memory.clone(), 0, sender.clone(), session_mode, registrations, sender_tasks.clone(), sender_check.clone(), sender_session_class.clone(), true).await;
                let common_session = start_makesession.elapsed();
                session_dt += common_session;

                match timings {
                  Some((prep_sess, write_sess, prep_change, set_change)) => {
                    
                    // Defensive timing check: a sub-stage cannot exceed the total session time.
                    if set_change.1 > common_session {
                      println!("common {}", start_makesession.duration_since(start_makesession).as_nanos());
                      println!("1 {}", prep_sess.0.duration_since(start_makesession).as_nanos());
                      println!("2 {}", write_sess.0.duration_since(start_makesession).as_nanos());
                      println!("3 {}", prep_change.0.duration_since(start_makesession).as_nanos());
                      println!("4 {}", set_change.0.duration_since(start_makesession).as_nanos());
                      println!("d1 {:?}", prep_sess.1);
                      println!("d2 {:?}", write_sess.1);
                      println!("d3 {:?}", prep_change.1);
                      println!("d4 {:?}", set_change.1);
                      println!("com {:?}", common_session);
                      panic!();
                    }
                    count += 1;
                    prepear_session_dt += prep_sess.1;
                    write_session_dt += write_sess.1;
                    prepear_change_dt += prep_change.1;
                    set_change_dt += set_change.1;
                  },
                  None => {}
                }

                // Start accumulating rows for the new session.
                record_memory.now_session_id = row.session_id;
                record_memory.rows = Vec::new();
                record_memory.rows.push(row);
              } else {
                // Still within the current session — append the row.
                record_memory.rows.push(row);
              }
            },
            None => {
              // Unparseable line — silently skipped.
            }
          };
        }
      }
  
      let elapsed = session_common_dt.elapsed();
    
      // Print aggregate timing diagnostics for the whole streaming pass.
      println!("Make cluster: {:?}", elapsed);

      println!("common session dt: {:?}", count);
      println!("common session dt: {:?}", session_dt);
    
    
      println!("prep session dt: {:?}", prepear_session_dt);
    
       
      println!("write session dt: {:?}", write_session_dt);
      println!("prep change dt: {:?}", prepear_change_dt);
      println!("set change dt: {:?}", set_change_dt);
    
}
