use crate::malgebra::matrix::{Matrix, VecMatrix};


pub trait ConfigFS {
    type OutType;
    async fn save(&self, path: &String);
    async fn read(path: &String) -> Option<Self::OutType>;
}

pub trait Neural {
    type OutType;
    async fn predict(&self, input: &VecMatrix) -> VecMatrix;
    async fn fit(&mut self, input: &VecMatrix, d_out: &VecMatrix);
}