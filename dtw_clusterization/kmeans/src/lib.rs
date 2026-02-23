
pub mod time_series;
pub mod types;
mod barycenters;
mod euclidean_centers;
mod init_plusplus;
mod tools;

#[cfg(test)]
mod tests {
    //use super::*;

    use rand::{RngExt, SeedableRng, seq::IndexedRandom};
    use rand_chacha::ChaChaRng;

    #[test]
    fn it_works() {
        let mut rng = ChaChaRng::seed_from_u64(55);
        dbg!(rng.random_range(0.0..52.0));
        dbg!(rng.random_range(0.0..52.0));
        let mut rng = ChaChaRng::seed_from_u64(55);
        dbg!(rng.random_range(0.0..52.0));
        dbg!(rng.random_range(0.0..52.0));
        let mut rng = ChaChaRng::seed_from_u64(55);
        dbg!(rng.random_range(0.0..52.0));
        dbg!(rng.random_range(0.0..52.0));

        let mut rng = ChaChaRng::seed_from_u64(55);
        let mut veca = vec![25, 25,36,78,95,45,2,1,96,5];
        veca.sort();
        dbg!(&veca);
        let idx = if let Some(&first_index) = veca.choose(&mut rng) {
            Some(first_index)
        } else {
            None
        };
        dbg!(&idx);
        
        let mut rng = ChaChaRng::seed_from_u64(55);
        let mut veca = vec![25, 25,36,78,95,45,2,1,96,5];
        veca.sort();
        dbg!(&veca);
        let idx = if let Some(&first_index) = veca.choose(&mut rng) {
            Some(first_index)
        } else {
            None
        };
        dbg!(&idx);
        
        let mut rng = ChaChaRng::seed_from_u64(55);
        let mut veca = vec![25, 25,36,78,95,45,2,1,96,5];
        veca.sort();
        dbg!(&veca);
        let idx = if let Some(&first_index) = veca.choose(&mut rng) {
            Some(first_index)
        } else {
            None
        };
        dbg!(&idx);
        assert_eq!(5, 4);
    }
}
