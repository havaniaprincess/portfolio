//! DTW Barycenter Averaging (DBA) for K-Means clustering
//! Implements barycenter calculation for time series clustering using DTW alignment.
//! A barycenter is the average time series that minimizes the sum of DTW distances
//! to all members of a cluster, computed through iterative refinement.

use std::{collections::HashMap, fmt::Debug};

use rayon::iter::{IntoParallelIterator, IntoParallelRefIterator, ParallelIterator};

use crate::{time_series::TimeSeriesKmeans, types::{DistanceMetric, DtwDistance, EuclideanDistance, KmeansValue}};

/// Recalculate cluster centroids using DTW Barycenter Averaging (DBA)
/// 
/// DBA computes the "average" time series for each cluster by iteratively:
/// 1. Aligning all cluster members to the current centroid using DTW
/// 2. Computing weighted averages based on the DTW alignment paths
/// 3. Updating the centroid with the new average
/// 
/// This method is superior to Euclidean averaging for time series because it
/// accounts for temporal distortions and finds the best representative sequence.
/// 
/// # Arguments
/// * `data` - HashMap of all data points (id -> time series)
/// * `model` - Mutable reference to TimeSeriesKmeans model (centroids will be updated)
/// * `assigned` - HashMap mapping data point IDs to their assigned cluster IDs
/// 
/// # Algorithm
/// For each iteration:
/// 1. Align each time series to its cluster's centroid using DTW
/// 2. Compute warping sums and valence (alignment counts) for each time point
/// 3. Calculate new centroid as warping_sum / valence for each position
/// 4. Repeat for specified number of iterations
/// 
/// # Note
/// This is computationally more expensive than Euclidean averaging but produces
/// better centroids for time series data with temporal variations.
pub fn barycenter_recalculate<T>(
    data: &HashMap<usize, T>,
    model: &mut TimeSeriesKmeans<T>,
    assigned: &HashMap<usize, usize>
) 
where 
    T: Clone + KmeansValue + Send + Sync + Debug + PartialEq + EuclideanDistance + DtwDistance
{
    // Get the number of DBA iterations from the model configuration
    let iterations = model.barycenter_iteration.unwrap();
    
    // Initialize barycenters with current centroids
    let mut new_barycentroids: HashMap<usize, T> = model.centroid.clone();
    
    // Iteratively refine barycenters through DBA iterations
    for _iteration in 0..iterations {
        // Step 1: Align all time series to their cluster's current barycenter
        // and compute warping sums and valence (alignment counts)
        let new_iter_barycentroids: HashMap<usize, (T, T)> = data.par_iter()
            .map(|(data_id, row)|{
                // Get the cluster assignment for this data point
                let cluster = assigned.get(data_id).unwrap();
                let centroid = new_barycentroids.get(cluster);
                if centroid == None {
                    panic!("There is not cluster after assigning");
                }
                
                // Compute DTW alignment path between this time series and its centroid
                let (dtw_path, _dtw) = match model.metric {
                    DistanceMetric::DTW => centroid.unwrap().dtw_path(&row),
                    DistanceMetric::DtwWindowed(window) => centroid.unwrap().dtw_path_windowed( &row, window),
                    DistanceMetric::Euclidean => centroid.unwrap().dtw_path_windowed( &row, 1)
                };
                
                // Compute warping sums and valence based on DTW alignment
                // warping: sum of aligned values, valence: count of alignments per position
                let (warping, valence) = row.get_warping_valence(&dtw_path);

                (*cluster, warping, valence)
            })
            // Step 2: Aggregate warping and valence for each cluster (parallel fold)
            .fold(|| HashMap::new(), |mut acc: HashMap<usize, (T, T)>, right| {

                // Accumulate warping sums and valence for each cluster
                let (mut warping, mut valence) = acc.remove(&right.0)
                    .unwrap_or((T::zero(), T::zero()));
                warping = warping.sum_by_field(&right.1);
                valence = valence.sum_by_field(&right.2);
                acc.insert(right.0, (warping, valence));
                acc
            })
            // Step 3: Reduce (combine) the partial results from parallel threads
            .reduce(|| HashMap::new(), |mut acc, right| {
                // Combine warping and valence from different parallel partitions
                right.into_iter().for_each(|(clust_id, (warping, valence))| {
                    let (mut warping_l, mut valence_l) = acc.remove(&clust_id)
                        .unwrap_or((T::zero(), T::zero()));
                    warping_l = warping_l.sum_by_field(&warping);
                    valence_l = valence_l.sum_by_field(&valence);
                    
                    acc.insert(clust_id, (warping_l, valence_l));
                });
                acc
            });
        
        // Step 4: Compute new barycenter for each cluster by dividing warping by valence
        // This gives the weighted average at each time position based on DTW alignments
        let new_iter_barycentroids: HashMap<usize, T> = new_iter_barycentroids.into_par_iter().map(|(i, (warping, valence))| {
            let new_centroid: T = warping.div(&valence);
            (i, new_centroid)
        }).collect();
        
        // Update barycenters for next iteration
        new_barycentroids = new_iter_barycentroids;
    }

    // Update the model's centroids with the computed barycenters
    new_barycentroids.iter().for_each(|(clust_id, centroid)| {
        match model.centroid.get_mut(clust_id) {
            Some(c) => {
                *c = centroid.clone();
            },
            None => {
                model.centroid.insert(*clust_id, centroid.clone());
            }
        };
    });
}