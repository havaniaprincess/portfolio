//! Time series clusterization example using DTW (Dynamic Time Warping) algorithm
//! This program performs clustering on time series data with configurable parameters
//! and can output results to CSV files and BigQuery

use std::collections::HashMap;
use std::path::Path;

use clap::Parser;
use algorythm::clusterization;
use bq::BQPreContext;
use chrono::Utc;
use csv::{write_assigned, write_clusters_base};
use kmeans_tw::context::ClusterizationContext;
use kmeans_tw::data_type::dataset::DataCollection;
use loading::load_data;
use kmeans_tw::data_type::timewrap::TimeWrap;
use kmeans::types::DistanceMetric;
use tokio::fs;

// Module declarations
mod algorythm; 
mod stats;
mod csv;
mod bq;
mod loading;

/// Command-line arguments for the clusterization program
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    /// Path to the input JSON data file
    #[arg(long)]
    pub data: String,
    /// Name of the field to use as X-axis (time)
    #[arg(long)]
    pub x_axis: String,
    /// Name of the field to use as Y-axis (value)
    #[arg(long)]
    pub y_axis: String,
    /// Name of the field to use as unique identifier
    #[arg(long)]
    pub id_field: String,

    /// BigQuery project ID (optional, for output to BigQuery)
    #[arg(long)]
    pub bq_project: Option<String>,
    /// BigQuery dataset ID (optional, for output to BigQuery)
    #[arg(long)]
    pub bq_dataset: Option<String>,
    /// Path to BigQuery service account key file (optional)
    #[arg(long)]
    pub bq_keypath: Option<String>,
    /// Number of time dimensions/buckets (default: 24)
    #[arg(long)]
    pub dimention: Option<usize>,
    /// Output directory for results
    #[arg(long)]
    pub outdir: String,
    /// Distance metric: "DTW", "DtwWindowed", or "Euclidean" (default: DTW)
    #[arg(long)]
    pub distance: Option<String>,
    /// Window size for DTW windowed distance (default: 1)
    #[arg(long)]
    pub dtw_window: Option<usize>,
    /// Distance threshold between clusters (default: 0.14)
    #[arg(long)]
    pub cluster_distance: Option<f64>,
    /// Sigma threshold for identifying poor clusters (default: 0.18)
    #[arg(long)]
    pub bad_sigma: Option<f64>,
    /// Sigma threshold for identifying good clusters (default: 0.05)
    #[arg(long)]
    pub good_sigma: Option<f64>,
    /// Minimum cluster size (default: 50)
    #[arg(long)]
    pub min_cluster: Option<usize>,
    /// Minimum number of clusters (default: 3)
    #[arg(long)]
    pub nmin: Option<usize>,
    /// Maximum number of clusters (default: 3)
    #[arg(long)]
    pub nmax: Option<usize>,
    /// Maximum number of iterations (default: 25)
    #[arg(long)]
    pub max_iter: Option<usize>,
    /// Number of barycenter iterations (optional)
    #[arg(long)]
    pub barycenter_iter: Option<usize>,
    /// Random seed for reproducibility (default: 0)
    #[arg(long)]
    pub seed: Option<u64>,
}

#[tokio::main]
async fn main() {
    // Parse command-line arguments
    let args: Args = Args::parse();

    // Initialize BigQuery context if all required parameters are provided
    let bq_pre_context: Option<BQPreContext> = 
    if args.bq_project.is_some() && args.bq_dataset.is_some() && args.bq_keypath.is_some() {
        Some(BQPreContext { 
            project_id: args.bq_project.unwrap(), 
            dataset_id: args.bq_dataset.unwrap(), 
            key_path: args.bq_keypath.unwrap(),
        })
    } else {
        None
    };

    // Get current timestamp for versioning/tracking
    let time: i64 = Utc::now().timestamp_millis() / 1000;
    println!("{}", time);
    
    // Extract configuration parameters
    let path: String = args.data.to_string();
    let x_axis_name: String = args.x_axis.to_string();
    let y_axis_name: String = args.y_axis.to_string();
    let id_field: String = args.id_field.to_string();
    
    // Set dimension size (number of time buckets, default 24 for hourly data)
    let dim_size = match args.dimention {
        Some(w) => w,
        None => 24
    };
    
    // Prepare output directory paths
    let path_name = path.replace(".json", "");
    let out_dir = args.outdir.to_string(); 
    let dir_path = Path::new(&out_dir);

    // Create output directory if it doesn't exist
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).await.expect("Failed to create output directory");
        println!("Output directory created: {}", path_name);
    }
    
    // Create project-specific subdirectory
    let project_folder = out_dir.to_string() + "/" + &path_name;
    let dir_path = Path::new(&project_folder);
    if !dir_path.exists() {
        fs::create_dir_all(dir_path).await.expect("Failed to create project directory");
        println!("Project directory created: {}", project_folder);
    }

    // Load and preprocess time series data
    // Fill missing time buckets with 0.0 to ensure all series have consistent dimensions
    let data = load_data(&path, &x_axis_name, &y_axis_name, &id_field).await.unwrap()
        .iter().map(|(external_id, row)| {
            let mut result = row.clone();
            (0..dim_size).for_each(|h| {
                let time = row.0.get(&h);
                if time == None {
                    result.0.insert(h, 0.0);
                }
            });
            (*external_id, result)
        }).collect::<HashMap<usize, TimeWrap>>();
    
    // Create normalized data collection for clustering
    let normal_data = DataCollection::new(&data, false).right.iter().map(|(id, d)| (*id, d.0.clone())).collect();

    // Determine distance metric to use for clustering
    let distance_metric: Option<DistanceMetric> = match args.distance {
        Some(type_metric) => {
            if type_metric == "DTW".to_string() {
                Some(DistanceMetric::DTW)
            } else if type_metric == "DtwWindowed".to_string() {
                let window = match args.dtw_window {
                    Some(w) => w,
                    None => 1
                };
                Some(DistanceMetric::DtwWindowed(window))
            } else if type_metric == "Euclidean".to_string() {
                Some(DistanceMetric::Euclidean)
            } else {
                Some(DistanceMetric::DTW)
            }
        },
        None => Some(DistanceMetric::DTW)
    };

    // Configure clusterization parameters
    let clusterization_context: ClusterizationContext = ClusterizationContext { 
        distance_metric: distance_metric, 
        distance_threshold_between_clusters: match args.cluster_distance {
            Some(w) => w,
            None => 0.14
        }, 
        bad_sigma_threshold: match args.bad_sigma {
            Some(w) => w,
            None => 0.18
        }, 
        good_sigma_threshold: match args.good_sigma {
            Some(w) => w,
            None => 0.05
        }, 
        min_cluster_len: match args.min_cluster {
            Some(w) => w,
            None => 50
        }, 
        n_cluster_max: match args.nmin {
            Some(w) => w,
            None => 3
        }, 
        n_cluster_min: match args.nmax {
            Some(w) => w,
            None => 3
        }, 
        dim_size: dim_size, 
        max_iteration: match args.max_iter {
            Some(w) => w,
            None => 25
        }, 
        barycenter_iteration: match args.barycenter_iter {
            Some(w) => Some(w),
            None => None
        }, 
        seed: match args.seed {
            Some(w) => w,
            None => 0
        }
    };

    // Perform time series clusterization
    // Returns: good clusters, outline clusters, assignment mapping, and statistics
    let (
        good_clusters, 
        _outline_cluster, 
        assigned, 
        _cluster_statistic
    ) = clusterization(
        &normal_data, 
        &HashMap::new(), 
        clusterization_context, 
        &(project_folder.to_string() + "/" + "stats.txt"), 
        &(project_folder.to_string() + "/" + "assigned.json"), 
        &project_folder
    ).await;

    // Write cluster assignments to CSV and optionally to BigQuery
    write_assigned(&assigned, bq_pre_context.clone(), &project_folder,  &"assigned".to_string(), time).await;

    // Write cluster centroids to CSV and optionally to BigQuery
    write_clusters_base(&good_clusters, bq_pre_context.clone(), &project_folder, &"centroid".to_string(), time).await;

}

// Example command line usage:
//cargo.exe run --release -p dtw-clust-bin -- --data .\examples_data\data.json --outdir .\examples_data\outdir --distance DtwWindowed --dtw-window 3 --barycenter-iter 25 --x-axis hour --y-axis time --id-field id