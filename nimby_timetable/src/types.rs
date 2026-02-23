
use serde::{Serialize, Deserialize};


use serde::{Serializer, Deserializer};
use serde::de::Error as DeError;

/// Alias for a numeric train identifier.
///
/// Using a dedicated type alias (rather than a bare `u32`) makes function
/// signatures self-documenting and allows the underlying integer type to be
/// changed in one place if needed.
pub type TrainId = u32;

/// A newtype wrapper around a raw second count (`i64`).
///
/// Storing durations and timestamps as a distinct type instead of a plain
/// integer prevents accidental mixing of unrelated numeric values and makes
/// the intent of a variable explicit in function signatures.
///
/// The inner `i64` represents a number of seconds and may be:
/// - an absolute offset from the start of the scheduling week (0 … 7×86400), or
/// - a duration / interval between two events.
#[derive(Copy, Clone, Eq, PartialEq, Ord, PartialOrd, Hash, Debug)]
pub struct Seconds(pub i64);

impl Seconds {
    /// Creates a `Seconds` value normalised to a single 24-hour day.
    ///
    /// Applies `(time + 86400) % 86400` so that negative offsets wrap
    /// correctly into the `[0, 86400)` range.
    ///
    /// # Parameters
    /// - `time` – a raw second offset, potentially negative or ≥ 86400.
    ///
    /// # Returns
    /// A `Seconds` whose inner value is always in `[0, 86400)`.
    pub fn _new(time: i64) -> Self {
        Self((time + 24*60*60) % (24*60*60))
    }
}

/// A human-readable time wrapper expressed as **HH:MM:SS**.
///
/// `Hms` wraps a [`Seconds`] value and provides custom serde implementations
/// so that it round-trips through RON (and any other serde format) as the
/// string `"HH:MM:SS"` rather than as a raw integer.
///
/// Hours are not clamped to 24, which allows `Hms` to represent durations
/// longer than one day (e.g. `"168:00:00"` for a full week).
#[derive(Copy, Clone, Debug, PartialEq, PartialOrd)]
pub struct Hms(pub Seconds);

impl Serialize for Hms {
    /// Serialises the time value as the string `"HH:MM:SS"`.
    ///
    /// Hours are derived from `total_seconds / 3600` and are not clamped,
    /// so values exceeding 23 are written literally (e.g. `"25:00:00"`).
    /// Minutes and seconds are always zero-padded to two digits.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where S: Serializer {
        let h = self.0.0 / (60 * 60);
        let m = self.0.0 / 60 % 60;
        let s = self.0.0 % 60;
        serializer.serialize_str(&format!("{h:02}:{m:02}:{s:02}"))
    }
}

impl<'de> Deserialize<'de> for Hms {
    /// Deserialises an `"HH:MM:SS"` string back into an `Hms` value.
    ///
    /// # Accepted format
    /// A colon-separated string with three numeric components:
    /// - `HH` – hours (any non-negative integer, no upper bound enforced).
    /// - `MM` – minutes in `[0, 59]`; returns an error if ≥ 60.
    /// - `SS` – seconds in `[0, 59]`; returns an error if ≥ 60.
    ///
    /// The resulting `Hms` wraps `HH * 3600 + MM * 60 + SS` as a [`Seconds`].
    ///
    /// # Errors
    /// Returns a deserialisation error if any component is missing, not a
    /// valid integer, or out of range.
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where D: Deserializer<'de> {
        let value = String::deserialize(deserializer)?;
        let mut parts = value.split(':');
        let h = parts.next().ok_or_else(|| DeError::custom("hours component not found"))?;
        let h: i64 = h.parse().map_err(|_| DeError::custom("hours component is not a number"))?;
        
        let m = parts.next().ok_or_else(|| DeError::custom("minutes component not found"))?;
        let m: i64 = m.parse().map_err(|_| DeError::custom("minutes component is not a number"))?;
        if m >= 60 {
            return Err(serde::de::Error::custom("minutes value should be less than 60"));
        }
        
        let s = parts.next().ok_or_else(|| DeError::custom("seconds component not found"))?;
        let s: i64 = s.parse().map_err(|_| DeError::custom("seconds component is not a number"))?;
        if s >= 60 {
            return Err(serde::de::Error::custom("seconds value should be less than 60"));
        }
        
        Ok(Hms(Seconds(h * 60 * 60 + m * 60 + s)))
    }
}
