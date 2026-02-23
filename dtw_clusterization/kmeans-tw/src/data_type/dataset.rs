use std::{collections::HashMap, fmt::Debug};


use crate::data_type::{timewrap::TimeWrap, traits::Transponent};

#[derive(Clone, Debug)]
pub struct DataCollection<T> {
    pub right: HashMap<usize, T>,
    pub transponent: HashMap<usize, HashMap<usize, f64>>
}

impl DataCollection<TimeWrap> {
    pub fn new(right: &HashMap<usize, TimeWrap>, normal: bool) -> Self {
        let normalized: HashMap<usize, TimeWrap> = if normal { right.iter().map(|(idx, line)| {
            let (sum, count) = line.0.iter().fold((0.0, 0 as usize), |(s_i, c_i), (_item_id, item)| {
                let nc: usize = c_i + 1;
                (s_i+(item).powf(2.0), nc)
            });
            let average_dev = 3.0 * (sum / (count as f64)).sqrt();

            (*idx, TimeWrap(line.0.iter().map(|(item_id, item)| (*item_id, 2.0 * (*item) / average_dev)).collect()))

        }).collect()} else {right.clone()};

        Self { right: normalized.clone(), transponent: normalized.transponent() }
    }
    pub fn from_vec(right: &Vec<Vec<f64>>, normal: bool) -> Self {
        let right_hash: HashMap<usize, TimeWrap> = right.iter().enumerate()
            .map(|(data_id, d)| (
                data_id,
                TimeWrap(d.iter().enumerate().map(|(d_id, p)| (d_id, *p)).collect())
            )).collect();

        Self::new(&right_hash, normal)
    }
}

impl Transponent<TimeWrap> for HashMap<usize, TimeWrap>
{
    type OutType = HashMap<usize, HashMap<usize, f64>>;
    fn transponent(&self) -> HashMap<usize, HashMap<usize, f64>> {
        //dbg!(self.keys());
        let mut res: HashMap<usize, HashMap<usize, Option<f64>>> = (0..self[&self.keys().next().take().unwrap()].0.len()).enumerate().map(|(idx, _)| (idx, HashMap::new())).collect();
        for (row_id, row) in self.iter() {
            for (dim_id, dim) in row.0.iter() {
                match res.get_mut(dim_id) {
                    Some(d) => {
                        d.insert(*row_id, Some(dim.clone()));
                    },
                    None => {
                        let mut d: HashMap<usize, Option<f64>> = HashMap::new();
                        d.insert(*row_id, Some(dim.clone()));
                        res.insert(*dim_id, d);
                    }
                };
            }
        }
        
        res.into_iter()
            .map(|(idx, item_vec)| 
               ( idx, item_vec.into_iter().map(|(item_idx, item)| match item {
                    Some(data) => (item_idx, data),
                    None => panic!("Error with unknown item in tranponent array")
                }).collect::<HashMap<usize, f64>>())
            ).collect()
    }
}
