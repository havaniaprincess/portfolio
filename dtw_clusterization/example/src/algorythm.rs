//! Clusterization algorithm implementation
//! Contains functions for performing temporal clustering on time series data
//! and calculating statistics for each cluster including revenue metrics

use std::collections::HashMap;

use kmeans_tw::clusterization::temporal_clustering;
use kmeans_tw::context::ClusterizationContext;
use kmeans_tw::data_type::timewrap::{PaymentUser, TimeWrap};
use kmeans_tw::data_type::types::ClusterSet;
//use tokio::io::{AsyncWriteExt, BufWriter};

use crate::stats::ClusterStatistic;

/// Calculate statistics for each cluster
/// 
/// # Arguments
/// * `clusters` - HashMap of cluster IDs to ClusterSet containing time series data
/// * `payment_data` - HashMap of user IDs to PaymentUser data (revenue, sea beast status)
/// 
/// # Returns
/// * Tuple containing:
///   - Formatted string with cluster statistics
///   - Vector of ClusterStatistic objects
pub async fn get_stats_clusters(
    clusters: &HashMap<usize, ClusterSet<TimeWrap>>,
    payment_data: &HashMap<usize, PaymentUser>,
) -> (String, Vec<ClusterStatistic>) {
    // Calculate total number of active users across all clusters
    let au: f64 = clusters.iter().map(|obj| obj.1.points.len() as f64).sum::<f64>();
    
    // Calculate statistics for each cluster
    let stats: Vec<ClusterStatistic> = 
    clusters.iter().map(|(clust_id, cluster)| {
        // Calculate total revenue for the cluster
        let sum_revenue = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => payment.0,
                None => 0.0
            }
        }).sum::<f64>();
        
        // Count paying users (users with revenue > 0)
        let count_pu = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.0 > 0.0 {
                        1
                    } else {
                        0
                    }
                },
                None => 0
            }
        }).sum::<usize>();
        
        // Calculate total revenue from "sea beast" users only
        let sum_revenue_sea_beast = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.1 {
                        payment.0
                    } else {
                        0.0
                    }
                },
                None => 0.0
            }
        }).sum::<f64>();
        
        // Count "sea beast" users in the cluster
        let count_pu_sea_beast = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.1 {
                        1
                    } else {
                        0
                    }
                },
                None => 0
            }
        }).sum::<usize>();
        ClusterStatistic::new(*clust_id, cluster.points.len() as f64, count_pu as f64, sum_revenue, count_pu_sea_beast as f64, sum_revenue_sea_beast, au)
    }).collect();
    
    // Format statistics output string for each cluster
    (clusters.iter().map(|(clust_id, cluster)| {
        // Recalculate metrics for formatting (same logic as above)
        let sum_revenue = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => payment.0,
                None => 0.0
            }
        }).sum::<f64>();
        let count_pu = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.0 > 0.0 {
                        1
                    } else {
                        0
                    }
                },
                None => 0
            }
        }).sum::<usize>();
        // Calculate sea beast revenue for this cluster
        let sum_revenue_sea_beast = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.1 {
                        payment.0
                    } else {
                        0.0
                    }
                },
                None => 0.0
            }
        }).sum::<f64>();
        // Count sea beast users for this cluster
        let count_pu_sea_beast = cluster.points.iter().map(|point_id| {
            match payment_data.get(&point_id.0) {
                Some(payment) => {
                    if payment.1 {
                        1
                    } else {
                        0
                    }
                },
                None => 0
            }
        }).sum::<usize>();
        (*clust_id, cluster.points.len(), sum_revenue, count_pu, sum_revenue_sea_beast, count_pu_sea_beast)
    })
    .map(|(clust_id, cluster_point_len, sum_revenue, count_pu, sum_revenue_sea_beast, count_pu_sea_beast)| {
        // Format output string with metrics: cluster_id, active users, revenue, paying users, ARPU, ARPPU, etc.
        let out_str = format!("cluster_id: {:3} | au: {:5} | revenue: {:6.3} | pu: {}| arpu: {:.3} | arppu: {:.3} | pu_rate: {:.3} | seabeast_pu: {:5} | seabeast_revenue: {:6.3} | seabeast_pu_rate: {:.3} | seabeast_arpu: {:.3} | not_seabeast_arpu: {:.3}\n", 
            clust_id,
            cluster_point_len,
            sum_revenue,
            format!("{:5}", count_pu),
            sum_revenue / (cluster_point_len as f64),
            sum_revenue / (count_pu as f64),
            100.0 * (count_pu as f64) / (cluster_point_len as f64),
            count_pu_sea_beast,
            sum_revenue_sea_beast,
            100.0 * (count_pu_sea_beast as f64) / (count_pu as f64),
            sum_revenue_sea_beast / (cluster_point_len as f64),
            (sum_revenue - sum_revenue_sea_beast) / (cluster_point_len as f64),
        );
        out_str
    }).collect(), stats)
}

/// Perform time series clusterization
/// 
/// # Arguments
/// * `data` - HashMap of time series data (user_id -> time_bucket -> value)
/// * `payment_data` - HashMap of payment/revenue data for users
/// * `context` - Clusterization configuration parameters
/// * `_stat_path` - Path for statistics output (unused, kept for compatibility)
/// * `_assigned_path` - Path for assignments output (unused, kept for compatibility)
/// * `_project_dir` - Project directory path (unused, kept for compatibility)
/// 
/// # Returns
/// * Tuple containing:
///   - Good clusters (clusters meeting quality thresholds)
///   - Outline clusters (outlier/noisy clusters)
///   - User assignments (user_id -> cluster_id mapping)
///   - Combined cluster statistics for both good and outline clusters
pub async fn clusterization(
    data: &HashMap<usize, HashMap<usize, f64>>,
    payment_data: &HashMap<usize, PaymentUser>,
    context: ClusterizationContext,
    _stat_path: &String,
    _assigned_path: &String,
    _project_dir: &String,
) -> (HashMap<usize, ClusterSet<TimeWrap>>, HashMap<usize, ClusterSet<TimeWrap>>, HashMap<usize, usize>, Vec<ClusterStatistic>) {
    // Perform temporal clustering to separate good clusters from outliers
    let (good_clusters, outline_clusters) = temporal_clustering(data, context);

    // Calculate and print statistics for good clusters
    let (stats, mut cluster_statistic_good) = get_stats_clusters(&good_clusters, payment_data).await;

    println!("{}", &stats);

    // Sort good clusters by ARPU (Average Revenue Per User)
    cluster_statistic_good.sort_by_key(|obj| (obj.arpu*1000.0) as u64);
    
    // Calculate and print statistics for outline/outlier clusters
    let (stats, mut cluster_statistic_outline) = get_stats_clusters(&outline_clusters, payment_data).await;
    
    println!("{}", &stats);
    
    // Sort outline clusters by ARPU
    cluster_statistic_outline.sort_by_key(|obj| (obj.arpu*1000.0) as u64);

    // Build user-to-cluster assignment mapping
    let mut assigned: HashMap<usize, usize> = HashMap::new();

    // Assign users from good clusters
    for (cluster_id, cluster) in good_clusters.iter() {
        cluster.points.iter().for_each(|user_id| {
            assigned.insert(*user_id.0, *cluster_id);
        });
    };

    // Assign users from outline clusters
    outline_clusters.iter().for_each(|(cluster_id, cluster)| {
        cluster.points.iter().for_each(|user_id| {
            assigned.insert(*user_id.0, *cluster_id);
        });
    });

    // Return all clustering results and combined statistics
    (good_clusters, outline_clusters, assigned, [cluster_statistic_good, cluster_statistic_outline].concat())
}