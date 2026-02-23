use std::collections::BTreeSet;
use std::fs::File;

use csv::ReaderBuilder;

/// A collection of device control events stored as `(timestamp_ms, event_code)` pairs.
///
/// Internally backed by a [`BTreeSet`] so all events are kept in chronological order
/// automatically. The outer tuple field is public to allow direct iteration when needed.
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ControlList(
    pub BTreeSet<(u128, String)>
);

impl ControlList {
    /// Constructs a [`ControlList`] by reading events from a semicolon-delimited CSV file.
    ///
    /// # Expected file format
    /// Each row must contain exactly two fields separated by `';'`:
    /// ```text
    /// timestamp_ms;event_code
    /// 1708600000000;1
    /// 1708600005000;2
    /// ```
    ///
    /// # Panics
    /// Panics if the file cannot be opened or if any row cannot be deserialized into
    /// `(u128, String)`.
    ///
    /// # Arguments
    /// * `path` – Absolute or relative path to the CSV file.
    pub fn from_path(path: &String) -> Self {
        // Resulting collection; BTreeSet keeps entries sorted by timestamp automatically.
        let mut out_map = BTreeSet::new();

        // Open the file at the given path.
        let file = File::open(path).unwrap();

        // Build a CSV reader that uses ';' as the field delimiter instead of the default ','.
        let mut rdr = ReaderBuilder::new()
            .delimiter(b';')
            .from_reader(file);

        // Deserialize each CSV row into a (timestamp, event_code) tuple and collect them.
        for result in rdr.deserialize() {
            let record: (u128, String) = result.unwrap();
            out_map.insert(record);
        }

        Self(out_map)
    }

    /// Returns the most recent valid device event code that occurred **before** `time`.
    ///
    /// Only events with codes `"1"`, `"2"`, or `"3"` are considered valid.
    /// Events at exactly `time` or after it are ignored.
    /// If no qualifying event is found the method returns `"1"` as a safe default,
    /// which represents the *device off* / *idle* state.
    ///
    /// # Arguments
    /// * `time` – Upper-exclusive timestamp boundary in milliseconds.
    ///
    /// # Returns
    /// The last valid event code (`"1"`, `"2"`, or `"3"`) seen before `time`, or
    /// `"1"` if none exists.
    pub fn get_last_device_event(&self, time: u128) -> String {
        self.0.iter()
            // Walk all events in ascending order, carrying the last accepted code forward.
            .fold("1".to_string(), |acc, item| {
                // Only consider events that occurred strictly before the requested time.
                if item.0 < time {
                    // Accept only the three recognised device state codes.
                    if item.1 == "1" || item.1 == "2" || item.1 == "3" {
                        // This event is more recent and valid — update the accumulator.
                        item.1.to_string()
                    } else {
                        // Unknown code: retain the previous valid state.
                        acc
                    }
                } else {
                    // Event is at or after the target time — skip it.
                    acc
                }
            })
    }
}

/// Simple addition helper used in unit tests.
///
/// # Arguments
/// * `left`  – First operand.
/// * `right` – Second operand.
///
/// # Returns
/// The sum `left + right`.
pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Verifies that [`add`] returns the correct sum for basic inputs.
    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
