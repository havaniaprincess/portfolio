use std::{collections::HashMap, time::{Duration, Instant}};

use std::fmt::Debug;
use rayon::prelude::*;
use rand::SeedableRng;
use crate::{barycenters::barycenter_recalculate, euclidean_centers::euclidean_recalculate, init_plusplus::set_centroid_by_data, types::{DistanceMetric, DtwDistance, EuclideanDistance, KMeansModel, KmeansValue}};
use rand_chacha::{ChaCha20Rng, ChaChaRng};

// Define the TimeSeriesKmeans struct as a generic struct that implements the KMeansModel for time series data. It includes fields for the number of clusters (k), batch size, dimension size, centroids, distance metric, random number generator, and an optional field for barycenter iteration.
// Generic type T is used to allow for flexibility in the type of data being clustered, as long as it implements the necessary traits for distance calculations and other operations required by the KMeans algorithm.
#[derive(Clone, Debug)]
pub struct TimeSeriesKmeans<T>
{
    pub k: usize,
    pub batch_size: usize,
    pub dim_size: usize,
    pub centroid: HashMap<usize, T>,
    pub metric: DistanceMetric,
    pub rng: ChaCha20Rng,
    pub barycenter_iteration: Option<usize>
}

impl<T> KMeansModel for TimeSeriesKmeans<T>{}

// Implement the TimeSeriesKmeans struct with a constructor method (new) that initializes the fields based on the provided parameters. The constructor allows for optional parameters for centroid values and barycenter iteration, and sets default values if they are not provided. The random number generator is seeded for reproducibility.
impl<T> TimeSeriesKmeans<T>
where 
    T: Clone + KmeansValue + Send + Sync + Debug + PartialEq + EuclideanDistance + DtwDistance
{
    pub fn new(k: usize, dim_size: usize, batch:usize, metric: Option<DistanceMetric>, centroid_value: Option<HashMap<usize, T>>, seed: u64, barycenter_iteration: Option<usize>) -> Self {
        let res_centroids: HashMap<usize, T> = match centroid_value {
            Some(centroid) => centroid.clone(),
            None => {
                let mut centroid: HashMap<usize, T> = HashMap::new();
                for clust in 0..k {
                    let clust_vec: T = T::zero();
                    centroid.insert(clust, clust_vec);
                }
                centroid
            }
        };
        Self { k: k, batch_size: batch, centroid: res_centroids, dim_size: dim_size, rng: ChaChaRng::seed_from_u64(seed), barycenter_iteration: barycenter_iteration, metric: metric.unwrap_or(DistanceMetric::Euclidean) }
    }
}

// The fit function is the main function for fitting the KMeans model to the provided time series data. It takes in a reference to the data, a mutable reference to the model, and the number of iterations to perform.
pub fn fit<T>(
    data: &HashMap<usize, T>,
    model: &mut TimeSeriesKmeans<T>, 
    n_iteration: usize
) -> ((Instant, Duration), Vec<((Instant, Duration), (Instant, Duration), (Instant, Duration))>, HashMap<usize, usize>)
where 
    T: Clone + KmeansValue + Send + Sync + Debug + PartialEq + EuclideanDistance + DtwDistance
{
    // Init centroids based on data
    model.centroid = set_centroid_by_data(data, &model, |distance_metric, row, centroid| match distance_metric {
        DistanceMetric::DTW => row.dtw_path(&centroid).1,
        DistanceMetric::DtwWindowed(window) => row.dtw_path_windowed( &centroid, *window).1,
        DistanceMetric::Euclidean => row.euclidean_distance(&centroid)
    }).unwrap_or(model.centroid.clone());

    // First assignment time series to centroids
    let mut assigned = assign_points(data, &model);
    let fit_timer = Instant::now();
    let iteration_timers: Vec<((Instant, Duration), (Instant, Duration), (Instant, Duration))> = (0..n_iteration).map(|i| {
        let iteration_timer = Instant::now();
        let recalculate_timer = Instant::now();
        // Recalculate centroids based on assigned points and next assignment of points to centroids
        recalculate(data, model, &assigned);
        
        let recalculate_timer_time = recalculate_timer.elapsed();
        let assign_timer = Instant::now();


        assigned = assign_points(data, &model);
        let assign_timer_time = assign_timer.elapsed();
        let iteration_timer_time = iteration_timer.elapsed();
        //println!("Iterate {:?}  ---  {:?}", i, iteration_timer_time);
        println!("End common {} iteration", i);
        ((iteration_timer, iteration_timer_time), (assign_timer, assign_timer_time), (recalculate_timer, recalculate_timer_time))
    }).collect();
    let fit_timer_time = fit_timer.elapsed();

    //println!("assigned: {}", assigned.len());
    //dbg!(&model.centroid);
    ((fit_timer, fit_timer_time), iteration_timers, assigned)
}

fn assign_points<T>(
    data: &HashMap<usize, T>,
    model: &TimeSeriesKmeans<T>
) -> HashMap<usize, usize> 
where 
    T: Clone + KmeansValue + Send + Sync + Debug + EuclideanDistance + DtwDistance
{
    let assigned: HashMap<usize, usize> = data.par_iter()
        .map(|row| (*row.0, *model.centroid.iter()
            .map(|(idx, centroid)| (idx, match model.metric {
                DistanceMetric::DTW => row.1.dtw_path(&centroid).1,
                DistanceMetric::DtwWindowed(window) => row.1.dtw_path_windowed( &centroid, window).1,
                DistanceMetric::Euclidean => row.1.euclidean_distance(&centroid)
            }))
            .min_by(|a, b| a.1.partial_cmp(&b.1).unwrap())
            .unwrap().0)
        )
        .collect();

    assigned
} 
fn recalculate<T>(
    data: &HashMap<usize, T>,
    model: &mut TimeSeriesKmeans<T>,
    assigned: &HashMap<usize, usize>
) 
where 
    T: Clone + KmeansValue + Send + Sync + Debug + PartialEq + EuclideanDistance + DtwDistance
{
    if model.barycenter_iteration == None {
        euclidean_recalculate(data, model, assigned);
    } else {
        barycenter_recalculate(data, model, assigned);
    }
    
}