use crate::{conv::{ConvLayer, ConvMethod}, malgebra::matrix::{Matrix, VecMatrix}};
use rand::rng;
use rand_distr::{Normal, Distribution};
use rayon::prelude::*;
use std::time::{Duration, Instant};


impl ConvLayer {
    pub fn new(input: (usize, usize, usize), window: (usize, usize), output_feachures: usize, stride: (usize, usize), padding: (usize, usize), method: ConvMethod) -> Self {
        let output: (usize, usize) = ((input.1 + 2 * padding.0 - window.0) / stride.0, (input.2 + 2 * padding.1 - window.1) / stride.1);
        let mean = 0.0;
        let std = (2.0 / ((input.0 * window.0 * window.1) as f64)).sqrt();
        
        
        let start_makesession = Instant::now();
        let normal = Normal::new(mean, std).unwrap();
        let mut rng = rng();
        let vec_w: Vec<f64> = (0..output_feachures * input.0 * window.0 * window.1).map(|_| normal.sample(&mut rng) as f64).collect();
        let weight: Vec<VecMatrix> = vec_w.par_windows(input.0 * window.0 * window.1)
            .step_by(input.0 * window.0 * window.1)
            .map(|feat| {
                VecMatrix(feat.par_windows( window.0 * window.1)
                    .step_by( window.0 * window.1)
                    .map(|oh| {
                        Matrix(oh.par_windows(window.1)
                            .step_by(window.1)
                            .map(|ow| {
                                
                                //let normal = Normal::new(mean, std).unwrap();
                                //let mut rng = rng();
                                //(0..window.1).map(|_| normal.sample(&mut rng) as f64).collect()
                                ow.to_vec()
                            }).collect())
                    }).collect())
            }).collect();
        let common_session: Duration = start_makesession.elapsed();
        dbg!(common_session);
        Self { 
            input: input, 
            window: (input.0, window.0, window.1), 
            stride: stride, 
            output: (output_feachures, output.1, output.0),
            weight: weight,
            padding: padding,
            method: method,
        }
    }
    pub fn get_weight_count(&self) -> u64 {
        return (self.output.0 * self.window.0 * self.window.1 * self.window.2) as u64;
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conv_test() {
        let result = ConvLayer::new((3,256,256), (7,7), 64, (2,2), (3, 3), ConvMethod::FFT);
        dbg!(result.get_weight_count());
        //dbg!(result);
        assert_eq!(2, 4);
    }
}