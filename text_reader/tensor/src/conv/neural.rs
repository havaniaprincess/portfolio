use rayon::iter::IntoParallelRefIterator;
use rayon::prelude::*;

use crate::{conv::ConvLayer, malgebra::matrix::{Matrix, VecMatrix}, traits::Neural};

fn padding(
    input: &mut VecMatrix,
    pad: (usize, usize),
) {
    if pad.0 == 0 && pad.1 == 0 {
        return;
    }

    input.0.iter_mut()
        .for_each(|param| {
            param.padding(pad);
        });
}

impl Neural for ConvLayer {
    type OutType = Self;
    async fn fit(&mut self, input: &VecMatrix, d_out: &VecMatrix) {
        let mut indata = input.clone();
        dbg!((indata.0.len(),indata.0[0].0.len(),indata.0[0].0[0].len()));
        padding(&mut indata, self.padding);
        dbg!((indata.0.len(),indata.0[0].0.len(),indata.0[0].0[0].len()));
        dbg!((indata.0.len(),indata.0[0].0.len(),indata.0[0].0[0].len()));

        let dy_dw: Vec<VecMatrix> = (0..self.output.0).into_par_iter().map(|feachure| {
            VecMatrix((0..self.input.0).into_par_iter().map(|channel| {
                indata.0[channel].convariation(&d_out.0[feachure], self.stride, (self.window.1, self.window.2), self.padding)
            }).collect())
        }).collect();
        //dbg!(&dy_dw);
        let dy_dx: VecMatrix = VecMatrix(
        (0..self.input.0).into_par_iter().map(|channel| {
            let mut mat_res: Matrix = Matrix(vec![vec![0.0; self.input.2]; self.input.1]);
            (0..self.input.1).for_each(|h| {
                (0..self.input.2).for_each(|w| {
                    (0..self.output.0).for_each(|feachure| {
                        (0..self.window.1).for_each(|m| {
                            (0..self.window.2).for_each(|n| {
                                if m <= h + self.padding.0 && (h + self.padding.0 - m) / self.stride.0 < d_out.0[0].0.len() && n <= w + self.padding.1 && (w + self.padding.1 - n) / self.stride.1 < d_out.0[0].0[0].len() {
                                    let h_out = (h + self.padding.0 - m) / self.stride.0;
                                    let w_out = (w + self.padding.1 - n) / self.stride.1;
                                    
                                    if (h + self.padding.0 - m) % self.stride.0 == 0 && (w + self.padding.1 - n) % self.stride.1 == 0 {
                                        mat_res.0[h][w] += d_out.0[feachure].0[h_out][w_out] * self.weight[feachure].0[channel].0[m][n];
                                    }
                                }
                            });
                        });
                    });
                });
            });
            mat_res
        }).collect());
        dbg!((dy_dw.len(), dy_dw[0].0.len(), dy_dw[0].0[0].0.len(), dy_dw[0].0[0].0[0].len()));
        dbg!((dy_dx.0.len(), dy_dx.0[0].0.len(), dy_dx.0[0].0[0].len()));
        //dbg!(&dy_dx);
        
    }
    async fn predict(&self, input: &VecMatrix) -> VecMatrix {
        let mut indata = input.clone();

        let res: VecMatrix = match self.method {
            super::ConvMethod::FFT => {
                VecMatrix(self.weight.par_iter()
                    .map(|camera| {
                        let out_matrix = indata.fft_convariation(camera, self.stride);
                        out_matrix
                    }).collect())
            },
            super::ConvMethod::Canonical => {
                dbg!((indata.0.len(),indata.0[0].0.len(),indata.0[0].0[0].len()));
                padding(&mut indata, self.padding);
                dbg!((indata.0.len(),indata.0[0].0.len(),indata.0[0].0[0].len()));
                VecMatrix(self.weight.par_iter()
                    .map(|camera| {
                        let out_matrix = indata.convariation(camera, self.stride);
                        out_matrix
                    }).collect())
            }
        };
        
        res

    }
}


#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use rand::rng;
    use rand_distr::{Normal, Distribution};

    use crate::conv::ConvMethod;

    use super::*;

    #[tokio::test]
    async fn conv_read_write_test() {
        let v = vec![1,2,3,4,5,6,7,8,9,10];
        let normal = Normal::new(0.5, 0.15).unwrap();
        let mut rng = rng();
        let input: VecMatrix = VecMatrix((0..512).map(|_| {
            Matrix((0..7).map(|_| {
                (0..7).map(|_| normal.sample(&mut rng) as f64).collect()
            }).collect())
        }).collect());
        let mut result = ConvLayer::new((512,7,7), (3,3), 512, (1,1), (1, 1), crate::conv::ConvMethod::FFT);

        dbg!(result.output);

        let start_makesession = Instant::now();
        dbg!(input.0.len());
        let pred_out = result.predict(&input).await;
        let predict_time: Duration = start_makesession.elapsed();
        dbg!(predict_time);
        dbg!((pred_out.0.len(), pred_out.0[0].0.len(), pred_out.0[0].0[0].len()));
        result.method = ConvMethod::Canonical;
        let start_makesession = Instant::now();
        let pred_out = result.predict(&input).await;
        let predict_time: Duration = start_makesession.elapsed();
        dbg!(predict_time);
        dbg!((pred_out.0.len(), pred_out.0[0].0.len(), pred_out.0[0].0[0].len()));

        let start_makesession = Instant::now();
        dbg!(input.0.len());
        result.fit(&input, &pred_out).await;
        let fit_time: Duration = start_makesession.elapsed();
        dbg!(fit_time);
        dbg!((result.weight.len(), result.weight[0].0.len(), result.weight[0].0[0].0.len(), result.weight[0].0[0].0[0].len()));

        assert_eq!(2, 4);
    }
}