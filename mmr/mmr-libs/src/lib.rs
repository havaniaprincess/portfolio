pub mod leaderboard_v1;
pub mod types;
pub mod leaderboard_row;
pub mod userstat;
pub mod datasets;
pub mod math;
pub mod memory;
pub mod writer;
pub mod statistic;
pub mod leaderboard_v2;
pub mod spread;
pub mod reader;

#[cfg(test)]
mod tests {

    #[test]
    fn it_works() {
        let result = 4;
        assert_eq!(result, 4);
    }
}
