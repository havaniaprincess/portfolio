use std::fmt::Debug;

use rustfft::{FftPlanner, num_complex::Complex};
use rustfft::num_traits::Zero;

#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct Matrix(pub Vec<Vec<f64>>);
#[derive(Clone, Debug, serde::Deserialize, serde::Serialize)]
pub struct VecMatrix(pub Vec<Matrix>);

impl Matrix {
    pub fn padding(&mut self, pad: (usize, usize)) {
        self.0.iter_mut()
            .for_each(|item| {
                padding_vec(item, pad.1, 0.0);
            });
        let line_vec = self.0[0].len();
        padding_vec(&mut self.0, pad.0, vec![0.0; line_vec]);
    }
    pub fn zero_padding(&mut self, pad: (usize, usize)) {
        self.0.iter_mut()
            .for_each(|item| {
                zero_padding_vec(item, pad.1, 0.0);
            });
        let line_vec = self.0[0].len();
        zero_padding_vec(&mut self.0, pad.0, vec![0.0; line_vec]);
    }
    pub fn convariation(&self, filter: &Matrix, stride: (usize, usize), kernel_size: (usize, usize), padding: (usize, usize)) -> Matrix {
        let kernel = filter.clone();
        let input = self.clone();
        let input_rows = input.0.len();
        let input_cols = input.0[0].len();
        //let in_channels_f = input.0.len();
        let kernel_rows = filter.0.len();
        let kernel_cols = filter.0[0].len();
        //dbg!((input.0.len(),input.0[0].len()));
        //dbg!((kernel.0.len(),kernel.0[0].len()));

        let mut result1: Matrix = Matrix(vec![vec![0.0; kernel_size.1]; kernel_size.0]);

        (0..kernel_size.0).for_each(|m| {
            (0..kernel_size.1).for_each(|n| {
                (0..kernel_rows).for_each(|h| {
                    (0..kernel_cols).for_each(|w| { 
                        if (h*stride.0 + m >= padding.0 && h*stride.0 + m - padding.0 < input_rows) && (w*stride.1 + n >= padding.1 && w*stride.1 + n - padding.1 < input_cols) {
                            let ix = h*stride.0 + m - padding.0;
                            let iy = w*stride.1 + n - padding.1;
                            result1.0[m][n] = filter.0[h][w] * input.0[ix][iy];
                        }
                    });
                });
            });
        });

        //dbg!(&input);
        //dbg!(&kernel);
        let indexies_input_row:Vec<usize> = (0..input_rows).step_by(stride.0).collect();
        let result: Vec<Vec<f64>> = indexies_input_row.windows(kernel_rows).map(|input_id_rows| {
            let indexies_input_col:Vec<usize> = (0..input_cols).step_by(stride.1).collect();
            indexies_input_col.windows(kernel_cols).map(|input_id_cols| {
                //dbg!(input_id_rows);
                //dbg!(input_id_cols);
                input_id_rows.iter().zip(kernel.0.iter()).map(|(row_id, kernel_row)| {
                    input_id_cols.iter().zip(kernel_row.iter()).map(|(col_id, kernel_value)| input.0[*row_id][*col_id] * kernel_value).sum::<f64>()
                }).sum::<f64>()
            }).collect()
        }).collect();

        //dbg!(&result);
        Matrix(result1.0)
    }
}

impl VecMatrix {
    pub fn fft_convariation(&self, filter: &VecMatrix, stride: (usize, usize)) -> Matrix {
        let mut kernel = filter.clone();
        let mut input = self.clone();
        //dbg!(&input);
        let in_channels = input.0.len();
        let input_rows = input.0[0].0.len();
        let input_cols = input.0[0].0[0].len();
        //let in_channels_f = input.0.len();
        let kernel_rows = filter.0[0].0.len();
        let kernel_cols = filter.0[0].0[0].len();
        //dbg!((in_channels, input_rows, input_cols));
        //dbg!((in_channels_f, kernel_rows, kernel_cols));

        let conv_rows = input_rows + kernel_rows - 1;
        let conv_cols = input_cols + kernel_cols - 1;
        //dbg!((conv_rows, conv_cols));

        let mut full_output = vec![vec![0.0 as f64; conv_cols]; conv_rows];

        let mut acc_fft = vec![vec![Complex::zero(); conv_cols]; conv_rows];

        let mut planner = FftPlanner::new();
        for ic in 0..in_channels {
            //let input_padded = pad_2d(&input[ic], conv_rows, conv_cols);
            input.0[ic].zero_padding((conv_rows - input_rows, conv_cols - input_cols));
            //let kernel_padded = pad_2d(&kernel[oc][ic], conv_rows, conv_cols);
            kernel.0[ic].zero_padding((conv_rows - kernel_rows, conv_cols - kernel_cols));
            /* let pad_input_rows = input.0[ic].0.len();
            let pad_input_cols = input.0[ic].0[0].len();
            dbg!((pad_input_rows, pad_input_cols));
            let pad_input_rows = kernel.0[ic].0.len();
            let pad_input_cols = kernel.0[ic].0[0].len();
            dbg!((pad_input_rows, pad_input_cols)); */

            let input_fft = fft2d(&input.0[ic], &mut planner, true);
            let kernel_fft = fft2d(&kernel.0[ic], &mut planner, true);

            for i in 0..conv_rows {
                for j in 0..conv_cols {
                    acc_fft[i][j] = acc_fft[i][j] + input_fft[i][j] * kernel_fft[i][j];
                }
            }
        }

        let convolved = ifft2d(&acc_fft, &mut planner);
        let mut out_matrix: Vec<Vec<f64>> = Vec::new();
        for i in (0..input_rows).step_by(stride.0) {
            let mut out_row: Vec<f64> = Vec::new();
            for j in (0..input_cols).step_by(stride.1) {
                if i < convolved.len() && j < convolved[0].len() {
                    out_row.push(convolved[i][j]);
                    full_output[i][j] = convolved[i][j];
                }
            }
            out_matrix.push(out_row);
        }

        //dbg!(&out_matrix);
        //dbg!(&full_output);
        Matrix(out_matrix)
    }
    pub fn convariation(&self, filter: &VecMatrix, stride: (usize, usize)) -> Matrix {
        let kernel = filter.clone();
        let input = self.clone();
        let in_channels = input.0.len();
        let input_rows = input.0[0].0.len();
        let input_cols = input.0[0].0[0].len();
        //let in_channels_f = input.0.len();
        let kernel_rows = filter.0[0].0.len();
        let kernel_cols = filter.0[0].0[0].len();

        //dbg!(&input);
        //dbg!(&kernel);
        let indexies_input_row:Vec<usize> = (0..input_rows).collect();
        let result: Vec<Vec<f64>> = indexies_input_row.windows(kernel_rows).step_by(stride.0).map(|input_id_rows| {
            let indexies_input_col:Vec<usize> = (0..input_cols).collect();
            indexies_input_col.windows(kernel_cols).step_by(stride.1).map(|input_id_cols| {
                //dbg!(input_id_rows);
                //dbg!(input_id_cols);
                (0..in_channels).map(|ch| {
                    input_id_rows.iter().zip(kernel.0[ch].0.iter()).map(|(row_id, kernel_row)| {
                        input_id_cols.iter().zip(kernel_row.iter()).map(|(col_id, kernel_value)| input.0[ch].0[*row_id][*col_id] * kernel_value).sum::<f64>()
                    }).sum::<f64>()
                }).sum()
            }).collect()
        }).collect();

        //dbg!(&result);
        Matrix(result)
    }
}

fn padding_vec<T>(vec: &mut Vec<T>, pad: usize, default: T) 
where 
    T: Clone+Default+Debug
{
    
        let old_len = vec.len();
        let left = vec.first().unwrap_or(&default).clone();
        let right = vec.last().unwrap_or(&default).clone();
        vec.resize(old_len + pad + pad, default);
        for i in (0..old_len).rev() { 
            vec[i+pad] = std::mem::take(&mut vec[i]); 
        }
        vec[0..pad].fill(left);
        vec[old_len + pad..].fill(right);
}

fn zero_padding_vec<T>(vec: &mut Vec<T>, pad: usize, default: T) 
where 
    T: Clone+Default
{
        let old_len = vec.len();
        vec.resize(old_len + pad, default);
}

fn fft2d(input: &Matrix, planner: &mut FftPlanner<f64>, forward: bool) -> Vec<Vec<Complex<f64>>> {
    let rows = input.0.len();
    let cols = input.0[0].len();
    //dbg!(cols);
    let mut complex_input: Vec<Vec<Complex<f64>>> = input.0
        .iter()
        .map(|row| row.iter().map(|&x| Complex::new(x, 0.0)).collect())
        .collect();

    for row in complex_input.iter_mut() {
        let fft = if forward {
            planner.plan_fft_forward(cols)
        } else {
            planner.plan_fft_inverse(cols)
        };
        //dbg!(row.len());
        fft.process(row);
        if !forward {
            for val in row.iter_mut() {
                *val /= cols as f64;
            }
        }
    }

    for col in 0..cols {
        let mut column: Vec<Complex<f64>> = complex_input.iter().map(|row| row[col]).collect();
        let fft = if forward {
            planner.plan_fft_forward(rows)
        } else {
            planner.plan_fft_inverse(rows)
        };
        fft.process(&mut column);
        if !forward {
            for val in column.iter_mut() {
                *val /= rows as f64;
            }
        }
        for (i, val) in column.into_iter().enumerate() {
            complex_input[i][col] = val;
        }
    }

    complex_input
}


fn ifft2d(input: &[Vec<Complex<f64>>], planner: &mut FftPlanner<f64>) -> Vec<Vec<f64>> {
    let rows = input.len();
    let cols = input[0].len();
    let mut real_input: Vec<Vec<f64>> = vec![vec![0.0; cols]; rows];
    let mut complex_input = input.to_vec();

    for row in complex_input.iter_mut() {
        let fft = planner.plan_fft_inverse(cols);
        fft.process(row);
        for val in row.iter_mut() {
            *val /= cols as f64;
        }
    }

    for col in 0..cols {
        let mut column: Vec<Complex<f64>> = complex_input.iter().map(|row| row[col]).collect();
        let fft = planner.plan_fft_inverse(rows);
        fft.process(&mut column);
        for val in column.iter_mut() {
            *val /= rows as f64;
        }
        for (i, val) in column.into_iter().enumerate() {
            real_input[i][col] = val.re;
        }
    }

    real_input
}


#[cfg(test)]
mod tests {
    use std::time::{Duration, Instant};

    use rand::rng;
    use rand_distr::{Normal, Distribution};

    use super::*;

    //#[tokio::test]
    async fn padding_test() {
        let v = vec![1,2,3,4,5,6,7,8,9,10];
        let normal = Normal::new(0.5, 0.15).unwrap();
        let mut rng = rng();
        let mut input: Vec<f64> = 
                (0..7).map(|_| normal.sample(&mut rng) as f64).collect();
        dbg!(&input);
        let start_makesession = Instant::now();
        let _ = padding_vec(&mut input, 2, 0.0);
        let padding_time: Duration = start_makesession.elapsed();
        dbg!(&input);
        dbg!(&padding_time);

        //dbg!(result);
        assert_eq!(2, 4);
    }
}