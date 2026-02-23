//! Cluster quality assessment and refinement algorithms
//! 
//! Provides functions for:
//! - Classifying clusters by quality (Good, Outline, Reclusterization)
//! - Detecting and merging duplicate/similar clusters
//! - Separating outliers from high-quality cluster cores

use std::collections::HashMap;
use kmeans::types::{DistanceMetric, DtwDistance, EuclideanDistance};
use rayon::prelude::*;

use crate::{data_type::timewrap::TimeWrap, data_type::types::{ClusterClass, ClusterSet}};

/// Classify a cluster based on its quality metrics
/// 
/// Analyzes cluster cohesion using standard deviation (sigma) of distances from points
/// to the centroid. Clusters are classified into three categories:
/// - Good: Low sigma, well-formed cohesive cluster
/// - Outline: High sigma or too few points, likely noise/outliers
/// - Reclusterization: Medium sigma, may benefit from re-clustering
/// 
/// # Arguments
/// * `score` - Average distance of points to cluster centroid
/// * `points_score` - HashMap of individual point distances (point_id -> distance)
/// * `bad_sigma_threshold` - Sigma above this value marks cluster as Outline
/// * `good_sigma_threshold` - Sigma below this value marks cluster as Good
/// * `min_cluster_len` - Minimum number of points for a valid cluster
/// 
/// # Returns
/// * Tuple containing:
///   - ClusterClass: Quality classification (Good/Outline/Reclusterization)
///   - HashMap of points with their distances and deviations
/// 
/// # Classification Logic
/// - Outline: sigma > bad_threshold OR size < min_size
/// - Good: sigma < good_threshold
/// - Reclusterization: good_threshold <= sigma <= bad_threshold
pub fn cluster_classificator( 
    score: f64,
    points_score: &HashMap<usize, f64>,
    bad_sigma_threshold: f64,
    good_sigma_threshold: f64,
    min_cluster_len: usize,
) -> (ClusterClass, HashMap<usize, (f64, f64)>) {
    
    // Calculate deviation of each point from the cluster's average distance
    let points_dev: HashMap<usize, (f64, f64)> = points_score.iter().map(|(point_id, dist)| (*point_id, (*dist, (dist - score).abs()))).collect();
    
    // Calculate standard deviation (sigma) of point distances
    // Lower sigma indicates more cohesive cluster
    let sigma: f64 = (points_dev.iter().map(|(_, (_, dev))| dev.powi(2)).sum::<f64>() / (points_dev.len() as f64)).sqrt();

    // Classify cluster based on sigma and size
    (if sigma > bad_sigma_threshold || points_score.len() < min_cluster_len {
        // Poor quality: high variation or too few points -> Outline
        ClusterClass::Outline(score, sigma)
    } else if sigma < good_sigma_threshold  {
        // High quality: low variation -> Good cluster
        ClusterClass::Good(score, sigma)
    } else {
        // Medium quality: may improve with re-clustering
        ClusterClass::Reclusterization(score, sigma)
    }, points_dev)
}

/// Detect and merge duplicate or very similar clusters
/// 
/// Identifies clusters that are too similar (based on centroid distance) and merges
/// them to avoid redundancy. This prevents over-segmentation of the data.
/// 
/// # Arguments
/// * `clusters` - HashMap of all clusters to check
/// * `distance_threshold` - Maximum distance between centroids to consider clusters as duplicates
/// * `distance_metric` - Distance metric to use for centroid comparison
/// 
/// # Returns
/// * HashMap of clusters with duplicates removed and merged
/// 
/// # Algorithm
/// 1. For each cluster, find its nearest neighbor cluster
/// 2. If distance < threshold, mark as duplicate and remove
/// 3. Merge points from removed clusters into their nearest neighbors
/// 4. Return deduplicated cluster set
pub fn cluster_dublicate_check(
    clusters: &HashMap<usize, ClusterSet<TimeWrap>>, 
    distance_threshold: f64,
    distance_metric: Option<DistanceMetric>
) -> HashMap<usize, ClusterSet<TimeWrap>> {

    // Track clusters marked for removal and their destination
    let mut removed: HashMap<usize, (HashMap<usize, (f64, f64)>, usize)> = HashMap::new();

    // Find duplicates and filter them out
    let mut result: HashMap<usize, ClusterSet<TimeWrap>> = clusters.iter().filter_map(|(i1, c1)| {
        // Find the nearest cluster to this one
        let (clust_id, min_distance) = clusters.iter().map(|(i2, c2)| {
            println!("Dub {} & {}", i1, i2);
            
            // Skip if already removed or comparing to self
            if removed.contains_key(i2) || *i2 == *i1 {
                return (*i2, f64::INFINITY);
            }
            
            // Calculate distance between cluster centroids
            let result = match distance_metric.clone().unwrap_or(DistanceMetric::Euclidean) {
                DistanceMetric::DTW => c1.centroid.0.dtw_path(&c2.centroid.0).1,
                DistanceMetric::DtwWindowed(window) => c1.centroid.0.dtw_path_windowed(&c2.centroid.0, window).1,
                DistanceMetric::Euclidean => c1.centroid.0.euclidean_distance(&c2.centroid.0)
                
            };
            println!("Dub {} & {}: {:.4}", i1, i2, result);
            //dbg!(result);
            (*i2, result)
        })
        .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal))
        .unwrap();
        
        // If nearest cluster is too close, mark this cluster as duplicate
        if min_distance < distance_threshold {
            removed.insert(*i1, (c1.points.clone(), clust_id));
            None  // Remove this cluster
        } else {
            Some((*i1, c1.clone()))  // Keep this cluster
        }
    }).collect();
    
    // Merge points from removed clusters into their destination clusters
    removed.iter()
        .for_each(|(_rem_id, (points, dest_id))| {
            match result.get_mut(dest_id) {
                Some(clust) => {
                    clust.points.extend(points.iter());
                },
                None => {}
            };
        });
    result
}

/// Separate outlier points from a good cluster core
/// 
/// Uses the 3-sigma rule to identify and remove outlier points from a cluster.
/// Points with deviation > 3*sigma are considered outliers and moved to a
/// separate outline cluster.
/// 
/// # Arguments
/// * `cluster` - The cluster to clean
/// * `sigma` - Standard deviation of point distances in the cluster
/// 
/// # Returns
/// * Tuple containing:
///   - New cluster with only core points (deviation <= 3*sigma)
///   - Outline cluster containing outlier points (deviation > 3*sigma)
/// 
/// # Statistical Basis
/// In a normal distribution, ~99.7% of values fall within 3 standard deviations.
/// Points beyond 3*sigma are statistical outliers.
pub fn clear_good_clusters(
    cluster: &ClusterSet<TimeWrap>,
    sigma:f64,
) -> (ClusterSet<TimeWrap>, ClusterSet<TimeWrap>) {
    // Identify outlier points using 3-sigma rule (parallel processing)
    let outline_points: HashMap<usize, (f64, f64)> = cluster.points.par_iter()
        .filter_map(|(point_id, (dist, dev))| {
            if *dev > 3.0 * sigma {
                return Some((*point_id, (*dist, *dev)));
            }
            None
        })
        .collect();
    
    // Keep only core points within 3 sigma
    let good_points: HashMap<usize, (f64, f64)> = cluster.points.par_iter()
        .filter_map(|(point_id, (dist, dev))| {
            if *dev <= 3.0 * sigma {
                return Some((*point_id, (*dist, *dev)));
            }
            None
        })
        .collect();
    
    // Create new cluster with only core points
    let mut new_cluster = cluster.clone();
    new_cluster.points = good_points;
    
    // Create outline cluster with outlier points (special ID: 666*100 + original_id)
    let mut outline_cluster = cluster.clone();
    outline_cluster.id = 666*100 + outline_cluster.id;
    outline_cluster.points = outline_points;
    
    (new_cluster.clone(), outline_cluster.clone())

}