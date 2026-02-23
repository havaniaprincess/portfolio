use crate::{line::{Line, LineDir}, run::{Run, RunArray}, way_getting::get_way};

/// Groups a week's worth of individual departure runs into driver shifts,
/// organised by day and then by shift index.
///
/// # Algorithm
///
/// For each of the 7 days in the week (day 0 … 6):
///
/// 1. **Slice** the full-week `runs_right` and `runs_back` arrays down to only
///    the runs whose departure time falls within that calendar day
///    (`[day × 86400, (day+1) × 86400)` seconds).
/// 2. **Merge** both directional slices into a single map keyed by
///    `(departure_time_seconds, LineDir)` for O(log n) look-ups.
/// 3. **Scan** every second of the day in chronological order.  Whenever an
///    unallocated, available run is found at the current second, call
///    [`way_getting::get_way`] to build the full shift sequence starting from
///    that run.  Each resulting sequence is stored under an auto-incrementing
///    `shift_id`.
/// 4. After processing all seconds, store the day's shift map in the outer
///    `day_shifts` map under the day index.
///
/// # Parameters
/// - `runs_right` – all generated forward-direction runs for the full 7-day week.
/// - `runs_back`  – all generated return-direction runs for the full 7-day week.
/// - `lines`      – line-definition index (`(line_id, direction)` → `Line`)
///                  forwarded to `get_way` for headway and duration look-ups.
///
/// # Returns
/// A nested `BTreeMap`:
/// ```text
/// day_index (0–6)  →  shift_index  →  Vec<Run>
/// ```
/// where each `Vec<Run>` is the ordered sequence of runs that make up one
/// driver shift on that day.
pub fn get_shifts(
    runs_right: &RunArray,
    runs_back: &RunArray,
    lines: &std::collections::HashMap<(String, String), Line>
) -> std::collections::BTreeMap<i64, std::collections::BTreeMap<i64, Vec<Run>>> {
    let mut day_shifts: std::collections::BTreeMap<i64, std::collections::BTreeMap<i64, Vec<Run>>> = std::collections::BTreeMap::new();
    for day in 0..7 {
        // Filter the full-week run arrays down to only this calendar day.
        let day_runs_right: RunArray = RunArray(runs_right.0.clone().into_iter().filter_map(|obj| if obj.time.0.0 >= day * 24 * 60 * 60 && obj.time.0.0 < (day + 1) * 24 * 60 * 60 { Some(obj)} else {None} ).collect());
        let day_runs_back: RunArray = RunArray(runs_back.0.clone().into_iter().filter_map(|obj| if obj.time.0.0 >= day * 24 * 60 * 60 && obj.time.0.0 < (day + 1) * 24 * 60 * 60 { Some(obj)} else {None} ).collect());
        // Merge both directions into a single map keyed by (time_seconds, direction).
        let days_runs_vec = RunArray([day_runs_right.0, day_runs_back.0].concat());
        let mut days_runs: std::collections::BTreeMap<(i64, LineDir), Run> = days_runs_vec.0.into_iter().map(|obj| ((obj.time.0.0, obj.side.clone()), obj)).collect();
        let mut shift_id = 0;
        let mut shifts: std::collections::BTreeMap<i64, Vec<Run>> = std::collections::BTreeMap::new();
        // Walk through every second of the day and start a new shift for each
        // unallocated run found at that exact second.
        for sec in 0..24*60*60 {
            let run_now = days_runs.clone().into_iter().filter_map(|(indx, obj)| if obj.able && obj.train == None && obj.time.0.0 == day*24*60*60+sec {Some(indx)} else {None}).collect::<Vec<(i64, LineDir)>>();

            for r_i in run_now.iter() {
                // Build the full shift sequence starting from this run.
                // `get_way` marks consumed runs as allocated and chains
                // subsequent runs until the shift ends.
                let shift_data = match get_way(&mut days_runs, &lines, r_i.clone(), shift_id) {
                    Some(data) => data,
                    None => vec![]
                };
                shifts.insert(shift_id, shift_data);
                shift_id += 1;
            }
        }
        day_shifts.insert(day, shifts);



    }
    return day_shifts;
}