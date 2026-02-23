use crate::run;
use crate::line;
use crate::train;
use crate::types;

/// Chains consecutive runs into a single driver shift starting from `start_run`.
///
/// Beginning at the given starting run, the function builds a sequence of
/// [`run::Run`] entries that a single train (or driver) can work back-to-back:
///
/// 1. The run identified by `start_run` is added to the result and marked as
///    allocated by setting its `train` field to `shift_id`.
/// 2. The next candidate run is found by searching for the earliest available
///    run in the **opposite** direction whose departure time falls within a
///    60-second window after `current_departure + line_duration`.  This models
///    the train arriving at the far terminal and immediately picking up the
///    return working.
/// 3. Steps 1–2 repeat until no further connectable run can be found.
///
/// Runs are looked up and mutated directly in the shared `runs` map so that
/// allocated runs are invisible to subsequent calls within the same day.
///
/// # Parameters
/// - `runs`      – mutable map of all unallocated runs for the day, keyed by
///                 `(departure_time_seconds, LineDir)`.  Matched runs have their
///                 `train` field set to `shift_id` in place.
/// - `lines`     – line-definition index (`(line_id, direction)` → `Line`) used
///                 to look up the one-way running time (`duration`) of each run.
/// - `start_run` – key of the first run in the chain `(time_seconds, direction)`.
/// - `shift_id`  – numeric identifier written into each allocated run's `train`
///                 field; corresponds to the shift index in the outer loop.
///
/// # Returns
/// `Some(Vec<Run>)` – the ordered sequence of runs that form the shift, or
/// `None` if the starting run cannot be retrieved from the map.
pub fn get_way(
    runs: &mut std::collections::BTreeMap<(i64, line::LineDir), run::Run>, 
    lines: &std::collections::HashMap<(String, String), line::Line>, 
    start_run: (i64, line::LineDir),
    shift_id: i64
) -> Option<Vec<run::Run>> {
    // Take a snapshot of unallocated runs so that look-ahead queries are not
    // affected by the allocations we are about to make in `runs`.
    let copy_runs = runs.clone();
    let mut now_run = runs.get_mut(&start_run);
    let mut res: Vec<run::Run> = vec![];
    while now_run != None {
        let run_loop = now_run?;
        // Record this run in the result and mark it as allocated.
        res.push(run_loop.clone());
        run_loop.train = Some(shift_id as u32);
        let time = run_loop.time.0.0;
        // Look up the travel duration for the current run's line and direction.
        let run_line = lines[&(run_loop.line.clone(), run_loop.side.to_string())].clone();
        // Find the next connectable run: departs from the opposite terminal
        // within 60 seconds after this run's arrival time.
        now_run = match run::get_first_after_with_timelimit_side(&copy_runs, time + run_line.duration.0.0, 60, run_loop.side.invert().clone()){
            Some(data) => runs.get_mut(&data),
            None => None
        };
    }   
    Some(res)
}

/// Inspects every train's maintenance countdown timers and injects service
/// events into the day's shift records when a service falls due.
///
/// This function is called once per day **before** revenue-service assignments
/// are made.  It performs six sequential passes over the train roster:
///
/// 1. **Type-A service** (`time_to_type_a < 86400`): light 1-day check at the
///    home depot.  Resets the A-counter to 30 days and advances `free_time` by
///    1 day.
/// 2. **Type-B service** (`time_to_type_b < 86400`): light 3-day overhaul at
///    the home depot.  Resets the A- and B-counters and advances `free_time`
///    by 3 days.
/// 3. **Type-C service** (`time_to_type_c < 86400`): medium 14-day overhaul at
///    a specialist depot.  Resets the A-, B- and C-counters and advances
///    `free_time` by 14 days.
/// 4. **Type-D service** (`time_to_type_d < 86400`): capital 45-day overhaul
///    at the manufacturer.  Resets all four counters and advances `free_time`
///    by 45 days.
/// 5. **End-of-service detection**: trains currently in `Place::Service` whose
///    remaining service time drops below 1 day are moved back to
///    `Place::Depot` with an extra 1-hour buffer added to `free_time`.
/// 6. **Service-time tick**: for all trains still in `Place::Service`, the
///    remaining service duration is decremented by one day (86 400 seconds).
///
/// Each service event is recorded in the train's `TrainShift` as a
/// `ShiftType::Service(ServiceType::*)` entry.
///
/// # Parameters
/// - `trains`          – mutable roster of all trains; place, free-time and
///                       countdown fields are updated in place.
/// - `train_day_shifts`– mutable map of today's `TrainShift` records; service
///                       `ShiftType` entries are appended to the matching record.
pub fn service_look(
    trains: &mut std::collections::BTreeMap<(String, types::TrainId), train::Train>,
    train_day_shifts: &mut std::collections::BTreeMap<(String, types::TrainId), train::TrainShift>,
) {
    // look on type A 
    
    // Collect all trains whose Type-A countdown has dropped below one day.
    let type_a_trains = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
        if obj.time_to_type_a.0.0 < 24*60*60 {Some((tr_id, obj))} else {None}
    }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();

    for (tr_id, _tr_data) in type_a_trains.into_iter() {
        let train_conf = match trains.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        let train_sh = match train_day_shifts.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        train_sh.shift.push(run::ShiftType::Service(run::ServiceType::A));
        train_conf.place = train::Place::Service((types::Hms(types::Seconds(24*60*60)), run::ServiceType::A));
        train_conf.time_to_type_a.0.0 = 30*24*60*60;
        train_conf.free_time.0.0 = train_conf.free_time.0.0 + 24*60*60;
    }
    // look on type B
    // Collect all trains whose Type-B countdown has dropped below one day.
    let type_b_trains = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
        if obj.time_to_type_b.0.0 < 24*60*60 {Some((tr_id, obj))} else {None}
    }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();

    for (tr_id, _tr_data) in type_b_trains.into_iter() {
        let train_conf = match trains.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        let train_sh = match train_day_shifts.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        train_sh.shift.push(run::ShiftType::Service(run::ServiceType::B));
        train_conf.place = train::Place::Service((types::Hms(types::Seconds(3*24*60*60)), run::ServiceType::B));
        train_conf.time_to_type_a.0.0 = 30*24*60*60;
        train_conf.time_to_type_b.0.0 = 4*30*24*60*60;
        train_conf.free_time.0.0 = train_conf.free_time.0.0 + 3*24*60*60;
    }
    // look on type C
    // Collect all trains whose Type-C countdown has dropped below one day.
    let type_c_trains = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
        if obj.time_to_type_c.0.0 < 24*60*60 {Some((tr_id, obj))} else {None}
    }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();

    for (tr_id, _tr_data) in type_c_trains.into_iter() {
        let train_conf = match trains.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        let train_sh = match train_day_shifts.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        train_sh.shift.push(run::ShiftType::Service(run::ServiceType::C));
        train_conf.place = train::Place::Service((types::Hms(types::Seconds(14*24*60*60)), run::ServiceType::C));
        train_conf.time_to_type_a.0.0 = 30*24*60*60;
        train_conf.time_to_type_b.0.0 = 4*30*24*60*60;
        train_conf.time_to_type_c.0.0 = 15*30*24*60*60;
        train_conf.free_time.0.0 = train_conf.free_time.0.0 + 14*24*60*60;
    }
    // look on type D
    // Collect all trains whose Type-D countdown has dropped below one day.
    let type_d_trains = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
        if obj.time_to_type_d.0.0 < 24*60*60 {Some((tr_id, obj))} else {None}
    }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();

    for (tr_id, _tr_data) in type_d_trains.into_iter() {
        let train_conf = match trains.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        let train_sh = match train_day_shifts.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        train_sh.shift.push(run::ShiftType::Service(run::ServiceType::D));
        train_conf.place = train::Place::Service((types::Hms(types::Seconds(45*24*60*60)), run::ServiceType::D));
        train_conf.time_to_type_a.0.0 = 30*24*60*60;
        train_conf.time_to_type_b.0.0 = 4*30*24*60*60;
        train_conf.time_to_type_c.0.0 = 15*30*24*60*60;
        train_conf.time_to_type_d.0.0 = 5*365*24*60*60;
        train_conf.free_time.0.0 = train_conf.free_time.0.0 + 45*24*60*60;
    }

    // End-of-service pass: find trains in Place::Service whose remaining
    // service duration has fallen below one day and return them to the depot.
    // look for end service
    let end_service_train = trains.clone().into_iter().filter_map(|(tr_id, obj)| {
        match obj.place.clone() {
            train::Place::Service((time, _service_type)) => {
                //dbg!((tr_id.clone(), time.clone()));
                if time.0.0 < 24*60*60 {
                    Some((tr_id, obj))
                } else {
                    None
                }
            },
            _ => None
        }
    }).collect::<std::collections::BTreeMap<(String, types::TrainId), train::Train>>();
        //dbg!(end_service_train.clone());
    for (tr_id, _tr_data) in end_service_train.into_iter() {
        let train_conf = match trains.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        let train_sh = match train_day_shifts.get_mut(&tr_id) {
            Some(data) => data,
            None => panic!()
        };
        train_sh.shift.push(run::ShiftType::Depot);
        train_conf.place = train::Place::Depot(train_conf.depot.clone());
        train_conf.free_time.0.0 = train_conf.free_time.0.0 + 60*60;
    }

    // Daily service-time tick: decrement the remaining service duration of
    // every train still in Place::Service by one day (86 400 seconds).
    // change time for service

    for (tr_id, tr_data) in trains.clone().iter_mut() {
        
        match tr_data.place.clone() {
            train::Place::Service((time, service_type)) => {
                let train_conf = match trains.get_mut(tr_id) {
                    Some(data) => data,
                    None => panic!()
                };
                train_conf.place = train::Place::Service((types::Hms(types::Seconds(time.0.0 - 24*60*60)), service_type));
                //dbg!((tr_id.clone(), tr_data.place.clone()));
            },
            _ => {}
        }
    }
}