# Temporal Clustering (kmeans-tw)

A Rust library implementing advanced [temporal clustering](https://towardsdatascience.com/time-series-clustering-deriving-trends-and-archetypes-from-sequential-data-bb87783312b4/) with quality-based refinement for time series data.

The algorithm follows the approach described in "Part 1: Temporal Clustering" with additional quality assessment and recursive refinement capabilities.

## Overview

This library extends standard K-Means clustering with temporal awareness and quality-based optimization. It automatically determines the optimal number of clusters through iterative refinement, classifying clusters by quality and recursively improving poor clusters.

## Key Features

- **Quality-Based Classification**: Automatically classifies clusters into three categories:
  - **Good** (σ < 0.5): High-quality clusters with tight grouping
  - **Outline** (0.5 ≤ σ < 0.8): Acceptable clusters with moderate spread
  - **Reclusterization** (σ ≥ 0.8): Poor clusters requiring refinement

- **Recursive Refinement**: Automatically re-clusters poor quality clusters with adjusted parameters

- **Outlier Detection**: Uses 3-sigma rule to separate outliers from cluster cores

- **Duplicate Detection**: Identifies and merges similar clusters based on centroid distance

- **Multi-Level Clustering**: Tries multiple k values and selects the best configuration

- **Distance Metrics**: Supports DTW, DTW Windowed (Sakoe-Chiba), and Euclidean distance

## Architecture

The library consists of several key modules:

### Core Modules

- **clusterization.rs**: Main temporal clustering algorithm with quality refinement
  - `temporal_clustering()`: Recursive clustering with quality-based refinement
  - `clustering_run()`: Single-level clustering with quality separation
  - `clustering_module()`: Multi-k clustering with best configuration selection

- **algorythm.rs**: Cluster quality assessment and refinement utilities
  - `cluster_classificator()`: Classifies clusters by sigma thresholds
  - `cluster_dublicate_check()`: Detects and merges duplicate clusters
  - `clear_good_clusters()`: Separates outliers using 3-sigma rule

- **metrics.rs**: Cluster quality metrics
  - `metric_calculate()`: Calculates distance-based quality scores

- **context.rs**: Clustering context and configuration management

### Data Types

- **data_type/dataset.rs**: Dataset and time series structures
- **data_type/timewrap.rs**: Time series wrapper with temporal operations
- **data_type/traits.rs**: Common traits for clustering operations
- **data_type/types.rs**: Type definitions and enums

## Quality Classification

The algorithm uses standard deviation (sigma) to assess cluster quality:

```
σ = sqrt(Σ(deviation_i²) / n)
```

Where `deviation_i` is each point's distance from its cluster centroid.

**Classification Thresholds:**
- σ < 0.5: **Good** - Tight, homogeneous cluster
- 0.5 ≤ σ < 0.8: **Outline** - Acceptable spread
- σ ≥ 0.8: **Reclusterization** - Too scattered, needs refinement

## Usage Example

```rust
use kmeans_tw::clusterization::temporal_clustering;
use kmeans_tw::data_type::{TimeWrap, ClusteringContext};
use kmeans::DistanceMetric;

// Prepare your time series data
let data: Vec<TimeWrap> = load_time_series_data();

// Configure clustering context
let context = ClusteringContext {
    max_k: 10,              // Maximum number of clusters to try
    min_k: 2,               // Minimum number of clusters
    sigma_good: 0.5,        // Threshold for good clusters
    sigma_outline: 0.8,     // Threshold for outline clusters
    distance_metric: Some(DistanceMetric::DTW),
    duplicate_threshold: 0.1,  // Distance threshold for duplicate detection
};

// Run temporal clustering
let result = temporal_clustering(
    &data,
    &context,
    0  // Initial recursion level
);

// Result contains:
// - Good clusters: High quality, stable clusters
// - Outline clusters: Acceptable clusters with some spread
// - Recursively refined clusters: Previously poor clusters that were improved
```

## Algorithm Flow

1. **Initial Clustering**: Try k values from min_k to max_k
2. **Quality Assessment**: Calculate sigma for each cluster
3. **Classification**: Categorize clusters as Good/Outline/Reclusterization
4. **Duplicate Removal**: Merge clusters with similar centroids
5. **Outlier Separation**: Remove 3-sigma outliers from good clusters
6. **Recursive Refinement**: Re-cluster poor quality clusters with adjusted k
7. **Iteration**: Repeat until all clusters meet quality thresholds or max recursion depth reached

## Statistical Basis

### 3-Sigma Rule
Points with deviation > 3σ from the centroid are considered outliers. In a normal distribution, 99.7% of values fall within 3 standard deviations, making points beyond this threshold statistical outliers.

### Mahalanobis Distance Concept
The deviation metric used approximates the concept of Mahalanobis distance, measuring how many standard deviations a point is from the cluster center.

## Performance

The library uses parallel processing (via Rayon) for computationally intensive operations:
- Distance calculations
- Quality metric computation
- Outlier detection

## Dependencies

- `kmeans`: Base K-Means implementation with DTW support
- `rayon`: Parallel iteration for performance
- `serde`: Serialization/deserialization support

## References

- [Time Series Clustering: Deriving Trends and Archetypes from Sequential Data](https://towardsdatascience.com/time-series-clustering-deriving-trends-and-archetypes-from-sequential-data-bb87783312b4/)
- Sakoe-Chiba Band: Global constraint for DTW alignment
- 3-Sigma Rule: Statistical outlier detection method