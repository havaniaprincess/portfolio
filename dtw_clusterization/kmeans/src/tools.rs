//! DTW (Dynamic Time Warping) helper functions
//! Contains utilities for calculating DTW distance and alignment paths between time series

use std::{collections::HashMap, iter::Sum, ops::{Mul, Sub}};

/// Calculate DTW cost matrix and path matrix for HashMap-based time series
/// 
/// This function computes the Dynamic Time Warping distance between two time series
/// represented as HashMaps, where keys are time indices and values are measurements.
/// 
/// # Arguments
/// * `left` - First time series as HashMap (time_index -> value)
/// * `right` - Second time series as HashMap (time_index -> value)
/// * `window` - Optional Sakoe-Chiba band constraint (limits warping window)
/// 
/// # Returns
/// * Tuple containing:
///   - Cost matrix: cumulative DTW costs at each point
///   - Path matrix: backtracking pointers for optimal alignment path
/// 
/// # Type Parameters
/// * `D` - Data type that supports subtraction, multiplication, conversion to f64, and other numeric operations
pub fn get_path_hashmap<D>(
    left: &HashMap<usize, D>,
    right: &HashMap<usize, D>,
    window: Option<usize>,
) -> (Vec<Vec<f64>>, Vec<Vec<Option<(usize, usize)>>>)  
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy + Sum + From<u32>
{
    let len1 = left.len();
    let len2 = right.len();

    // Initialize cost matrix with infinity (unvisited cells)
    let mut cost_matrix = vec![vec![f64::INFINITY; len2]; len1];
    // Initialize path matrix to track optimal alignment path
    let mut path_matrix = vec![vec![None; len2]; len1];

    // Iterate through all cells in the cost matrix
    for i in 0..len1 {
        // Apply window constraint if specified (Sakoe-Chiba band)
        for j in if window.is_none() { 0..len2 } else { ((0.max(i as isize - window.unwrap() as isize)) as usize)..(len2.min(i + window.unwrap())) }  {
            // Get values from left and right series (use 0.0 for missing time points)
            let l_d: f64 = match left.get(&i).copied() {
                Some(d) => d.into(),
                None => 0.0
            };
            let r_d: f64 = match right.get(&j).copied() {
                Some(d) => d.into(),
                None => 0.0
            };
            // Calculate local cost (squared Euclidean distance)
            let cost = (l_d - r_d).powi(2);
            let mut prev = None;
            // Calculate cumulative cost by adding minimum of three possible paths
            cost_matrix[i][j] = cost
                + if i > 0 && j > 0 {
                    // Three possible predecessors: diagonal, left, or top
                    if cost_matrix[i - 1][j] < cost_matrix[i][j - 1] && cost_matrix[i - 1][j] < cost_matrix[i - 1][j - 1] {
                        prev = Some((i - 1, j));
                    } else if cost_matrix[i][j - 1] < cost_matrix[i - 1][j] && cost_matrix[i][j - 1] < cost_matrix[i - 1][j - 1] {
                        prev = Some((i, j - 1));
                    } else {
                        prev = Some((i - 1, j - 1));
                    }
                    cost_matrix[i - 1][j]
                        .min(cost_matrix[i][j - 1])
                        .min(cost_matrix[i - 1][j - 1])
                } else if i > 0 {
                    prev = Some((i - 1, j));
                    cost_matrix[i - 1][j]
                } else if j > 0 {
                    prev = Some((i, j - 1));
                    cost_matrix[i][j - 1]
                } else {
                    0.0
                };
            path_matrix[i][j] = prev;
        }
    }

    (cost_matrix, path_matrix)
}

/// Calculate DTW cost matrix and path matrix for Vec-based time series
/// 
/// Similar to `get_path_hashmap` but works with Vec instead of HashMap.
/// This is more efficient for dense time series where all time points are present.
/// 
/// # Arguments
/// * `left` - First time series as Vector
/// * `right` - Second time series as Vector
/// * `window` - Optional Sakoe-Chiba band constraint (limits warping window)
/// 
/// # Returns
/// * `Some(tuple)` containing cost matrix and path matrix, or `None` if either input is empty
/// 
/// # Type Parameters
/// * `D` - Data type that supports subtraction, multiplication, conversion to f64, and other numeric operations
pub fn get_path_vec<D>(
    left: &Vec<D>,
    right: &Vec<D>,
    window: Option<usize>,
) -> Option<(Vec<Vec<f64>>, Vec<Vec<Option<(usize, usize)>>>)>  
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy + Sum + From<u32>
{
    let len1 = left.len();
    let len2 = right.len();
    let mut cost_matrix = vec![vec![f64::INFINITY; len2]; len1];
    let mut path_matrix = vec![vec![None; len2]; len1];

    // Return None if either time series is empty
    if len1 == 0 || len2 == 0 {
        return None;
    }

    // Iterate through all cells in the cost matrix
    for i in 0..len1 {
        // Apply window constraint if specified (Sakoe-Chiba band)
        for j in if window.is_none() { 0..len2 } else { ((0.max(i as isize - window.unwrap() as isize)) as usize)..(len2.min(i + window.unwrap())) } {
            // Calculate local cost (squared Euclidean distance)
            let cost = ((left[i] - right[j]) * (left[i] - right[j])).into();
            let mut prev = None;
            // Calculate cumulative cost by adding minimum of three possible paths
            cost_matrix[i][j] = cost
                + if i > 0 && j > 0 {
                    // Choose the predecessor with minimum cost
                    if cost_matrix[i - 1][j] < cost_matrix[i][j - 1] && cost_matrix[i - 1][j] < cost_matrix[i - 1][j - 1] {
                        prev = Some((i - 1, j));  // From top
                    } else if cost_matrix[i][j - 1] < cost_matrix[i - 1][j] && cost_matrix[i][j - 1] < cost_matrix[i - 1][j - 1] {
                        prev = Some((i, j - 1));  // From left
                    } else {
                        prev = Some((i - 1, j - 1));  // From diagonal
                    }
                    cost_matrix[i - 1][j]
                        .min(cost_matrix[i][j - 1])
                        .min(cost_matrix[i - 1][j - 1])
                } else if i > 0 {
                    // Only top predecessor available (first column)
                    prev = Some((i - 1, j));
                    cost_matrix[i - 1][j]
                } else if j > 0 {
                    // Only left predecessor available (first row)
                    prev = Some((i, j - 1));
                    cost_matrix[i][j - 1]
                } else {
                    // Origin cell (0, 0)
                    0.0
                };
            path_matrix[i][j] = prev;
        }
    }

    Some((cost_matrix, path_matrix))
}