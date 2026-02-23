//! Euclidean centroid calculation for K-Means clustering
//! Implements centroid recalculation by computing the mean (average) of all points in each cluster

use std::{collections::{hash_map::Entry, HashMap}, fmt::Debug};

use crate::{time_series::TimeSeriesKmeans, types::KmeansValue};

/// Recalculate cluster centroids using Euclidean mean (arithmetic average)
/// 
/// This function updates the model's centroids by computing the mean of all data points
/// assigned to each cluster. This is the standard centroid update step in K-Means.
/// 
/// # Arguments
/// * `data` - HashMap of all data points (id -> time series)
/// * `model` - Mutable reference to TimeSeriesKmeans model (centroids will be updated)
/// * `assigned` - HashMap mapping data point IDs to their assigned cluster IDs
/// 
/// # Algorithm
/// 1. Sum all data points in each cluster
/// 2. Divide by the count of points in each cluster to get the mean
/// 3. Update model centroids with the computed means
/// 
/// # Note
/// This is used when the distance metric is Euclidean. For DTW distance,
/// a different barycenter calculation method is typically used.
pub fn euclidean_recalculate<T>(
    data: &HashMap<usize, T>,
    model: &mut TimeSeriesKmeans<T>,
    assigned: &HashMap<usize, usize>
) 
where 
    T: Clone + KmeansValue + Send + Sync + Debug
{
    // HashMap to accumulate sums of data points for each cluster   
    let mut new_centroids: HashMap<usize, T> = HashMap::new();
    // HashMap to count the number of points in each cluster
    let mut counts: HashMap<usize, usize> = HashMap::new();

    // Step 1: Sum all data points within each cluster
    for (i, row) in data.iter() {
        let cluster = assigned.get(i).unwrap();
        match new_centroids.entry(*cluster) {
            Entry::Occupied(mut e) => {
                // Cluster already has accumulated values, add current row
                *e.get_mut() = e.get().sum_by_field(row);
            },
            Entry::Vacant(e) => {
                // First point for this cluster, initialize with its value
                e.insert(row.clone());
            }
        }
        // Increment count for this cluster
        *counts.entry(*cluster).or_insert(0) += 1;
    }
    
    // Step 2: Divide each cluster's sum by its count to compute the mean
    new_centroids.iter_mut().for_each(|(i, centroid)| {
        if let Some(count) = counts.get(i) {
            if *count > 0 {
                // Compute mean by dividing accumulated sum by count
                *centroid = centroid.div_by_n(*count);
            }
        }
    });

    // Step 3: Update the model with the newly computed centroids
    model.centroid = new_centroids;
}