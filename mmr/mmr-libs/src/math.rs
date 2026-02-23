use core::f64;

use crate::types::{MMRChangeDebug, MMRType};


/// Signed power curve: preserves the sign of `x`, scales by `a` and shifts by `b`.
///
/// Formula: `sign(x) * a * |x|^p + b`
///
/// Useful for smooth, non-linear transformations of signed MMR deltas.
pub fn sqr_pow(x: f64, p: f64, a: f64, b: f64) -> f64 {
    (if x < 0.0 {-1.0} else {1.0}) * a*x.abs().powf(p) + b
}

/// Arctangent-based smoothing: maps any real input to a bounded range around `b`.
///
/// Formula: `a * atan(x) + b`
pub fn atan_pow(x: f64, a: f64, b: f64) -> f64 {
    a*x.atan()+b
}

/// Arcsine-based transformation helper.
///
/// Formula: `a * asin(x) + b`. `x` must be in `[-1, 1]`.
pub fn asin_pow(x: f64, a: f64, b: f64) -> f64 {
    a*x.asin()+b
}

/// Generic power curve.
///
/// Formula: `a * x^p + b`
pub fn cub_func(x: f64, p: f64, a: f64, b: f64) -> f64 {
    a*x.powf(p)+b
}

/// Returns the smaller of two totally ordered values.
pub fn min<T>(a: T, b: T) -> T
where 
    T: Ord+Eq
{
    if a > b {
        return b;
    } else {
        return a;
    }
}

/// Returns the smaller of two partially ordered values (e.g. `f64`).
pub fn minf<T>(a: T, b: T) -> T
where 
    T: PartialEq+PartialOrd
{
    if a > b {
        return b;
    } else {
        return a;
    }
}

/// Returns the larger of two totally ordered values.
pub fn max<T>(a: T, b: T) -> T
where 
    T: Ord+Eq
{
    if a < b {
        return b;
    } else {
        return a;
    }
}

/// Returns the larger of two partially ordered values (e.g. `f64`).
pub fn maxf<T>(a: T, b: T) -> T
where 
    T: PartialEq+PartialOrd
{
    if a < b {
        return b;
    } else {
        return a;
    }
}

/// Safe division that avoids extreme spikes when the denominator is near zero.
///
/// Returns `1.0` when `down` is effectively zero (below `1e-8`) or equal to `top`,
/// preventing division-by-zero and infinite amplification in rating formulas.
pub fn divide_or_0(top: f64, down: f64) -> f64 {
    if down == top || down - 0.0 < 0.00000001 {
        1.0
    } else {
        top / down
    }
}

/// Parametric logistic (sigmoid) curve.
///
/// Formula: `c / (1 + e^(-a*(x-b))) + d`
///
/// Parameters:
/// - `a` — steepness of the curve
/// - `b` — horizontal midpoint (inflection point)
/// - `c` — output amplitude
/// - `d` — vertical offset
pub fn sigmoid(x: f64, a: f64, b: f64, c: f64, d: f64) -> f64 {
    c / (1.0 + std::f64::consts::E.powf(/* -5 */-a * (x - b))) + d
}

/// Dynamic low-rank attenuation sigmoid used for MMR weighting.
///
/// Computes a position-based weight in `[0, 1]` for a player at rank `x` out of `max`
/// ranked players. Players near the bottom of the distribution receive a lower weight,
/// reducing their influence on the weighted average MMR computation.
pub fn sigmoid_mmr_low_place(x: f64, max: f64) -> f64 {
    let n = if max - 0.0 < 0.000001 {max} else {0.99 * max / x};
    if x - 0.0 < 0.00000001 {1.0} else {-n / (1.0 + std::f64::consts::E.powf(/* -5 */-(1.5*n)* ((x / max)-0.5))) + (n + 1.0)}
}

/// Computes a weighted average MMR from a set of opponent MMR values (typically top-3).
///
/// Each `MMRType::MMR` value is weighted by a positional attenuation coefficient from
/// [`sigmoid_mmr_low_place`], which down-weights lower-ranked opponents. Provisional
/// (`NotEnought`) and absent (`None`) values are ignored.
///
/// Returns `None` when the input is empty, contains no calibrated values, or fewer than
/// 3 valid values are present.
pub fn avg_3(mmrs: Vec<MMRType>) -> Option<u32> {
    let fl = true;
    let max = mmrs.iter().filter_map(|obj| match obj {
        MMRType::MMR(d) => Some(*d),
        _ => None
    }).max();
    if mmrs.len() == 0 || max == None {
        return None;
    }
    let mut pairs: Vec<Option<(f64, f64)>> = Vec::new();
    let mut max_x = 0.0 as f64;
    for m in mmrs.iter() {
        let k = match m {
            MMRType::MMR(data) => {
                let k = sigmoid_mmr_low_place(*data as f64, max.unwrap() as f64);
                if (*data as f64) * k > max_x{
                    max_x = (*data as f64)*k;
                }
                Some((*data as f64, k))
            },
            _ => {None}
        };
        pairs.push(k);
    }
    let (sum, count) = pairs.into_iter().filter_map(|obj| obj).fold(
        (0.0,0), |base, other| {
            let res = (base.0 as f64 + if max_x - 0.0 < 0.0000001 {1.0} else {((max.unwrap() as f64) / max_x) * other.0*other.1}, base.1 + 1);
            res
        });
    // Require at least 3 valid calibrated MMR values to avoid noisy estimates.
    if fl && count > 2 {
        Some(((sum as f64) / (count as f64)) as u32)
    } else {
        None
    }
}


/// Computes the v1 per-user MMR delta for a single battle result.
///
/// The delta is built from three additive components:
/// 1. **Score component** — a soft exponential curve on `score`:
///    `50 / e^(1_000_000 / score²)`. Rewards high individual performance.
/// 2. **Matchup pressure** (`mul * ±35`) — a sigmoid term activated only when the
///    MMR gap between the player and the weighted average opponent exceeds 250.
///    Positive for underdogs winning; negative for favorites losing.
/// 3. **Situational modifiers** — flat bonuses/penalties:
///    - early quit: −20
///    - top-20 percent team score: +20
///
/// Returns a tuple of `(mmr_delta, MMRChangeDebug)` where `mmr_delta` is clamped to
/// ≥ 0 on the victory branch.
pub fn diff_mmr(victory: bool, score: i32, top_3: Vec<MMRType>, mmr: MMRType, early_quite: bool, top_20: bool) -> (i32, MMRChangeDebug) {
    if victory {
        // Victory branch: base gain from score + situational modifiers + matchup pressure.
        let score_mmr: i32 = (50.0 / (std::f64::consts::E.powf(1000000.0 / (score as f64).powf(2.0)))) as i32;
        let mmr_diff = match mmr {
            MMRType::MMR(data) => {
                let avg_opp: i32 = match avg_3(top_3.clone()) {
                    Some(data_left) => data_left as i32,
                    None => data as i32
                };
                avg_opp - data as i32
            },
            _ => 0
        };
        let early_quite_bonus: i32 = if early_quite {-20} else {0};
        let top_20_bonus = if top_20 {20} else {0};
        // Apply matchup pressure only for large MMR gaps.
        let mul = if mmr_diff.abs() < 250 {0.0} else { 
            2.0 / (1.0 + std::f64::consts::E.powf(-0.001 * (mmr_diff as f64))) - 1.0
        };
        let mmr_base = (score_mmr + early_quite_bonus + top_20_bonus) as f64;
        //dbg!((mmr_base, score_mmr, mul, mmr_diff));
        (if mmr_base + mul * 35.0 < 0.0 {0} else {(mmr_base + mul * 35.0) as i32}, MMRChangeDebug(score_mmr, mmr_diff, early_quite_bonus, top_20_bonus, mul, mmr_base))
    } else {
        // Defeat branch: base loss = score_curve − 66 + situational modifiers + matchup pressure.
        let score_mmr: i32 = (50.0 / (std::f64::consts::E.powf(1000000.0 / (score as f64).powf(2.0)))) as i32 - 66;
        let mmr_diff = match mmr {
            MMRType::MMR(data) => {
                let avg_opp: i32 = match avg_3(top_3.clone()) {
                    Some(data_left) => data_left as i32,
                    None => data as i32
                };
                data as i32 - avg_opp
            },
            _ => 0
        };
        let early_quite_bonus: i32 = if early_quite {-20} else {0};
        let top_20_bonus = if top_20 {20} else {0};
        // Apply matchup pressure only for large MMR gaps.
        let mul = if mmr_diff.abs() < 250 {0.0} else { 
            2.0 / (1.0 + std::f64::consts::E.powf(-0.001 * (mmr_diff as f64))) - 1.0
        };
        let mmr_base = (score_mmr + early_quite_bonus + top_20_bonus) as f64;
        //dbg!((mmr_base, score_mmr, mul, mmr_diff));
        ((mmr_base - mul * 35.0) as i32, MMRChangeDebug(score_mmr, mmr_diff, early_quite_bonus, top_20_bonus, mul, mmr_base))
    }
}