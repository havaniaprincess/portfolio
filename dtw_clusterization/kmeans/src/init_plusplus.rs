//! K-Means++ initialization algorithm
//! Implements the K-Means++ algorithm for smart initial centroid selection
//! which improves clustering quality and convergence speed

use std::collections::{BTreeMap, HashMap};

use rand::prelude::*;
use rayon::prelude::*;

use crate::{time_series::TimeSeriesKmeans, types::{DistanceMetric, DtwDistance, EuclideanDistance, KmeansValue}};

/// Initialize cluster centroids using K-Means++ algorithm
/// 
/// K-Means++ selects initial centroids with probability proportional to their
/// squared distance from existing centroids, spreading them out across the data space.
/// This typically results in better clustering than random initialization.
/// 
/// # Arguments
/// * `data` - HashMap of data points (id -> time series)
/// * `model` - TimeSeriesKmeans model containing k (number of clusters) and RNG
/// * `distance_func` - Distance function to use (DTW, Euclidean, etc.)
/// 
/// # Returns
/// * `Some(HashMap)` containing k initial centroids, or `None` if data is empty
/// 
/// # Algorithm
/// 1. Choose first centroid randomly from data
/// 2. For each remaining centroid:
///    - Calculate distance from each point to nearest existing centroid
///    - Choose next centroid with probability proportional to squared distance
///    - This spreads centroids across the data space
pub fn set_centroid_by_data<T>(
    data: &HashMap<usize, T>,
    model: &TimeSeriesKmeans<T>,
    distance_func: fn(&DistanceMetric, &T, &T) -> f64
) -> Option<HashMap<usize, T>> 
where 
    T: Clone + KmeansValue + Send + Sync + EuclideanDistance + DtwDistance
{
    // HashMap to store selected centroids
    let mut res_centroids: HashMap<usize, T> = HashMap::new();
    
    // Get sorted list of data point IDs
    let mut vec_idx = data.keys().map(|key| *key).collect::<Vec<usize>>();
    vec_idx.sort();
    
    // Clone the random number generator from the model
    let mut rng = model.rng.clone();
    
    // Step 1: Select first centroid uniformly at random
    let first_index = if let Some(&first_index) = vec_idx.choose(&mut rng) {
        first_index
    } else {
        return None;  // Return None if data is empty
    };
    dbg!(first_index);
    
    // Add first randomly selected centroid
    res_centroids.insert(0, data[&first_index].clone());
    
    // Step 2: Select remaining k-1 centroids using K-Means++ algorithm
    for clust in 1..model.k {
        // Calculate minimum distance from each data point to nearest existing centroid
        // Using parallel iteration for performance
        let distances: HashMap<usize, f64> = data.par_iter()
            .map(|(data_id, row)| (*data_id, res_centroids.iter()
                .map(|(_cid, centroid)| distance_func(&model.metric, &row, &centroid))
                .min_by(|a, b| a.partial_cmp(b).unwrap())
                .unwrap())
        
        ).collect();

        // Calculate total distance for normalization
        let total_distance: f64 = distances.iter().map(|(_, d)| *d).sum();
        
        // Calculate probability for each point: proportional to its distance
        // Points farther from existing centroids have higher probability of being selected
        let probabilities: BTreeMap<usize, f64> = distances.iter().map(|(id, &d)| (*id, d / total_distance)).collect();
        
        let prob_total: f64 = probabilities.iter().map(|(_, d)| *d).sum();
        
        // Perform weighted random selection using cumulative probabilities
        let mut cumulative = 0.0;
        let rand_val: f64 = rng.random_range(0.0..prob_total);
        let mut selected_index = 0;
        
        // Find the data point corresponding to the random value
        for (i, &prob) in probabilities.iter() {
            cumulative += prob;
            // Debug line commented out
            //dbg!((rand_val, prob_total, cumulative));
            if rand_val < cumulative {
                selected_index = *i;
                break;
            }
        } 
        // Debug line commented out
        //dbg!((rand_val, prob_total, selected_index));
        
        // Add the selected data point as the next centroid
        res_centroids.insert(clust, data[&selected_index].clone());
    }
    
    // Return the k initialized centroids
    Some(res_centroids)
}