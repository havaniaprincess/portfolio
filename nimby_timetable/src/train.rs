use crate::{line, run::{self, Run, ShiftType}, types::{self, Hms, Seconds}};

use serde::{Serialize, Deserialize};
use rand::prelude::*;

/// Describes the current physical location and activity state of a train.
///
/// - `Depot(String)` – the train is stabled at the named depot and is not in
///   revenue service.  The `String` payload is the depot identifier (matches
///   `Train::depot`).
/// - `Station(LineDir)` – the train has just completed a run and is waiting at
///   the terminal station associated with the given direction.  `LineDir` records
///   which end of the line the train is on so the scheduler knows which direction
///   the next run should depart in.
/// - `Service((Hms, ServiceType))` – the train is undergoing scheduled
///   maintenance.  The `Hms` payload is the expected end time of the service
///   event and `ServiceType` identifies the maintenance tier (A–D).
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Place{
    Depot(String),
    Station(line::LineDir), // last route dir
    Service((types::Hms, run::ServiceType))
}

/// A display-friendly version of [`TrainShift`] used for serialisation to output files.
///
/// Identical in layout to `TrainShift` but all `RunShift` departure times are
/// normalised to a local-day clock offset:
/// `(absolute_seconds % 86400) + 4*3600`
/// so that times are expressed relative to the operating day start (04:00)
/// rather than the raw week-second counter.
///
/// Fields:
/// - `id`    – the numeric train identifier.
/// - `depot` – the depot/line identifier string this train belongs to.
/// - `shift` – the ordered sequence of [`run::ShiftType`] activities for the day,
///             with `RunShift` times remapped to the local operating-day clock.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainShiftOut {
    pub id: types::TrainId,
    pub depot: String,
    pub shift: Vec<run::ShiftType>
}

impl TrainShiftOut {
    /// Converts a [`TrainShift`] into a `TrainShiftOut` by remapping all
    /// `RunShift` departure times to the local operating-day clock.
    ///
    /// For every `RunShift` variant the absolute week-second timestamp of each
    /// [`Run`] is converted via:
    /// ```text
    /// display_time = (absolute_seconds % 86400) + 4 * 3600
    /// ```
    /// which places times within the 04:00–04:00 operating day window.
    /// All other `ShiftType` variants are cloned unchanged.
    ///
    /// # Parameters
    /// - `shift` – the raw `TrainShift` to convert (consumed by value).
    ///
    /// # Returns
    /// A new `TrainShiftOut` with the same `id` and `depot` as the input and
    /// the remapped `shift` sequence.
    pub fn new(shift: TrainShift) -> Self {
        let new_shifts = shift.shift.iter()
            .map(|obj| {
                match obj {
                    run::ShiftType::RunShift(data) => {
                        run::ShiftType::RunShift(data.iter()
                            .map(|rr| {
                                Run { id: rr.id.clone(), time: Hms(Seconds((rr.time.0.0 % (24*60*60)) + 4*60*60)), train: rr.train.clone(), line: rr.line.clone(), able: rr.able, side: rr.side.clone() }
                            }).collect::<Vec<Run>>()
                        )
                    },
                    _ => obj.clone()
                }
            }).collect::<Vec<ShiftType>>();
        Self { 
            id: shift.id.clone(), 
            depot: shift.depot.clone(), 
            shift: new_shifts,
        }
    }
}

/// Records everything a single train does during one calendar day.
///
/// A `TrainShift` is built progressively during the assignment loop in
/// `main`: service moves are injected first by `way_getting::service_look`,
/// then revenue `RunShift` and `Depot` entries are appended as shifts are
/// matched to trains.
///
/// Fields:
/// - `id`    – numeric train identifier, mirrors `Train::id`.
/// - `depot` – the depot/line string the train is associated with.
/// - `shift` – ordered list of [`run::ShiftType`] activities representing the
///             full duty sequence for the day.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct TrainShift {
    pub id: types::TrainId,
    pub depot: String,
    pub shift: Vec<run::ShiftType>
}

impl TrainShift {
    /// Creates a new, empty `TrainShift` for the given train.
    ///
    /// # Parameters
    /// - `id`    – numeric identifier of the train.
    /// - `depot` – depot/line identifier string.
    ///
    /// The `shift` vector is initialised as empty; activities are pushed
    /// into it during the day-assignment loop in `main`.
    pub fn new(id: types::TrainId, depot: String) -> Self {
        
        Self { 
            id: id, 
            depot: depot.clone(), 
            shift: vec![],
        }
    }
}

/// Represents a single rolling-stock unit and its full operational state.
///
/// Fields:
/// - `id`            – unique numeric identifier for this train.
/// - `depot`         – the depot/line string this train is home-based at.
/// - `work_time`     – cumulative revenue running time accumulated over the
///                     train's lifetime (used to track total mileage).
/// - `time_to_type_a`– countdown to the next **Type-A** service: light check,
///                     1 day, performed at the home depot every ~30 days.
/// - `time_to_type_b`– countdown to the next **Type-B** service: light overhaul,
///                     3 days, performed at the home depot every ~4 months.
/// - `time_to_type_c`– countdown to the next **Type-C** service: medium overhaul,
///                     14 days, performed at a specialist depot every ~15 months.
/// - `time_to_type_d`– countdown to the next **Type-D** service: capital
///                     overhaul, 45 days, performed at the manufacturer every
///                     ~5 years.
/// - `dam_a_prob`    – per-second probability of a minor (A-class) random fault.
/// - `dam_b_prob`    – per-second probability of a moderate (B-class) random fault.
/// - `dam_c_prob`    – per-second probability of a serious (C-class) random fault.
/// - `dam_d_prob`    – per-second probability of a critical (D-class) random fault.
/// - `place`         – current location/state of the train (see [`Place`]).
/// - `free_time`     – absolute second at which the train next becomes
///                     available for assignment after its current activity ends.
/// - `home_station`  – the terminal direction (`Right` or `Back`) this train
///                     is normally associated with; used when filtering depot
///                     candidates for a shift.
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Train {
    pub id: types::TrainId,
    pub depot: String,
    pub work_time: types::Hms,
    pub time_to_type_a: types::Hms, // light service for 1 day in usual depot every 30 day
    pub time_to_type_b: types::Hms, // light service for 3 day in usual depot every 4 month
    pub time_to_type_c: types::Hms, // medium service for 14 day in special depot every 1 year and 3 month
    pub time_to_type_d: types::Hms, // capital servic for 45 day in manufactory every 5 year
    pub dam_a_prob: f64,
    pub dam_b_prob: f64,
    pub dam_c_prob: f64,
    pub dam_d_prob: f64,
    pub place: Place,
    pub free_time: types::Hms,
    pub home_station: line::LineDir
}

impl Train {
    /// Creates a new `Train` with randomised initial state, simulating a fleet
    /// that is already mid-lifecycle at the start of the simulation.
    ///
    /// Randomisation is applied to:
    /// - `work_time` – sampled uniformly in `[0, 5 years)` to spread fleet age.
    /// - `time_to_type_a` – sampled in `[0, 30 days)`.
    /// - `time_to_type_b` – sampled in `[0, 120 days)`.
    /// - `time_to_type_c` – sampled in `[0, 15 months)`.
    /// - `time_to_type_d` – set to `5 years − work_time` so that the D-overhaul
    ///   due date aligns with the simulated age of the train.
    ///
    /// All damage probabilities are initialised to fixed baseline values.
    /// The train is placed in the named depot with `free_time = 0` (available
    /// immediately from the start of the schedule).
    ///
    /// # Parameters
    /// - `id`      – unique numeric identifier for the new train.
    /// - `depot`   – depot/line identifier string.
    /// - `station` – the terminal direction this train is home-based at.
    pub fn _new(id: types::TrainId, depot: String, station: line::LineDir) -> Self {
        
        let mut rng = rand::thread_rng();
        let y = types::Hms(types::Seconds((rng.gen::<f64>() * (5.0*365.0*24.0*60.0*60.0)) as i64)); 
        Self { 
            id: id, 
            depot: depot.clone(), 
            work_time: y, 
            time_to_type_a: types::Hms(types::Seconds((rng.gen::<f64>() * (30.0*24.0*60.0*60.0)) as i64)), 
            time_to_type_b: types::Hms(types::Seconds((rng.gen::<f64>() * (120.0*24.0*60.0*60.0)) as i64)), 
            time_to_type_c: types::Hms(types::Seconds((rng.gen::<f64>() * (15.0*30.0*24.0*60.0*60.0)) as i64)), 
            time_to_type_d: types::Hms(types::Seconds((5.0*365.0*24.0*60.0*60.0) as i64 - y.0.0)), 
            dam_a_prob: 0.002, 
            dam_b_prob: 0.0001, 
            dam_c_prob: 0.000001, 
            dam_d_prob: 0.00000001,
            place: Place::Depot(depot.clone()),
            free_time: types::Hms(types::Seconds(0)),
            home_station: station
        }
    }
}