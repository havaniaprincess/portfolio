# DTW Time Series Clustering

A Rust library for time series clustering using Dynamic Time Warping (DTW) distance metrics with advanced quality-based refinement.

## Overview

This project provides a comprehensive implementation of K-Means clustering optimized for time series data. It uses DTW (Dynamic Time Warping) as the distance metric to better handle temporal shifts and variations in time series patterns.

The project consists of three main components:

1. **kmeans** - Core K-Means implementation with DTW distance support
2. **kmeans-tw** - Temporal clustering with quality-based refinement
3. **example** - Command-line application for running clustering tasks

## Features

### Core Capabilities

- **Multiple Distance Metrics**:
  - DTW (Dynamic Time Warping) - Standard unbounded DTW
  - DTW Windowed - DTW with Sakoe-Chiba band constraint
  - Euclidean - Standard Euclidean distance

- **Advanced Initialization**:
  - K-Means++ algorithm for smart centroid initialization
  - Reduces convergence time and improves cluster quality

- **DTW Barycenter Averaging (DBA)**:
  - Calculates optimal centroids for DTW-based clusters
  - Iterative alignment and weighted averaging
  - More accurate than simple arithmetic means for time series

- **Quality-Based Refinement**:
  - Automatic cluster quality assessment using standard deviation (sigma)
  - Recursive refinement of poor-quality clusters
  - Outlier detection and removal using 3-sigma rule
  - Duplicate cluster detection and merging

- **Parallel Processing**:
  - Utilizes Rayon for parallel computation
  - Significant performance improvements on multi-core systems

## Project Structure

### kmeans/

Core K-Means implementation with DTW support.

**Key modules:**
- `tools.rs` - DTW distance calculation (cost matrices, path matrices)
- `types.rs` - Core types and traits (DistanceMetric, KmeansValue, DtwDistance)
- `init_plusplus.rs` - K-Means++ initialization algorithm
- `euclidean_centers.rs` - Euclidean centroid calculation
- `barycenters.rs` - DTW Barycenter Averaging implementation
- `time_series.rs` - Time series specific implementations
- `lib.rs` - Main library interface

See [kmeans/README.md](kmeans/README.md) for detailed documentation.

### kmeans-tw/

Temporal clustering with quality-based refinement.

**Key modules:**
- `clusterization.rs` - Main temporal clustering algorithm with recursive refinement
- `algorythm.rs` - Cluster quality assessment and classification
- `metrics.rs` - Cluster quality metrics calculation
- `context.rs` - Clustering context and configuration
- `data_type/` - Data structures for time series and clustering

**Features:**
- Quality classification: Good (σ < 0.5), Outline (0.5 ≤ σ < 0.8), Reclusterization (σ ≥ 0.8)
- Automatic k-value optimization (tries multiple k values)
- Recursive refinement of poor-quality clusters
- Outlier separation using statistical methods

See [kmeans-tw/README.md](kmeans-tw/README.md) for detailed documentation.

### example/

Command-line application for running clustering tasks.

**Key modules:**
- `main.rs` - CLI entry point with argument parsing
- `algorythm.rs` - Cluster statistics calculation (revenue metrics, ARPU, ARPPU)
- `loading.rs` - Data loading utilities
- `csv.rs` - CSV output functionality
- `bq.rs` - BigQuery integration
- `stats.rs` - Statistical calculations

**Capabilities:**
- Load data from JSON files
- Run temporal clustering with customizable parameters
- Calculate business metrics (revenue, active users, paying users)
- Export results to CSV or BigQuery

See [example/README.md](example/README.md) for usage instructions.

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
kmeans = { path = "kmeans" }
kmeans-tw = { path = "kmeans-tw" }
```

Or for the complete workspace:

```bash
cargo build --release
```

## Quick Start

### Using the Library

```rust
use kmeans_tw::clusterization::temporal_clustering;
use kmeans_tw::data_type::{TimeWrap, ClusteringContext};
use kmeans::DistanceMetric;

// Prepare time series data
let data: Vec<TimeWrap> = load_your_data();

// Configure clustering
let context = ClusteringContext {
    max_k: 10,
    min_k: 2,
    sigma_good: 0.5,
    sigma_outline: 0.8,
    distance_metric: Some(DistanceMetric::DTW),
    duplicate_threshold: 0.1,
};

// Run clustering
let result = temporal_clustering(&data, &context, 0);
```

### Using the CLI Application

```bash
# Basic clustering
cargo run --bin example -- \
    --file data.json \
    --distance dtw \
    --max-k 10 \
    --sigma-good 0.5 \
    --sigma-outline 0.8

# With CSV output
cargo run --bin example -- \
    --file data.json \
    --distance dtw \
    --max-k 10 \
    --csv-dir ./output
```

## Algorithm Details

### K-Means with DTW

Traditional K-Means uses Euclidean distance, which doesn't work well for time series with temporal shifts. This implementation uses DTW, which aligns sequences before measuring distance:

1. **Initialization**: K-Means++ selects initial centroids by maximizing minimum distance
2. **Assignment**: Each point assigned to nearest centroid using DTW distance
3. **Update**: Centroids recalculated using DBA (DTW Barycenter Averaging)
4. **Convergence**: Iterate until centroid movement falls below threshold

### Temporal Clustering with Quality Refinement

Enhanced algorithm that automatically optimizes cluster quality:

1. **Multi-k Clustering**: Try k ∈ [min_k, max_k], select best configuration
2. **Quality Assessment**: Calculate σ (standard deviation) for each cluster
3. **Classification**: 
   - Good: σ < 0.5 (tight, homogeneous)
   - Outline: 0.5 ≤ σ < 0.8 (acceptable spread)
   - Reclusterization: σ ≥ 0.8 (too scattered)
4. **Duplicate Removal**: Merge clusters with centroids closer than threshold
5. **Outlier Separation**: Remove points beyond 3σ from cluster center
6. **Recursive Refinement**: Re-cluster poor quality clusters with adjusted parameters
7. **Termination**: Stop when all clusters are Good/Outline or max recursion reached

### DTW Distance

Dynamic Time Warping finds optimal alignment between two sequences:

```
DTW(A, B) = min(Σ cost(alignment))
```

The algorithm uses dynamic programming to compute the optimal warping path, allowing for non-linear alignment of time series.

**Windowed DTW** (Sakoe-Chiba band) restricts alignment to a diagonal band, reducing computation time and preventing pathological alignments.

### DTW Barycenter Averaging (DBA)

Calculates the centroid of DTW-aligned time series:

1. Initialize with random series from cluster
2. For each iteration:
   - Align all series to current centroid using DTW
   - Calculate weighted average along alignment paths
   - Update centroid
3. Converge when centroid changes minimally

This produces more representative centroids than arithmetic means for time series data.

## Performance Considerations

- **DTW Complexity**: O(n²) for two series of length n
- **Windowed DTW**: O(n·w) where w is window size (typically w << n)
- **K-Means Iterations**: Typically 10-50 iterations until convergence
- **Parallel Processing**: Distance calculations parallelized with Rayon
- **Memory**: Cost matrices for DTW can be memory-intensive for long series

**Optimization Tips:**
- Use windowed DTW for long time series (reduces O(n²) to O(n·w))
- Set appropriate max_iterations to prevent excessive computation
- Use Euclidean distance for initial experiments (much faster than DTW)
- Downsample time series if temporal resolution isn't critical

## Use Cases

This library is suitable for:

- **User Behavior Clustering**: Group users by temporal activity patterns
- **Load Pattern Analysis**: Identify typical server/network load profiles
- **Financial Time Series**: Cluster stocks/assets by price movement patterns
- **Sensor Data Analysis**: Group sensors with similar temporal readings
- **Anomaly Detection**: Identify unusual patterns by cluster membership
- **Forecasting**: Build separate models for different behavior clusters

## Testing

Run tests for all components:

```bash
# All tests
cargo test

# Specific module
cargo test -p kmeans
cargo test -p kmeans-tw
cargo test -p example
```

## Documentation

Generate and view documentation:

```bash
cargo doc --open
```

## Dependencies

### Core Libraries
- `rayon` - Parallel iteration
- `serde` - Serialization/deserialization
- `serde_json` - JSON parsing

### Example Application
- `clap` - Command-line argument parsing
- `gcp-bigquery-client` - BigQuery integration (optional)

## Contributing

When contributing, please:

1. Run `cargo fmt` before committing
2. Ensure `cargo clippy` passes without warnings
3. Add tests for new functionality
4. Update relevant README files

## References

- [Time Series Clustering: Deriving Trends and Archetypes from Sequential Data](https://towardsdatascience.com/time-series-clustering-deriving-trends-and-archetypes-from-sequential-data-bb87783312b4/)
- Sakoe, H., & Chiba, S. (1978). Dynamic programming algorithm optimization for spoken word recognition. IEEE Transactions on Acoustics, Speech, and Signal Processing.
- Arthur, D., & Vassilvitskii, S. (2007). k-means++: The advantages of careful seeding. SODA '07.
- Petitjean, F., Ketterlin, A., & Gançarski, P. (2011). A global averaging method for dynamic time warping, with applications to clustering. Pattern Recognition.

## Acknowledgments

This implementation follows the temporal clustering methodology described in the Towards Data Science article and incorporates established algorithms like K-Means++, DTW, and DBA.
