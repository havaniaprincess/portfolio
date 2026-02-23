use train::TrainShiftOut;
use std::fs;

use ron::ser::{to_string_pretty, PrettyConfig};
use ron::de::from_reader;
use rand::prelude::*;


/// Application module declarations.
///
/// - `types`       – shared newtype wrappers (`Hms`, `Seconds`, `TrainId`, …).
/// - `line`        – `Line` / `LineDir` structures and run-generation logic.
/// - `run`         – `Run`, `RunArray`, `ShiftType` and related helpers.
/// - `math`        – small numeric utilities (min, …).
/// - `way_getting` – helper that injects service (depot pull-out/push-in) moves.
/// - `train`       – `Train`, `TrainShift`, `TrainShiftOut`, `Place` definitions.
/// - `config`      – `Config` structure loaded from `config.ron`.
/// - `shifts`      – `get_shifts` that groups individual runs into driver shifts.
mod types;
mod line;
mod run;
mod math;
mod way_getting;
mod train;
mod config;
mod shifts;

/// Entry point of the timetable generator.
///
/// High-level execution flow:
///
/// 1. **Load line definitions** from `lines.ron` and index them by `(id, direction)`.
/// 2. **Generate runs** for the `"sok"` line in both directions for a full 7-day week.
/// 3. **Group runs into driver shifts** via [`shifts::get_shifts`] and write the result
///    to `data/1/weeks/0/shifts.ron`.
/// 4. **Load runtime config** (`config.ron`) and the **train roster** (`data/1/trains.ron`).
/// 5. **Assign shifts to trains** day by day (days 0–6):
///    - Trains already waiting at a terminal station are matched first (station pass).
///    - Remaining unassigned shifts pull trains out of the depot (depot pass).
///    - After each day, cumulative work-time / maintenance counters are updated.
///    - Intermediate snapshots are written to `trains_out_<day>.ron` and
///      `trains_stat_out_<day>.ron`.
/// 6. **Write final outputs**: end-of-week train states, full shift map, and a
///    display-friendly `TrainShiftOut` variant – all serialised as RON.
fn main() {

    // ── Step 1: Load line definitions ──────────────────────────────────────
    // Read all `Line` entries from `lines.ron` into a flat vector, then index
    // them into a HashMap keyed by (line_id, direction_string) for O(1) look-ups.
    let fl = fs::File::open("lines.ron").expect("Failed opening file");
    let lines_vec: Vec<line::Line> = match from_reader(fl) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            std::process::exit(1);
        }
    };
    // Build the (id, direction) → Line index used throughout the program.
    let lines: std::collections::HashMap<(String, String), line::Line> = lines_vec.iter().map(|line| ((line.get_id().clone(), line.get_dir().clone()), line.clone())).collect();

    // ── Step 2: Generate week-long departure runs ─────────────────────────
    // Each direction receives the other direction's travel time so that the
    // full round-trip cycle length is computed correctly inside `get_runs`.
    let runs_right = lines[&("sok".to_string(), "right".to_string())].get_runs(lines[&("sok".to_string(), "back".to_string())].duration.0.0, line::LineDir::Right);
    let runs_back = lines[&("sok".to_string(), "back".to_string())].get_runs(lines[&("sok".to_string(), "right".to_string())].duration.0.0, line::LineDir::Back);

    // ── Step 3: Group individual runs into driver shifts ───────────────────
    // Returns a nested map: day_index → shift_index → Vec<Run>.
    let day_shifts: std::collections::BTreeMap<i64, std::collections::BTreeMap<i64, Vec<run::Run>>> = shifts::get_shifts(&runs_right, &runs_back, &lines);
    
    //dbg!(day_shifts.clone());
    // Serialise day_shifts to `data/1/weeks/0/shifts.ron` for human inspection.
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let config_str = to_string_pretty(&day_shifts, pretty).expect("Serialization failed");

    let file = "data/1/weeks/0/shifts".to_string() + ".ron";
    fs::write(file.as_str(), config_str)
        .expect("Should have been able to read the file");
















/*     let mut trains: std::collections::BTreeMap<(String, types::TrainId), train::Train> = std::collections::BTreeMap::new();
    for train in 1..41 {
        trains.insert(("th19".to_string(), train), train::Train::new(train, "th19".to_string(), line::LineDir::Back));

    }
    for train in 1..41 {
        trains.insert(("th15".to_string(), train), train::Train::new(train, "th15".to_string(), line::LineDir::Right));

    }
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let config_str = to_string_pretty(&trains, pretty).expect("Serialization failed");

    let file = "data/10/trains".to_string() + ".ron";
    fs::write(file.as_str(), config_str)
        .expect("Should have been able to read the file");  */




    // ── Step 4a: Load runtime configuration ───────────────────────────────
    // `config.ron` holds scheduling parameters, notably `now_time` – the
    // reference clock offset used when comparing train free-times against
    // shift start times.
    let fl = fs::File::open("config.ron").expect("Failed opening file");
    let config: config::Config = match from_reader(fl) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            std::process::exit(1);
        }
    };
    

    // ── Step 4b: Load train roster ──────────────────────────────────────
    // Each entry carries the train's current `place` (Depot / Station),
    // cumulative `work_time`, maintenance countdown timers and `home_station`.
    let fl = fs::File::open("data/1/trains.ron").expect("Failed opening file");
    let mut trains: std::collections::BTreeMap<(String, types::TrainId), train::Train> = match from_reader(fl) {
        Ok(x) => x,
        Err(e) => {
            println!("Failed to load config: {}", e);

            std::process::exit(1);
        }
    };

    // ── Step 5: Assign shifts to trains, day by day ────────────────────
    // `train_shifts` accumulates the final per-day assignment:
    //   day_index → { (line_id, train_id) → TrainShift }
    let mut train_shifts: std::collections::BTreeMap<i64, std::collections::BTreeMap<(String, types::TrainId), train::TrainShift>> = std::collections::BTreeMap::new();
    for (day, mut shifts_day) in day_shifts.clone().into_iter() {
        dbg!(day);
        let mut train_day_shifts: std::collections::BTreeMap<(String, types::TrainId), train::TrainShift> = std::collections::BTreeMap::new();
        // Initialise an empty TrainShift record for every known train for this day.
        // GENERATE train shifts
        for (tr_id, _tr_data) in trains.clone().into_iter() {
            train_day_shifts.insert(tr_id.clone(), train::TrainShift::new(tr_id.1, tr_id.0.clone()));
        }

        // Service look: inject depot pull-out / push-in legs into each train's
        // shift so that non-revenue movements are tracked before revenue runs.
        // Service look
        way_getting::service_look(&mut trains, &mut train_day_shifts);

        // Station-to-shift matching pass:
        // Trains already sitting at a terminal station are matched first to the
        // nearest unassigned shift departing from that station.  This avoids
        // unnecessary dead-mileage depot trips for trains already in position.
        // Look station -- station shifts
        let station_trains = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
            match obj.place {
                train::Place::Station(_) => Some((tr_id, obj)),
                _ => None
            }
        }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();

        for (tr_id, tr_data) in station_trains.into_iter() {
            let station =  match tr_data.place {
                train::Place::Station(st) => st,
                _ => panic!()
            }.invert();
            let line_conf = lines[&("sok".to_string(), station.to_string().clone())].clone();
            dbg!(&tr_data);
                // Collect candidate shifts whose start time is within one headway
                // after the train becomes free; sort descending so index 0 is closest.
                let mut shift_to_train: Vec<(i64, Vec<run::Run>)> = shifts_day.clone().into_iter().filter_map(|(idx, obj)| {
                //dbg!(&obj[0], &config, &line_conf.base_time);
                if obj[0].side == station && obj[0].time.0.0 + config.now_time.0.0 - tr_data.free_time.0.0 <= 2*line_conf.base_time.0.0 && obj[0].time.0.0 + config.now_time.0.0 - tr_data.free_time.0.0 >= 0 {
                    Some((idx, obj))
                } else {
                    None
                }
            }).collect();
            shift_to_train.sort_unstable_by_key(|obj| obj.1[0].time.0.0);
            shift_to_train.reverse();

            let train_sh = match train_day_shifts.get_mut(&tr_id) {
                Some(data) => data,
                None => panic!()
            };
            let train_conf = match trains.get_mut(&tr_id) {
                Some(data) => data,
                None => panic!()
            };
            if shift_to_train.len() > 0 {
                train_sh.shift.push(run::ShiftType::RunShift(shift_to_train[0].1.clone()));
                shifts_day.remove(&shift_to_train[0].0);
                train_conf.place = train::Place::Station(shift_to_train[0].1[shift_to_train[0].1.len() - 1].side.clone());
                let free_time = types::Hms(types::Seconds(shift_to_train[0].1[shift_to_train[0].1.len() - 1].time.0.0 + line_conf.duration.0.0));
                train_conf.free_time = types::Hms(types::Seconds(config.now_time.0.0 + free_time.0.0));
                dbg!(config.now_time, free_time, line_conf.base_time);
                if config.now_time.0.0 + free_time.0.0 < config.now_time.0.0 + (day+1) * 24*60*60 - line_conf.base_time.0.0 {
                    train_sh.shift.push(run::ShiftType::Depot);
                    train_conf.place = train::Place::Depot(train_sh.depot.clone());
                    train_conf.free_time.0.0 = train_conf.free_time.0.0 + 60*60;
                }
            } else {
                train_sh.shift.push(run::ShiftType::Depot);
                train_conf.place = train::Place::Depot(train_sh.depot.clone());
                train_conf.free_time.0.0 = train_conf.free_time.0.0 + 60*60;
           }

        }
        // Write a snapshot of train states after the station-assignment pass
        // (before depot trains are pulled out) for diagnostic purposes.
        let pretty = PrettyConfig::new()
            .depth_limit(5)
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        let config_str = to_string_pretty(&trains, pretty).expect("Serialization failed");
    
        let file = "data/1/trains_stat_out_".to_string() + day.to_string().as_str() + ".ron";
        fs::write(file.as_str(), config_str)
            .expect("Should have been able to read the file");

        // Depot-to-shift assignment pass:
        // Process every shift not claimed in the station pass.  The eligible pool
        // is all depot trains whose `home_station` matches the shift direction and
        // whose `free_time` is at or before the shift start time.  A random train
        // is selected from the pool to spread mileage evenly across the fleet.
        for (idx, shift) in shifts_day.clone().into_iter() {
            let start_time = shift[0].clone().time;
            let dir = shift[0].clone().side;
            let line_conf = lines[&("sok".to_string(), dir.to_string().clone())].clone();
            let mut train_choise_id = ("".to_string(), 1);
            dbg!(&dir, &shift[0].id);
            let trains_to_shift: std::collections::BTreeMap<(String, types::TrainId), train::Train> = trains.clone().into_iter()
                .filter_map(|(tr_id, obj)| {
                    let depot_fl = match obj.place {
                        train::Place::Depot(_) => true,
                        _ => false
                    };
                    /* if depot_fl && obj.home_station == dir {
                        dbg!(&tr_id, (obj.clone(), start_time.0.0));
                    } */
                    if depot_fl && obj.home_station == dir && obj.free_time.0.0 <= start_time.0.0 + config.now_time.0.0 {
                        dbg!(&tr_id, (obj.clone(), start_time.0.0));
                        train_choise_id = tr_id.clone();
                        Some((tr_id, obj))
                    } else {
                        None
                    }
                }).collect();

            //dbg!(idx, shifts_day.len(), &shift, trains_to_shift.len());
            let mut rng = rand::thread_rng();
            let mut rnd = rng.gen::<f64>() * (trains_to_shift.len() as f64); 
            for (tr_id, _tr_data) in trains_to_shift.clone().into_iter() {
                if rnd < 0.0 {
                    break;
                }
                train_choise_id = tr_id.clone();
                rnd -= 1.0;
            }
           //dbg!((trains_to_shift.len(), train_choise_id.clone(), shifts_day.len(), dir));
            let train_sh = match train_day_shifts.get_mut(&train_choise_id) {
                Some(data) => data,
                None => panic!()
            };
            let train_conf = match trains.get_mut(&train_choise_id) {
                Some(data) => data,
                None => panic!()
            };

            train_sh.shift.push(run::ShiftType::RunShift(shift.clone()));
            shifts_day.remove(&idx);
            train_conf.place = train::Place::Station(shift[shift.len() - 1].side.clone());
            let free_time = types::Hms(types::Seconds(shift[shift.len() - 1].time.0.0 + line_conf.duration.0.0));
            train_conf.free_time = types::Hms(types::Seconds(config.now_time.0.0 + free_time.0.0));
            if config.now_time.0.0 + free_time.0.0 < config.now_time.0.0 + (day+1) * 24*60*60 - line_conf.base_time.0.0 {
                train_sh.shift.push(run::ShiftType::Depot);
                train_conf.place = train::Place::Depot(train_sh.depot.clone());
                train_conf.free_time.0.0 = train_conf.free_time.0.0 + 60*60;
            }

        }

        // Work-time accounting pass:
        // Sum the revenue running time (last run time − first run time) of every
        // shift worked today, then update each train's cumulative `work_time` and
        // decrement its four maintenance countdown timers by the same amount.
        for (idx, tr_sh) in train_day_shifts.clone().into_iter() {
            let mut work_time_day: i64 = 0;
            
            for sh in tr_sh.shift.clone().iter() {
                work_time_day += match sh.clone() {
                    run::ShiftType::RunShift(data) => {
                        data[data.len() - 1].time.0.0 - data[0].time.0.0
                    },
                    _ => 0
                };
            }
            let train_conf = match trains.get_mut(&idx) {
                Some(data) => data,
                None => panic!()
            };

            train_conf.work_time.0.0 += work_time_day;
            train_conf.time_to_type_a.0.0 -= work_time_day;
            train_conf.time_to_type_b.0.0 -= work_time_day;
            train_conf.time_to_type_c.0.0 -= work_time_day;
            train_conf.time_to_type_d.0.0 -= work_time_day;
        }

        // Store today's completed assignments; serialise updated train roster
        // (with refreshed counters) to `trains_out_<day>.ron`.
        train_shifts.insert(day, train_day_shifts.clone());
        let pretty = PrettyConfig::new()
            .depth_limit(5)
            .separate_tuple_members(true)
            .enumerate_arrays(true);
        let config_str = to_string_pretty(&trains, pretty).expect("Serialization failed");
    
        let file = "data/1/trains_out_".to_string() + day.to_string().as_str() + ".ron";
        fs::write(file.as_str(), config_str)
            .expect("Should have been able to read the file");
    }


    // ── Step 6a: Write end-of-week train states ─────────────────────────
    // `trains_out_.ron` (no day suffix) is the roster as it stands after all
    // 7 days have been processed.
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let config_str = to_string_pretty(&trains, pretty).expect("Serialization failed");

    let file = "data/1/trains_out_".to_string() + ".ron";
    fs::write(file.as_str(), config_str)
        .expect("Should have been able to read the file");


    // ── Step 6b: Write the full per-day train shift map ───────────────────
    // `train_shifts.ron` stores the complete nested map used by downstream
    // tooling to reconstruct each train's duty sequence for the week.
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let config_str = to_string_pretty(&train_shifts, pretty).expect("Serialization failed");

    let file = "data/1/weeks/0/train_shifts".to_string() + ".ron";
    fs::write(file.as_str(), config_str)
        .expect("Should have been able to read the file");
    //dbg!(lines);
    // ── Step 6c: Build and write display-friendly shift output ────────────
    // Convert every `TrainShift` to a `TrainShiftOut` (a leaner structure
    // suited for rendering / export) and serialise to `out_shifts.ron`.
    let out_shifts: std::collections::BTreeMap<i64, std::collections::BTreeMap<(String, types::TrainId), train::TrainShiftOut>> = train_shifts
        .iter()
        .map(|(day_id, tr_data)| {
            let new_data: std::collections::BTreeMap<(String, types::TrainId), train::TrainShiftOut> = tr_data
                .iter()
                .map(|(tr_id, shift)| {
                    (tr_id.clone(), TrainShiftOut::new(shift.clone()))
                }).collect();
            (*day_id, new_data)
        }).collect();
    let pretty = PrettyConfig::new()
        .depth_limit(5)
        .separate_tuple_members(true)
        .enumerate_arrays(true);
    let config_str = to_string_pretty(&out_shifts, pretty).expect("Serialization failed");

    let file = "data/1/weeks/0/out_shifts".to_string() + ".ron";
    fs::write(file.as_str(), config_str)
        .expect("Should have been able to read the file");

}
