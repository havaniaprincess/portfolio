//! Metrics calculation for cluster quality assessment
//! Provides functions to calculate distances and scores for evaluating cluster cohesion

use std::collections::HashMap;

use rayon::prelude::*;
use kmeans::types::{DistanceMetric, DtwDistance, EuclideanDistance};

/// Calculate cluster quality metrics by measuring distances from points to centroid
/// 
/// This function computes the distance from each point to a cluster centroid using
/// the specified distance metric (DTW, DTW Windowed, or Euclidean). The results
/// can be used to evaluate cluster cohesion and quality.
/// 
/// # Arguments
/// * `points` - Vector of data point IDs belonging to the cluster
/// * `clusters` - Cluster centroid represented as HashMap (time -> value)
/// * `data` - HashMap of all data points (id -> time series)
/// * `distance_metric` - Distance metric to use for calculations
/// 
/// # Returns
/// * Tuple containing:
///   - Average distance score for the cluster (lower is better/more cohesive)
///   - HashMap of individual distance scores for each point
/// 
/// # Performance
/// Uses parallel iteration for efficiency when calculating distances for many points
pub fn metric_calculate(
    points: &Vec<usize>,
    clusters: &HashMap<usize, f64>,
    data: &HashMap<usize, HashMap<usize, f64>>,
    distance_metric: Option<DistanceMetric>,
) -> (f64, HashMap<usize, f64>) {
    // Calculate distance from each point to the cluster centroid in parallel
    let point_scores: HashMap<usize, f64> = 
                points.par_iter()
                    .map(|data_id| {
                        let row = data.get(data_id).unwrap();
                        // Apply the configured distance metric
                        (*data_id, match distance_metric.clone().unwrap_or(DistanceMetric::Euclidean) {
                            DistanceMetric::DTW => clusters.dtw_path(&row).1,
                            DistanceMetric::DtwWindowed(window) => clusters.dtw_path_windowed( &row, window).1,
                            DistanceMetric::Euclidean => clusters.euclidean_distance( &row)
                        })
                    })
                    .collect();
    
    // Calculate average distance score as overall cluster quality metric
    // Lower score indicates better cluster cohesion (points closer to centroid)
    let cluster_score: f64 = point_scores.iter()
        .map(|(_data_id, score)| score).sum::<f64>() / (point_scores.len() as f64);
    
    (cluster_score, point_scores)
}