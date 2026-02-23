//! Temporal clustering implementation with quality-based refinement
//! 
//! This module implements a sophisticated clustering algorithm that:
//! 1. Performs K-Means clustering with multiple cluster counts
//! 2. Classifies clusters as Good, Outline, or requiring Reclusterization
//! 3. Recursively re-clusters poor quality clusters
//! 4. Merges outliers into a single outline cluster
//! 
//! The algorithm ensures high-quality clusters while handling noisy data gracefully.

use core::f64;
use std::collections::HashMap;
use kmeans::{time_series::{fit, TimeSeriesKmeans}, types::{DistanceMetric, DtwDistance, EuclideanDistance}};
use rayon::prelude::*;

use crate::{algorythm::{clear_good_clusters, cluster_classificator, cluster_dublicate_check}, context::ClusterizationContext, data_type::{timewrap::TimeWrap, types::{ClusterClass, ClusterSet}}, metrics::metric_calculate}; 

/// Core clustering module that finds optimal number of clusters
/// 
/// Performs K-Means clustering for different values of k (from n_cluster_min to n_cluster_max)
/// and selects the configuration with the best overall score.
/// 
/// # Arguments
/// * `data` - Time series data to cluster (id -> time series)
/// * `context` - Clustering configuration parameters
/// * `add_to_cluster` - Offset to add to cluster IDs (for hierarchical clustering)
/// 
/// # Returns
/// * Tuple containing:
///   - Best clusters found with their classifications
///   - Best overall clustering score
/// 
/// # Algorithm
/// 1. Try different numbers of clusters (n_cluster_min to n_cluster_max)
/// 2. For each k: run K-Means, classify clusters by quality
/// 3. Select k with lowest average distance score
fn clustering_module(
    data: &HashMap<usize, HashMap<usize, f64>>,
    context: ClusterizationContext,
    add_to_cluster: usize
) -> (HashMap<usize, ClusterSet<TimeWrap>>, f64) {
    println!("data: {}", data.len());
    
    // Try different cluster counts and select the best one
    let (best_clusters, best_score) = (context.n_cluster_min..=context.n_cluster_max).fold(
        (HashMap::new(), f64::INFINITY), 
        |(best_clusters, best_score), n| {
            // Initialize K-Means model with n clusters
            let mut model = TimeSeriesKmeans::new(n, context.dim_size, 10000, context.distance_metric.clone(), None, context.seed, context.barycenter_iteration);
            
            // Fit the model to the data
            let ((_fit_timer, _fit_timer_time), _times, assigned) = fit(&data, &mut model, context.max_iteration); 
            let clusters = model.centroid.clone();
            println!("N: {} | clusters: {} | data: {}", n, clusters.len(), data.len());
            
            // Build ClusterSet structures with quality classification
            let clusters: HashMap<usize, ClusterSet<TimeWrap>> = clusters.iter().map(|(cluster_id, centroid)| {
                // Get all points assigned to this cluster
                let points: Vec<usize> = assigned.iter().filter_map(|(data_id, cl_id)| if cluster_id == cl_id {Some(*data_id)} else {None} ).collect();
                
                // Calculate quality metrics for this cluster
                let (score, point_scores) = metric_calculate(&points, centroid, data, context.distance_metric.clone());
                //dbg!(format!("{:?}", &point_scores));
                
                // Classify cluster as Good, Outline, or Reclusterization based on quality
                let (cluster_class, points) = cluster_classificator(score, &point_scores, context.bad_sigma_threshold, context.good_sigma_threshold, context.min_cluster_len);
                (*cluster_id , ClusterSet{
                    id: *cluster_id,
                    centroid: TimeWrap(centroid.clone()),
                    points: points,
                    class: cluster_class
                })
            }).collect();
            
            // Calculate overall clustering score (average distance to centroids)
            let score = assigned.par_iter()
                .map(|(data_id, cluster_id)| {
                    let centroid = &clusters.get(cluster_id).unwrap().centroid.0;
                    let row = data.get(data_id).unwrap();
                    let metric = match model.metric {
                        DistanceMetric::DTW => centroid.dtw_path(&row).1,
                        DistanceMetric::DtwWindowed(window) => centroid.dtw_path_windowed( &row, window).1,
                        DistanceMetric::Euclidean => centroid.euclidean_distance( &row)
                    };
                    metric
                })
                .sum::<f64>() / (data.len() as f64);
            
            // Print cluster quality information
            clusters.iter().for_each(|(cluster_id, clusetr)| {
                println!("Cluster id: {} | score: {:.4}", cluster_id, match clusetr.class {
                    ClusterClass::Good(score, _) => score,
                    ClusterClass::Outline(score, _) => score,
                    ClusterClass::Reclusterization(score, _) => score,
                    _ => f64::INFINITY
                });
            });
            
            // Adjust cluster IDs by offset for hierarchical clustering
            let mut clusters = clusters;
            let clusters = clusters.iter_mut().map(|(cid, cd)| {
                cd.id += add_to_cluster;
                (*cid + add_to_cluster, cd.clone())
            }).collect();
            
            println!("N: {} | score: {:.4}", n, score);
            
            // Keep this configuration if it has the best score so far
            if score < best_score {
                return (clusters, score);
            }
            (best_clusters, best_score)
        }
    );

    (best_clusters, best_score)
}

/// Execute clustering and separate clusters by quality classification
/// 
/// Runs clustering_module and then separates the resulting clusters into three categories:
/// - Good: High quality clusters
/// - Outline: Outlier/noise points
/// - Reclusterization: Poor quality clusters that need to be re-clustered
/// 
/// # Arguments
/// * `data` - Time series data to cluster
/// * `context` - Clustering configuration parameters  
/// * `add_to_cluster` - Offset to add to cluster IDs
/// 
/// # Returns
/// * Tuple containing three HashMaps:
///   - Good clusters (high quality)
///   - Outline clusters (outliers/noise)
///   - Reclusterization clusters (require further clustering)
fn clustering_run(
    data: &HashMap<usize, HashMap<usize, f64>>,
    context: ClusterizationContext,
    add_to_cluster: usize,
) -> (HashMap<usize, ClusterSet<TimeWrap>>,HashMap<usize, ClusterSet<TimeWrap>>,HashMap<usize, ClusterSet<TimeWrap>>) {
    
    // Perform clustering to find best configuration
    let (best_clusters, _best_score) = clustering_module(data, context.clone(), add_to_cluster);

    // Check for and merge duplicate/similar clusters
    let best_clusters = cluster_dublicate_check(&best_clusters, context.distance_threshold_between_clusters, context.distance_metric.clone());

    // Separate clusters by classification and clean good clusters from outliers
    let (good_clusters, outline_clusters, reclusterization_clusters) = best_clusters.iter()
        .map(|(clust_id, cluster)| {
            match cluster.class {
                ClusterClass::Good(score, sigma) => {
                    println!("[G]cluster: {} | score: {:.4} | sigma: {:.4} | rate: {:.4}", clust_id, score, sigma, sigma / score);
                    // Separate good cluster core from outlier points
                    let (new_cluster, mut outline_cluster) = clear_good_clusters(cluster, sigma);
                    outline_cluster.class = outline_cluster.class.make_outline();
                    (Some(new_cluster), Some(outline_cluster), None)
                },
                ClusterClass::Outline(score, sigma) => {
                    println!("[O]cluster: {} | score: {:.4} | sigma: {:.4} | rate: {:.4}", clust_id, score, sigma, sigma / score);
                    // Keep as outline cluster
                    (None, Some(cluster.clone()), None)
                },
                ClusterClass::Reclusterization(score, sigma) => {
                    println!("[R]cluster: {} | score: {:.4} | sigma: {:.4} | rate: {:.4}", clust_id, score, sigma, sigma / score);
                    // Mark for reclusterization
                    (None, None, Some(cluster.clone()))
                },
                _ => {(None, None, None)}
            }
        })
        .fold((vec![], vec![], vec![]), |(good, outline, reclust), (right_good, right_out, right_reclust)| {
            ([good, vec![right_good]].concat(), [outline, vec![right_out]].concat(), [reclust, vec![right_reclust]].concat())
        });
    
    // Convert vectors to HashMaps, filtering out None values
    let outlined_clusters: HashMap<usize, ClusterSet<TimeWrap>> = outline_clusters.into_iter().filter_map(|obj| {
        match obj {
            Some(c) => Some((c.id, c)),
            None => None
        }
    }).collect();
    let good_clusters: HashMap<usize, ClusterSet<TimeWrap>> = good_clusters.into_iter().filter_map(|obj| {
        match obj {
            Some(c) => Some((c.id, c)),
            None => None
        }
    }).collect();
    let reclusterization_clusters: HashMap<usize, ClusterSet<TimeWrap>> = reclusterization_clusters.into_iter().filter_map(|obj| {
        match obj {
            Some(c) => Some((c.id, c)),
            None => None
        }
    }).collect();
    (good_clusters, outlined_clusters, reclusterization_clusters)
}

/// Main entry point for temporal clustering with recursive refinement
/// 
/// Performs multi-level clustering:
/// 1. Initial clustering of all data
/// 2. Recursive re-clustering of poor quality clusters
/// 3. Merging all outliers into a single outline cluster
/// 4. Final duplicate cluster removal
/// 
/// # Arguments
/// * `data` - Time series data to cluster (id -> time series HashMap)
/// * `context` - Clustering configuration parameters
/// 
/// # Returns
/// * Tuple containing:
///   - Good clusters: HashMap of high-quality clusters
///   - Outline clusters: HashMap containing merged outlier/noise points
/// 
/// # Algorithm Flow
/// 1. Run initial clustering to classify data into Good/Outline/Reclusterization
/// 2. For each cluster marked for reclusterization:
///    - Extract its points and re-cluster them recursively  
///    - Classify results into Good/Outline/Reclusterization again
///    - Clean outliers from good clusters
/// 3. Merge all outline clusters into one
/// 4. Final duplicate cluster check on good clusters
pub fn temporal_clustering(
    data: &HashMap<usize, HashMap<usize, f64>>,
    context: ClusterizationContext,
) -> (HashMap<usize, ClusterSet<TimeWrap>>, HashMap<usize, ClusterSet<TimeWrap>>) {

    // Initial clustering run to classify all data
    let (mut good_clusters, mut outline_clusters, reclusterization_clusters) = clustering_run(data, context.clone(), 0);

    // Recursively re-cluster poor quality clusters
    reclusterization_clusters.iter()
        .for_each(|(cluster_id, cluster_set)| {
            // Extract data points that belong to this cluster marked for reclusterization
            let data_cluster: HashMap<usize, HashMap<usize, f64>> = cluster_set.points.iter().filter_map(|(point_id, _)| {
                let d = data.get(point_id);
                if d == None {
                    return None;
                }
                Some((*point_id, d.unwrap().clone()))
            }).collect();

            // Re-cluster this subset of data with adjusted cluster ID offset
            let (good_clusters_left, outline_clusters_left, reclusterization_clusters) = clustering_run(&data_cluster, context.clone(), (cluster_id+1)*100);

            // Merge outline clusters from recursive call into main outline clusters
            outline_clusters_left.into_iter().map(|obj| obj.1)
                .for_each(|obj| {
                    outline_clusters.insert(obj.id, obj);
            });
            
            // Merge good clusters from recursive call into main good clusters
            good_clusters_left.into_iter().map(|obj| obj.1)
                .for_each(|obj| {
                    good_clusters.insert(obj.id, obj);
            });
            
            // Process any remaining reclusterization clusters from recursive call
            // These are split into good core and outliers
            reclusterization_clusters.into_iter().map(|obj| obj.1)
                .for_each(|obj| {
                    let (mut new_cluster, mut outline_cluster) = clear_good_clusters(&obj, match obj.class {
                        ClusterClass::Reclusterization(_, sigma) => sigma,
                        _ => panic!("In reclusterization clusters not found ClusterClass::Reclusterization")
                    });
                    new_cluster.class = new_cluster.class.make_good();
                    outline_cluster.class = outline_cluster.class.make_outline();
                    good_clusters.insert(new_cluster.id, new_cluster);
                    outline_clusters.insert(outline_cluster.id, outline_cluster);
            });
        });

    // Merge all outline clusters into a single combined outline cluster
    let outline_clusters = match outline_clusters.into_iter().reduce(|mut acc, cluster| {
        acc.1.points.extend(cluster.1.points.iter());
        acc
    }) {
        Some((id, outline)) => {
            // Create HashMap with single merged outline cluster
            let mut result: HashMap<usize, ClusterSet<TimeWrap>> = HashMap::new();
            result.insert(id, outline.clone());
            result
        },
        None => {
            HashMap::new()
        }
    };
    
    // Final check for duplicate/similar clusters among good clusters
    let good_clusters = cluster_dublicate_check(&good_clusters, context.distance_threshold_between_clusters, context.distance_metric.clone());
    
    (good_clusters, outline_clusters)
}


#[cfg(test)]
mod tests {
    //use std::collections::{BTreeMap, HashMap};

    //use super::*;

    //use std::collections::HashMap;

    //use kmeans::{time_series::{fit, TimeSeriesKmeans}, types::DistanceMetric};

    //use crate::{data_type::{dataset::DataCollection, timewrap::TimeWrap}};

}