use kmeans::types::DistanceMetric;

// Struct to hold the context for clusterization, including parameters and settings
#[derive(Clone)]
pub struct ClusterizationContext{
    pub distance_metric: Option<DistanceMetric>,
    pub distance_threshold_between_clusters: f64,
    pub bad_sigma_threshold: f64,
    pub good_sigma_threshold: f64,
    pub min_cluster_len: usize,
    pub n_cluster_max: usize,
    pub n_cluster_min: usize,
    pub dim_size: usize,
    pub max_iteration: usize,
    pub barycenter_iteration: Option<usize>,
    pub seed: u64,
}