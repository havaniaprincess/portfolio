use crate::malgebra::matrix::{Matrix, VecMatrix};

pub mod impliment;
pub mod fs;
pub mod neural;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub enum ConvMethod{
    FFT,
    Canonical,
}

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct ConvLayer {
    // params, height, width
    pub input: (usize, usize, usize),
    // params, height, width
    pub window: (usize, usize, usize),
    pub stride: (usize, usize),
    pub output: (usize, usize, usize),
    // feature -> params -> win_h -> win_w
    pub weight: Vec<VecMatrix>,
    pub padding: (usize, usize),
    pub method: ConvMethod,
}