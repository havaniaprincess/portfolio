# K-Means Clustering for Time Series

A Rust library implementing K-Means clustering algorithms optimized for time series data with support for multiple distance metrics including Dynamic Time Warping (DTW) and Euclidean distance.

## Features

- **Multiple Distance Metrics**
  - Standard DTW (Dynamic Time Warping)
  - DTW with Sakoe-Chiba band window constraint for faster computation
  - Euclidean distance

- **Smart Initialization**
  - K-Means++ algorithm for better initial centroid selection
  - Custom centroid initialization support

- **Advanced Centroid Calculation**
  - Euclidean mean for standard clustering
  - DTW Barycenter Averaging (DBA) for time series-aware centroids

- **Performance Optimizations**
  - Parallel processing using Rayon
  - Support for both dense (Vec) and sparse (HashMap) time series representations

## Usage

### Basic K-Means with Time Series

```rust
use kmeans::{TimeSeriesKmeans, DistanceMetric};
use std::collections::HashMap;

// Initialize the model
let cluster_count = 3;
let dimension_count = 24;  // e.g., 24 hours
let batch_size = 100;
let distance_metric = Some(DistanceMetric::DTW);
let centroid_override = None;
let seed = 0;
let barycenter_iteration = Some(10);

let mut model = TimeSeriesKmeans::new(
    cluster_count,
    dimension_count,
    batch_size,
    distance_metric,
    centroid_override,
    seed,
    barycenter_iteration
);

// Prepare your data (HashMap<id, Vec<f64>> or HashMap<id, HashMap<usize, f64>>)
let data: HashMap<usize, Vec<f64>> = // ... your time series data

// Fit the model
let max_iterations = 25;
fit(&data, &mut model, max_iterations);

// Access cluster assignments and centroids
let assignments = model.assigned;
let centroids = model.centroid;
```

### Distance Metrics

#### DTW (Dynamic Time Warping)
```rust
let metric = DistanceMetric::DTW;
```
Standard DTW finds optimal alignment between two time series. Best for data with temporal distortions.

#### DTW Windowed (Sakoe-Chiba Band)
```rust
let window_size = 3;
let metric = DistanceMetric::DtwWindowed(window_size);
```
Constrained DTW that limits the warping window for faster computation while maintaining good alignment quality.

#### Euclidean Distance
```rust
let metric = DistanceMetric::Euclidean;
```
Standard Euclidean distance assuming perfect temporal alignment. Fastest option.

### Centroid Calculation Methods

#### Euclidean Mean
Used automatically when distance metric is Euclidean. Computes the arithmetic mean of all points in each cluster.

#### DTW Barycenter Averaging (DBA)
Used when `barycenter_iteration` is specified with DTW metrics. Produces time series-aware centroids by:
1. Aligning all cluster members to the current centroid using DTW
2. Computing weighted averages based on alignment paths
3. Iteratively refining to minimize DTW distances

```rust
// Enable DBA with 10 refinement iterations
let barycenter_iteration = Some(10);
```

### K-Means++ Initialization

K-Means++ is automatically used for centroid initialization when no custom centroids are provided. This spreads initial centroids across the data space with probability proportional to distance from existing centroids, typically resulting in better clustering.

## Data Formats

The library supports two time series representations:

### Dense Time Series (Vec)
```rust
let data: HashMap<usize, Vec<f64>> = HashMap::from([
    (0, vec![1.0, 2.0, 3.0, 4.0]),
    (1, vec![2.0, 3.0, 4.0, 5.0]),
    // ...
]);
```
Use when all time points are present. More memory efficient for complete series.

### Sparse Time Series (HashMap)
```rust
let data: HashMap<usize, HashMap<usize, f64>> = HashMap::from([
    (0, HashMap::from([(0, 1.0), (5, 2.0), (10, 3.0)])),
    (1, HashMap::from([(0, 2.0), (7, 3.0), (15, 4.0)])),
    // ...
]);
```
Use when many time points are missing. Only stores present values.

## Module Structure

- `types.rs` - Core type definitions, traits, and distance metric implementations
- `tools.rs` - DTW distance calculation utilities
- `time_series.rs` - TimeSeriesKmeans model and fitting logic
- `barycenters.rs` - DTW Barycenter Averaging (DBA) implementation
- `euclidean_centers.rs` - Euclidean mean centroid calculation
- `init_plusplus.rs` - K-Means++ initialization algorithm

## Performance Considerations

- **DTW vs Euclidean**: DTW is more accurate for time series with temporal variations but significantly slower. Use windowed DTW for a good balance.
- **Window Size**: Smaller windows are faster but may miss important alignments. Typical values: 1-5.
- **Barycenter Iterations**: More iterations produce better centroids but increase computation time. Typical values: 5-25.
- **Parallel Processing**: The library automatically parallelizes distance calculations using Rayon for better performance on multi-core systems.

## Algorithm Complexity

- **Euclidean Distance**: O(n) per comparison
- **DTW**: O(n·m) per comparison where n and m are series lengths
- **DTW Windowed**: O(n·w) per comparison where w is the window size
- **K-Means**: O(k·n·d·i) where k=clusters, n=data points, d=distance complexity, i=iterations

