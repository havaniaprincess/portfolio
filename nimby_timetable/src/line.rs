

use crate::{math, run, types};

use serde::{Serialize, Deserialize};

/// Represents the direction of travel along a transit line.
///
/// A line can run in the forward (`Right`) direction or the return (`Back`) direction.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd, Eq, Ord, Copy)]
pub enum LineDir{
    Right,
    Back
}

impl LineDir {
    /// Returns the direction as a human-readable string: `"right"` or `"back"`.
    pub fn to_string(&self) -> String {
        
        match self {
            LineDir::Right => "right".to_string(),
            LineDir::Back => "back".to_string()
        }
    }

    /// Returns the opposite direction.
    ///
    /// `Right` becomes `Back` and `Back` becomes `Right`.
    pub fn invert(&self) -> Self {
        match self {
            LineDir::Right => LineDir::Back,
            LineDir::Back => LineDir::Right
        }
    }
}

/// Describes a transit line with its scheduling parameters.
///
/// A `Line` holds all the information needed to generate departure runs for a
/// full week (7 days):
/// - `id`: unique identifier of the line.
/// - `direction`: the direction of travel (`Right` or `Back`).
/// - `base_time`: the default headway (interval between departures) for the whole week.
/// - `depth`: a list of time-range overrides that shorten the headway during peak hours;
///   each entry is `(divisor, range_start, range_end)` where the effective headway is
///   `base_time / divisor`.
/// - `duration`: one-way travel time from the first stop to the last stop.
/// - `way_time`: currently stored but used for additional way-time calculations.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, PartialOrd)]
pub struct Line {
    pub id: String,
    pub direction: LineDir,
    pub base_time: types::Hms,
    pub depth: Vec<(u16, types::Hms, types::Hms)>,
    pub duration: types::Hms,
    pub way_time: types::Hms,
} 

impl Line {
    /// Constructs a new `Line` with all fields explicitly provided.
    ///
    /// # Parameters
    /// - `id` – unique string identifier for this line.
    /// - `base_time` – default headway between consecutive runs (as `Hms`).
    /// - `depth` – slice of peak-hour overrides `(divisor, start, end)`; cloned internally.
    /// - `duration` – one-way running time of the line.
    /// - `way_time` – additional way time associated with the line.
    /// - `direction` – the direction (`Right` or `Back`) this line operates in.
    pub fn _new(
        id: String,
        base_time: types::Hms,
        depth: &Vec<(u16, types::Hms, types::Hms)>,
        duration: types::Hms,
        way_time: types::Hms,
        direction: LineDir, 
    ) -> Self {
        Self { 
            id: id,
            base_time: base_time,
            depth: depth.clone(),
            duration: duration,
            direction: direction.clone(),
            way_time: way_time
        }
    }

    /// Returns a reference to the line's unique identifier string.
    pub fn get_id(&self) -> &String {
        &self.id
    }

    /// Returns the line's direction as a human-readable string: `"right"` or `"back"`.
    pub fn get_dir(&self) -> String {
        match self.direction {
            LineDir::Right => "right".to_string(),
            LineDir::Back => "back".to_string()
        }
    }

    /// Generates all departure runs for this line over a full 7-day week.
    ///
    /// The algorithm works as follows:
    /// 1. A full second-by-second timetable for the week is initialised with the
    ///    line's `base_time` headway at every second.
    /// 2. Each entry in `depth` overrides the headway within its time range
    ///    to `base_time / divisor`, using the minimum of the existing and the
    ///    new value so that multiple overlapping ranges are handled correctly.
    /// 3. The week is then divided into slots of size `duration + duration_back`
    ///    seconds. Within each slot every second whose offset is divisible by the
    ///    effective headway at that second becomes a departure time.
    /// 4. The starting offset of the first slot depends on `dir`: `Right` starts at
    ///    second 0, while `Back` starts at `duration_back - step` to interleave
    ///    return trips with forward trips.
    ///
    /// # Parameters
    /// - `duration_back` – the return-leg travel time in seconds. Combined with
    ///   `self.duration` it determines the full round-trip cycle length.
    /// - `dir` – which direction to generate runs for (`Right` or `Back`).
    ///
    /// # Returns
    /// A [`run::RunArray`] containing all generated [`run::Run`] entries in the
    /// order they were produced (the internal debug vector is sorted, but the
    /// result vector preserves insertion order).
    pub fn get_runs(&self, duration_back: i64, dir: LineDir) -> run::RunArray {
        let mut res: Vec<run::Run> = vec![];
        let mut timetable_full: std::collections::BTreeMap<i64, i64> = std::collections::BTreeMap::new();

        for sec_i in 0..7*24*60*60 {
            timetable_full.insert(sec_i, self.base_time.0.0);
        }

        for depth_i in self.depth.iter() {

            for sec_i in depth_i.1.0.0..depth_i.2.0.0 {
                let now = timetable_full[&sec_i].clone();
                timetable_full.insert(sec_i, math::min(self.base_time.0.0 / depth_i.0 as i64, now));
            }
        }

        let step = self.duration.0.0 + duration_back;
        let mut start_time = if dir == LineDir::Right {0} else {0 + duration_back - step}  as i64;


        let mut runs: Vec<i64> = vec![];

        while start_time < 7*24*60*60 - step {
            for i in 0..step {
                if start_time + i >= 7*24*60*60 {
                    break;
                }
                if start_time + i < 0 {
                    continue;
                }
                let now_time_i = (start_time + i) % (7*24*60*60);
                let now = timetable_full[&now_time_i].clone();
                if i % now == 0 {
                    runs.push(now_time_i);
                    res.push(run::Run::new(&self.id.clone(), &types::Hms(types::Seconds(now_time_i)), dir.clone()));
                }
            }

            start_time += step;
        }
        runs.sort();

        
        //dbg!(runs.clone());
        run::RunArray(res)
    }
}