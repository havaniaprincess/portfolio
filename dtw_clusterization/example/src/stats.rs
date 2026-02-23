use std::{collections::HashMap, fmt};

use kmeans_tw::data_type::{timewrap::{PaymentUser, TimeWrap}, types::{ClusterClass, ClusterSet}};
//use tokio::io::{AsyncWriteExt, BufWriter};

use crate::algorythm::get_stats_clusters;


#[derive(Clone)]
pub struct ClusterStatistic{
    pub cluster_id: usize,
    pub au: f64,
    pub au_rate: f64,
    pub pu: f64,
    pub revenue: f64,
    pub seabeast_pu: f64,
    pub seabeast_revenue: f64,
    pub pu_rate: f64,
    pub seabeast_pu_rate: f64,
    pub arpu: f64,
    pub seabeast_arpu: f64,
    pub not_seabeast_arpu: f64,
    pub arppu: f64,
    pub seabeast_arppu: f64,
    pub not_seabeast_arppu: f64,
    pub seabeast_pu_rate_f_au: f64,
}

impl ClusterStatistic{
    pub fn new(cluster_id: usize, au: f64, pu: f64, revenue: f64, seabeast_pu: f64, seabeast_revenue: f64, au_all: f64) -> Self {
        Self { cluster_id: cluster_id, au: au, au_rate: au / au_all, pu: pu, revenue: revenue, seabeast_pu: seabeast_pu, seabeast_revenue: seabeast_revenue, pu_rate: pu / au, seabeast_pu_rate: seabeast_pu / pu, arpu: revenue / au, seabeast_arpu: seabeast_revenue / au, not_seabeast_arpu: (revenue - seabeast_revenue) / au, arppu: revenue / pu, seabeast_arppu: seabeast_revenue / seabeast_pu, not_seabeast_arppu: (revenue - seabeast_revenue) / (pu - seabeast_pu), seabeast_pu_rate_f_au: seabeast_pu / au }
    }
    
    pub fn to_csv_info(&self) -> String {
        let result: String = format!("{};{};{};{};{};{};{};{};{};{};{};{};{};{};{}", 
            self.au, 
            self.au_rate, 
            self.pu, 
            self.revenue, 
            self.seabeast_pu, 
            self.seabeast_revenue, 
            self.pu_rate, 
            if self.seabeast_pu_rate.is_nan() {"".to_string()} else {self.seabeast_pu_rate.to_string()}, 
            self.seabeast_pu_rate_f_au, 
            self.arpu, 
            self.seabeast_arpu, 
            self.not_seabeast_arpu, 
            if self.arppu.is_nan() {"".to_string()} else {self.arppu.to_string()}, 
            if self.seabeast_arppu.is_nan() {"".to_string()} else {self.seabeast_arppu.to_string()}, 
            if self.not_seabeast_arppu.is_nan() {"".to_string()} else {self.not_seabeast_arppu.to_string()}
        );
        result
    }
}

impl fmt::Debug for ClusterStatistic {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "\nClusterStatistic {{\n")?;
        write!(f, "\t{}: {},\n", "cluster_id", self.cluster_id)?;
        write!(f, "\t{}: {},\n", "au", self.au as u64)?;
        write!(f, "\t{}: {:.4},\n", "au_rate", self.au_rate * 100.0)?;
        write!(f, "\t{}: {},\n", "pu", self.pu as u64)?;
        write!(f, "\t{}: {:.4},\n", "revenue", self.revenue)?;
        write!(f, "\t{}: {},\n", "seabeast_pu", self.seabeast_pu as u64)?;
        write!(f, "\t{}: {:.4},\n", "seabeast_revenue", self.seabeast_revenue)?;
        write!(f, "\t{}: {:.4},\n", "pu_rate", self.pu_rate * 100.0)?;
        write!(f, "\t{}: {:.4},\n", "seabeast_pu_rate", self.seabeast_pu_rate * 100.0)?;
        write!(f, "\t{}: {:.4},\n", "seabeast_pu_rate_f_au", self.seabeast_pu_rate_f_au * 100.0)?;
        write!(f, "\t{}: {:.4},\n", "arpu", self.arpu)?;
        write!(f, "\t{}: {:.4},\n", "seabeast_arpu", self.seabeast_arpu)?;
        write!(f, "\t{}: {:.4},\n", "not_seabeast_arpu", self.not_seabeast_arpu)?;
        write!(f, "\t{}: {:.4},\n", "arppu", self.arppu)?;
        write!(f, "\t{}: {:.4},\n", "seabeast_arppu", self.seabeast_arppu)?;
        write!(f, "\t{}: {:.4},\n", "not_seabeast_arppu", self.not_seabeast_arppu)?;
        write!(f, " }}\n")
    }
}

pub async fn _grouping_by_key(
    cluster_statistic: &mut Vec<ClusterStatistic>,
    clusters: &HashMap<usize, ClusterSet<TimeWrap>>,
    _project_folder: &String,
    payment_data: &HashMap<usize, PaymentUser>,
    sort_fn: fn(&ClusterStatistic) -> u64,
    add_to_path: &String,
) -> (HashMap<usize, ClusterSet<TimeWrap>>, Vec<ClusterStatistic>, HashMap<(String, usize), Vec<usize>>) {
    cluster_statistic.sort_by_key(|obj| sort_fn(obj));
    let (low_clusters, _) = cluster_statistic.iter().fold((Vec::new(), 0.0), |(acc_clusters, acc_au), right| {
        if acc_au > 0.15 {
            return (acc_clusters, acc_au);
        }
        let mut resul = acc_clusters;
        resul.push(right.cluster_id);
        (resul, acc_au + right.au_rate)
    });
    cluster_statistic.reverse();
    let (high_clusters, _) = cluster_statistic.iter().fold((Vec::new(), 0.0), |(acc_clusters, acc_au), right| {
        if acc_au > 0.15 {
            return (acc_clusters, acc_au);
        }
        let mut resul = acc_clusters;
        resul.push(right.cluster_id);
        (resul, acc_au + right.au_rate)
    });
    let middle_clusters = cluster_statistic.iter().fold(Vec::new(), |acc_clusters, right| {
        if low_clusters.contains(&right.cluster_id) || high_clusters.contains(&right.cluster_id) {
            return acc_clusters;
        }
        let mut resul = acc_clusters;
        resul.push(right.cluster_id);
        resul
    });
    let mut clusters_to_csv: HashMap<(String, usize), Vec<usize>> = HashMap::new();
    clusters_to_csv.insert((add_to_path.to_string(), 20), low_clusters.clone());
    clusters_to_csv.insert((add_to_path.to_string(), 50), middle_clusters.clone());
    clusters_to_csv.insert((add_to_path.to_string(), 80), high_clusters.clone());

    // Создаем кластеры и собираем игроков.
    let mut low_cluster: ClusterSet<TimeWrap> = ClusterSet { id: 20, points: HashMap::new(), centroid: TimeWrap(HashMap::new()), class: ClusterClass::NotClassified };

    for cluster_id in low_clusters.iter() {
        low_cluster.points.extend(clusters.get(cluster_id).unwrap().points.iter());
    }

    let mut high_cluster: ClusterSet<TimeWrap> = ClusterSet { id: 80, points: HashMap::new(), centroid: TimeWrap(HashMap::new()), class: ClusterClass::NotClassified };

    for cluster_id in high_clusters.iter() {
        high_cluster.points.extend(clusters.get(cluster_id).unwrap().points.iter());
    }

    let mut middle_cluster: ClusterSet<TimeWrap> = ClusterSet { id: 50, points: HashMap::new(), centroid: TimeWrap(HashMap::new()), class: ClusterClass::NotClassified };

    for cluster_id in middle_clusters.iter() {
        middle_cluster.points.extend(clusters.get(cluster_id).unwrap().points.iter());
    }

    let mut result_cluster_groups: HashMap<usize, ClusterSet<TimeWrap>> = HashMap::new();
    result_cluster_groups.insert(low_cluster.id, low_cluster.clone());
    result_cluster_groups.insert(middle_cluster.id, middle_cluster.clone());
    result_cluster_groups.insert(high_cluster.id, high_cluster.clone());
    

    // Подсчитываем статистику.
    //let stat_path = project_folder.to_string() + "/" + "stats_groups_" + add_to_path.as_str() + ".txt";
    //let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    //let mut data_file = BufWriter::new(data_file);
    let (stats, mut cluster_statistic) = get_stats_clusters(&result_cluster_groups, &payment_data).await;
    cluster_statistic.sort_by_key(|obj| (obj.arpu*1000.0) as u64);
    println!("GROUPING: {}", &add_to_path);
    println!("{}", &stats);
    //data_file.write_all((format!("HIGH:\n{:?}", high_clusters) + "\n").as_bytes()).await.unwrap();
    //data_file.write_all((format!("MIDDLE:\n{:?}", middle_clusters) + "\n").as_bytes()).await.unwrap();
    //data_file.write_all((format!("LOW:\n{:?}", low_clusters) + "\n").as_bytes()).await.unwrap();
    //data_file.write_all((stats + "\n").as_bytes()).await.unwrap();
    //data_file.write_all((format!("{:?}\n", cluster_statistic) + "\n").as_bytes()).await.unwrap();
    //data_file.flush().await.unwrap();
    (result_cluster_groups, cluster_statistic, clusters_to_csv)
}