//! Type definitions and trait implementations for K-Means clustering
//! Includes distance metrics (DTW, Euclidean), mathematical operations,
//! and helper traits for working with time series data

use std::{collections::HashMap, iter::Sum, ops::{Add, Div, Mul, Sub}};

use crate::tools::{get_path_hashmap, get_path_vec};

/// Marker trait for K-Means model types
pub trait KMeansModel{}

/// Distance metrics available for clustering
#[derive(Clone, Debug)]
pub enum DistanceMetric{
    /// Standard Dynamic Time Warping (no window constraint)
    DTW,
    /// DTW with Sakoe-Chiba band window constraint for faster computation
    DtwWindowed(usize),
    /// Standard Euclidean distance (assumes perfect alignment)
    Euclidean
}

/// Mathematical helper functions for computing square roots
pub trait MathFun {
    /// Calculate square root of the value
    fn sqrtm(&self) -> f64;
}

impl MathFun for f64 {
    fn sqrtm(&self) -> f64 {
        f64::sqrt(*self)
    }
}

impl MathFun for u64 {
    fn sqrtm(&self) -> f64 {
        f64::sqrt(*self as f64)
    }
}

impl MathFun for i64 {
    fn sqrtm(&self) -> f64 {
        f64::sqrt(*self as f64)
    }
}

/// Operations required for K-Means centroid calculations
pub trait KmeansValue {
    /// Create a zero/empty value
    fn zero() -> Self;
    /// Element-wise sum of two values
    fn sum_by_field(&self, right: &Self) -> Self;
    /// Divide all elements by a scalar value (for averaging)
    fn div_by_n(&self, div: usize) -> Self;
    /// Element-wise division
    fn div(&self, div: &Self) -> Self;
}

/// Implementation of KmeansValue for Vec (dense time series)
impl<D> KmeansValue for Vec<D> 
where 
    D: Add<Output = D> + Div<Output = D> + From<u32> + Copy
{
    /// Create an empty vector
    fn zero() -> Self {
        Vec::new()
    }
    /// Element-wise addition of two vectors
    fn sum_by_field(&self, right: &Self) -> Self {
        self.iter().zip(right.iter()).map(|(a, b)| a.add(*b)).collect()
    }
    /// Divide all elements by scalar (for centroid averaging)
    fn div_by_n(&self, div: usize) -> Self {
        self.iter().map(|a| *a / (div as u32).into()).collect()
    }
    /// Element-wise division of two vectors
    fn div(&self, right: &Self) -> Self {
        self.iter().zip(right.iter()).map(|(a, b)| a.div(*b)).collect()
    }
}

/// Implementation of KmeansValue for HashMap (sparse time series)
impl<D> KmeansValue for HashMap<usize, D> 
where 
    D: Add<Output = D> + Div<Output = D> + From<u32> + Copy
{
    /// Create an empty HashMap
    fn zero() -> Self {
        HashMap::new()
    }
    /// Element-wise addition of two HashMaps (handles missing keys with 0)
    fn sum_by_field(&self, right: &Self) -> Self {
        let mut left = self.clone();
        right.into_iter().for_each(|(i, a)| {
            let b = left.get(i).copied().unwrap_or((0 as u32).into()).add(*a);
            left.insert(*i, b);
        });
        left
    }
    /// Divide all values by scalar (for centroid averaging)
    fn div_by_n(&self, div: usize) -> Self {
        self.into_iter().map(|(i, a)| (*i, *a / (div as u32).into())).collect()
    }
    /// Element-wise division of two HashMaps
    fn div(&self, right: &Self) -> Self {
        let mut left = self.clone();
        right.into_iter().for_each(|(i, a)| {
            let b = left.get(i).copied().unwrap_or((0 as u32).into()).div(*a);
            left.insert(*i, b);
        });
        left
    }
}

/// Trait for calculating Euclidean distance between time series
pub trait EuclideanDistance {
    /// Calculate Euclidean distance between two time series
    fn euclidean_distance(&self, right: &Self) -> f64;
}

/// Euclidean distance implementation for Vec
impl<D> EuclideanDistance for Vec<D> 
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy
{
    /// Calculate sqrt(sum((a[i] - b[i])^2)) for all paired elements
    fn euclidean_distance(&self, right: &Self) -> f64 {
        self.iter().zip(right.iter())
            .filter_map(|(&a, &b)| Some(((a - b) * (a - b)).into()))
            .sum::<f64>()
            .sqrt()
    }
}

/// Euclidean distance implementation for HashMap (only compares common keys)
impl<D> EuclideanDistance for HashMap<usize, D> 
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy
{
    /// Calculate distance for keys present in both HashMaps
    fn euclidean_distance(&self, right: &Self) -> f64 {
        self.iter()
            .filter_map(|(key, &val1)| right.get(key).map(|&val2| ((val1 - val2) * (val1 - val2)).into()))
            .sum::<f64>()
            .sqrt()
    }
}

/// Trait for calculating Dynamic Time Warping distance and alignment
pub trait DtwDistance 
where
    Self: Sized
{
    /// Calculate DTW distance and optimal warping path (no window constraint)
    fn dtw_path(&self, right: &Self) -> (Vec<(usize, usize)>, f64);
    /// Calculate DTW distance with Sakoe-Chiba band window constraint
    fn dtw_path_windowed(&self, right: &Self, window: usize) -> (Vec<(usize, usize)>, f64);
    /// Get warping sums and valence counts for barycenter averaging
    fn get_warping_valence(&self, warp_path: &Vec<(usize, usize)>) -> (Self, Self);
}

/// DTW distance implementation for Vec
impl<D> DtwDistance for Vec<D> 
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy + Sum + From<u32>
{
    /// Calculate DTW distance without window constraint
    fn dtw_path(&self, right: &Self) -> (Vec<(usize, usize)>, f64) {
        let len1 = self.len();
        let len2 = right.len();
        let result = get_path_vec(self, right, None);
        if result.is_none() {
            return (Vec::new(), f64::INFINITY);
        }
        let (cost_matrix, path_matrix) = result.unwrap();
        
        // Backtrack from end to start to construct optimal warping path
        let mut warp_path = vec![];
        let mut i = len1 - 1;
        let mut j = len2 - 1;

        while let Some((pi, pj)) = path_matrix.get(i).unwrap_or(&Vec::new()).get(j).copied().unwrap_or(None) {
            warp_path.push((i, j));
            i = pi;
            j = pj;
        }
        warp_path.push((0, 0));
        warp_path.reverse();
        (warp_path, cost_matrix[len1-1][len2-1].sqrt())
    }
    /// Calculate DTW distance with Sakoe-Chiba band window constraint
    fn dtw_path_windowed(&self, right: &Self, window: usize) -> (Vec<(usize, usize)>, f64) {
        let len1 = self.len();
        let len2 = right.len();
        let result = get_path_vec(self, right, Some(window));
        if result.is_none() {
            return (Vec::new(), f64::INFINITY);
        }
        let (cost_matrix, path_matrix) = result.unwrap();

        // Backtrack to construct optimal warping path
        let mut warp_path = vec![];
        let mut i = len1 - 1;
        let mut j = len2 - 1;

        while let Some((pi, pj)) = path_matrix.get(i).unwrap_or(&Vec::new()).get(j).copied().unwrap_or(None) {
            warp_path.push((i, j));
            i = pi;
            j = pj;
        }
        warp_path.push((0, 0));
        warp_path.reverse();
        (warp_path, cost_matrix[len1 - 1][len2 - 1].sqrt())
    }
    /// Calculate warping sums and alignment counts for DTW barycenter averaging
    /// Returns (warping, valence) where warping contains sums of aligned values
    /// and valence contains counts of how many times each position was aligned
    fn get_warping_valence(&self, warp_path: &Vec<(usize, usize)>) -> (Self, Self) {
        // Sum of all values aligned to each position in left series
        let warping: Vec<D> = self.iter().enumerate()
            .map(|(i, _y)| {
                warp_path.iter().filter_map(|(l_i, r_i)| if i == *l_i {Some(self[*r_i])} else {None}).sum::<D>()
            }).collect();
        // Count of alignments for each position in left series
        let valence: Vec<D> = self.iter().enumerate()
            .map(|(i, _y)| {
                warp_path.iter().filter_map(|(l_i, _r_i)| if i == *l_i {Some(1.into())} else {None}).sum::<D>()
            }).collect();
        (warping, valence)
    }
}

/// DTW distance implementation for HashMap
impl<D> DtwDistance for HashMap<usize, D> 
where 
    D: Sub<Output = D> + Mul<Output = D> + Into<f64> + Copy + Sum + From<u32>
{
    /// Calculate DTW distance without window constraint
    fn dtw_path(&self, right: &Self) -> (Vec<(usize, usize)>, f64) {
        let len1 = self.len();
        let len2 = right.len();
        
        let (cost_matrix, path_matrix) = get_path_hashmap(self, right, None);

        // Backtrack from end to start to construct optimal warping path
        let mut warp_path = vec![];
        let mut i = len1 - 1;
        let mut j = len2 - 1;
        while let Some((pi, pj)) = path_matrix.get(i).unwrap_or(&Vec::new()).get(j).copied().unwrap_or(None) {
            warp_path.push((i, j));
            i = pi;
            j = pj;
        }
        warp_path.push((0, 0));
        warp_path.reverse();
        (warp_path, cost_matrix[len1-1][len2-1].sqrt())        
    }
    /// Calculate DTW distance with Sakoe-Chiba band window constraint
    fn dtw_path_windowed(&self, right: &Self, window: usize) -> (Vec<(usize, usize)>, f64) {
        let len1 = self.len();
        let len2 = right.len();
    
        let (cost_matrix, path_matrix) = get_path_hashmap(self, right, Some(window));
        
        // Backtrack to construct optimal warping path
        let mut warp_path = vec![];
        let mut i = len1 - 1;
        let mut j = len2 - 1;

        while let Some((pi, pj)) = path_matrix.get(i).unwrap_or(&Vec::new()).get(j).copied().unwrap_or(None) {
            warp_path.push((i, j));
            i = pi;
            j = pj;
        }
        warp_path.push((0, 0));
        warp_path.reverse();
        (warp_path, cost_matrix[len1-1][len2-1].sqrt())        
    }
    /// Calculate warping sums and alignment counts for DTW barycenter averaging
    /// Returns (warping, valence) where warping contains sums of aligned values
    /// and valence contains counts of how many times each position was aligned
    fn get_warping_valence(&self, warp_path: &Vec<(usize, usize)>) -> (Self, Self) {
        // Sum of all values aligned to each position in left series
        let warping: HashMap<usize, D> = self.iter().enumerate()
            .map(|(i, _y)| {
                (i, warp_path.iter().filter_map(|(l_i, r_i)| {
                    let x = self.get(r_i).copied().unwrap_or((0 as u32).into());
                    if i == *l_i {Some(x)} else {None}
                }).sum::<D>())
            }).collect();
        // Count of alignments for each position in left series
        let valence: HashMap<usize, D> = self.iter().enumerate()
            .map(|(i, _y)| {
                (i, warp_path.iter().filter_map(|(l_i, _r_i)| if i == *l_i {Some(1.into())} else {None}).sum::<D>())
            }).collect();
        (warping, valence)
    }
}
