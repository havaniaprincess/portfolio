mod statistic;

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}


#[cfg(test)]
mod tests {
    use crate::statistic::DBStatistic;

    use super::*;

    //#[test]
    fn fish_stat() {
        let db = DBStatistic("../database_data/".to_string());
        let fish_name = "солнечник красноухий".to_string();
        db.get_point_info_fish(&fish_name, &"../krasn.csv".to_string(), &None);
        let result = add(2, 2);
        assert_eq!(2, 4);
    }
    
    #[test]
    fn fish_stat_full() {
        let db = DBStatistic("../database_data/".to_string());
        //let fish_name = "солнечник красноухий".to_string();
        db.make_statistic( &"../statistic.csv".to_string(), &None, 1748736000000);
        let result = add(2, 2);
        assert_eq!(2, 4);
    }
}
