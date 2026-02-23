# DTW Clusterization Example

This example demonstrates how to perform time series clusterization using Dynamic Time Warping (DTW) distance metrics with K-Means clustering.

## Overview

The example application loads time series data from a JSON file, performs temporal clustering to group similar patterns, and outputs the results including:
- Cluster centroids
- User-to-cluster assignments
- Statistical metrics (ARPU, ARPPU, conversion rates, etc.)

Results can be written to CSV files and optionally uploaded to BigQuery.

## Usage

### Command Line Arguments

```bash
cargo run --release -p dtw-clust-bin -- \
  --data <path_to_json> \
  --outdir <output_directory> \
  --x-axis <time_field_name> \
  --y-axis <value_field_name> \
  --id-field <id_field_name> \
  [options]
```

### Required Arguments

- `--data` - Path to the input JSON data file
- `--outdir` - Output directory for results
- `--x-axis` - Name of the field to use as X-axis (time dimension)
- `--y-axis` - Name of the field to use as Y-axis (value)
- `--id-field` - Name of the field to use as unique identifier

### Optional Arguments

#### BigQuery Output
- `--bq-project` - BigQuery project ID (optional)
- `--bq-dataset` - BigQuery dataset ID (optional)
- `--bq-keypath` - Path to BigQuery service account key file (optional)

#### Clusterization Parameters
- `--dimention` - Number of time dimensions/buckets (default: 24)
- `--distance` - Distance metric: "DTW", "DtwWindowed", or "Euclidean" (default: DTW)
- `--dtw-window` - Window size for DTW windowed distance (default: 1)
- `--cluster-distance` - Distance threshold between clusters (default: 0.14)
- `--bad-sigma` - Sigma threshold for identifying poor clusters (default: 0.18)
- `--good-sigma` - Sigma threshold for identifying good clusters (default: 0.05)
- `--min-cluster` - Minimum cluster size (default: 50)
- `--nmin` - Minimum number of clusters (default: 3)
- `--nmax` - Maximum number of clusters (default: 3)
- `--max-iter` - Maximum number of iterations (default: 25)
- `--barycenter-iter` - Number of barycenter iterations (optional)
- `--seed` - Random seed for reproducibility (default: 0)

### Example Command

```bash
cargo run --release -p dtw-clust-bin -- \
  --data ./examples_data/data.json \
  --outdir ./examples_data/outdir \
  --distance DtwWindowed \
  --dtw-window 3 \
  --barycenter-iter 25 \
  --x-axis hour \
  --y-axis time \
  --id-field id
```

## Input Data Format

The input JSON file should contain an array of objects with time series data:

```json
[
  {
    "id": 1,
    "hour": 0,
    "time": 12.5
  },
  {
    "id": 1,
    "hour": 1,
    "time": 15.3
  },
  ...
]
```

## Output Files

The application generates the following outputs in the specified output directory:

- `assigned.csv` - User-to-cluster assignments
- `centroid.csv` - Cluster centroids (representative time series patterns)
- `stats.txt` - Clustering statistics and metrics

### Statistics Metrics

For each cluster, the following metrics are calculated:
- **AU** - Active Users (total users in cluster)
- **Revenue** - Total revenue from cluster
- **PU** - Paying Users count
- **ARPU** - Average Revenue Per User
- **ARPPU** - Average Revenue Per Paying User
- **PU Rate** - Conversion rate (percentage of paying users)
- **SeaBeast metrics** - Specific metrics for "sea beast" users (if applicable)

## Algorithm

The clusterization process consists of:

1. **Data Loading** - Load time series data from JSON and fill missing time buckets with zeros
2. **Temporal Clustering** - Apply K-Means with DTW distance metric to group similar patterns
3. **Quality Assessment** - Separate good clusters from outliers based on sigma thresholds
4. **Statistics Calculation** - Compute revenue and user metrics for each cluster
5. **Output Generation** - Write results to CSV and optionally to BigQuery

## Distance Metrics

### DTW (Dynamic Time Warping)
Standard DTW algorithm that finds optimal alignment between two time series.

### DTW Windowed
DTW with a constraint window that limits how far points can be matched. Faster than standard DTW.

### Euclidean
Simple Euclidean distance between time series points (assumes perfect alignment).

## Project Structure

- `main.rs` - Entry point, argument parsing, and orchestration
- `algorythm.rs` - Core clusterization logic and statistics calculation
- `loading.rs` - Data loading from JSON files
- `csv.rs` - CSV output generation
- `bq.rs` - BigQuery integration
- `stats.rs` - Statistics data structures
