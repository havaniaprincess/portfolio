use std::collections::HashMap;

use gcp_bigquery_client::model::{field_type::FieldType, table_data_insert_all_request::TableDataInsertAllRequest, table_field_schema::TableFieldSchema, table_schema::TableSchema};
use kmeans_tw::data_type::{timewrap::TimeWrap, types::{ClusterClass, ClusterSet}};
use serde_json::{json, Map, Number, Value};
use tokio::io::{AsyncWriteExt, BufWriter};

use crate::{bq::{send_data, BQContext, BQPreContext}, stats::ClusterStatistic};


pub async fn write_clusters_base(
    clusters: &HashMap<usize, ClusterSet<TimeWrap>>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64,
) {

    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
            table_id: table.to_string(),
            key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
            schema: TableSchema::new(vec![
                TableFieldSchema::new("time", FieldType::Timestamp),
                TableFieldSchema::new("id", FieldType::Integer),
                TableFieldSchema::new("x", FieldType::Integer),
                TableFieldSchema::new("y", FieldType::Float)
            ]),
            partition_field: Some("time".to_string())
        })
    } else {
        None
    };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();

    for (cluster_id, cluster) in clusters.iter() {
        for cluster_point in cluster.centroid.0.iter() {
            let _ = data.add_row(None, json!({"time": time, "id": cluster_id, "x": cluster_point.0, "y": cluster_point.1}));
        }
    }
    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }

    let mut result = format!("id;x;y\n");
    for (_cluster_id, cluster) in clusters.iter() {
        result = format!("{}{}", result, cluster.to_csv());
    }
    let stat_path = project_folder.to_string() + "/" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}

pub async fn _write_clusters_info(
    clusters: &HashMap<usize, ClusterSet<TimeWrap>>,
    clusters_statistic: &Vec<ClusterStatistic>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64,
) {
    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
        table_id: table.to_string(),
        key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
        schema: TableSchema::new(vec![
            TableFieldSchema::new("time", FieldType::Timestamp),
            TableFieldSchema::new("id", FieldType::Integer),
            TableFieldSchema::new("score", FieldType::Float),
            TableFieldSchema::new("sigma", FieldType::Float),
            TableFieldSchema::new("class", FieldType::String),
            TableFieldSchema::new("au", FieldType::Integer),
            TableFieldSchema::new("au_rate", FieldType::Float),
            TableFieldSchema::new("pu", FieldType::Integer),
            TableFieldSchema::new("revenue", FieldType::Float),
            TableFieldSchema::new("seabeast_pu", FieldType::Integer),
            TableFieldSchema::new("seabeast_revenue", FieldType::Float),
            TableFieldSchema::new("pu_rate", FieldType::Float),
            TableFieldSchema::new("seabeast_pu_rate", FieldType::Float),
            TableFieldSchema::new("seabeast_pu_rate_f_au", FieldType::Float),
            TableFieldSchema::new("arpu", FieldType::Float),
            TableFieldSchema::new("seabeast_arpu", FieldType::Float),
            TableFieldSchema::new("not_seabeast_arpu", FieldType::Float),
            TableFieldSchema::new("arppu", FieldType::Float),
            TableFieldSchema::new("seabeast_arppu", FieldType::Float),
            TableFieldSchema::new("not_seabeast_arppu", FieldType::Float)
        ]),
        partition_field: Some("time".to_string())
    })} else {
        None
    };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();
    for cluster_stat in clusters_statistic.iter() {
        let cluster = clusters.get(&cluster_stat.cluster_id).unwrap();
        let (score, sigma, class) = match cluster.class {
            ClusterClass::Good(score, sigma) => (Some(score), Some(sigma), "good".to_string()),
            ClusterClass::Reclusterization(score, sigma) => (Some(score), Some(sigma), "reclust".to_string()),
            ClusterClass::Outline(score, sigma) => (Some(score), Some(sigma), "outline".to_string()),
            _ => (None, None, "not_class".to_string()),
        };
        let mut maps: Map<String, Value> = Map::new();
        maps.insert("time".to_string(), Value::Number(Number::from(time)));
        maps.insert("id".to_string(), Value::Number(Number::from(cluster_stat.cluster_id)));
        match score {
            Some(s) => {
                maps.insert("score".to_string(), Value::Number(Number::from_f64(s).unwrap()));
            },
            None => {}
        };
        match sigma {
            Some(s) => {
                maps.insert("sigma".to_string(), Value::Number(Number::from_f64(s).unwrap()));
            },
            None => {}
        };
        //dbg!(&cluster_stat);
        maps.insert("class".to_string(), Value::String(class));
        maps.insert("au".to_string(), Value::Number(Number::from_f64(cluster_stat.au).unwrap()));
        maps.insert("au_rate".to_string(), Value::Number(Number::from_f64(cluster_stat.au_rate).unwrap()));
        maps.insert("pu".to_string(), Value::Number(Number::from_f64(cluster_stat.pu).unwrap()));
        maps.insert("revenue".to_string(), Value::Number(Number::from_f64(cluster_stat.revenue).unwrap()));
        maps.insert("seabeast_pu".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_pu).unwrap()));
        maps.insert("seabeast_revenue".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_revenue).unwrap()));
        maps.insert("pu_rate".to_string(), Value::Number(Number::from_f64(cluster_stat.pu_rate).unwrap()));
        if !cluster_stat.seabeast_pu_rate.is_nan() {
            maps.insert("seabeast_pu_rate".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_pu_rate).unwrap()));
        }
        maps.insert("seabeast_pu_rate_f_au".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_pu_rate_f_au).unwrap()));
        maps.insert("arpu".to_string(), Value::Number(Number::from_f64(cluster_stat.arpu).unwrap()));
        maps.insert("seabeast_arpu".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_arpu).unwrap()));
        maps.insert("not_seabeast_arpu".to_string(), Value::Number(Number::from_f64(cluster_stat.not_seabeast_arpu).unwrap()));
        if !cluster_stat.arppu.is_nan() {
            maps.insert("arppu".to_string(), Value::Number(Number::from_f64(cluster_stat.arppu).unwrap()));
        }
        if !cluster_stat.not_seabeast_arppu.is_nan() {
            maps.insert("not_seabeast_arppu".to_string(), Value::Number(Number::from_f64(cluster_stat.not_seabeast_arppu).unwrap()));
        }
        if !cluster_stat.seabeast_arppu.is_nan() {
            maps.insert("seabeast_arppu".to_string(), Value::Number(Number::from_f64(cluster_stat.seabeast_arppu).unwrap()));
        }

        let json = Value::Object(maps);
        let _ = data.add_row(None, json);
    }
    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }
    let mut result = format!("id;score;sigma;class;au;au_rate;pu;revenue;seabeast_pu;seabeast_revenue;pu_rate;seabeast_pu_rate;seabeast_pu_rate_f_au;arpu;seabeast_arpu;not_seabeast_arpu;arppu;seabeast_arppu;not_seabeast_arppu\n");
    for cluster_stat in clusters_statistic.iter() {
        let cluster = clusters.get(&cluster_stat.cluster_id).unwrap();
        result = format!("{}{};{};{}\n", result, cluster_stat.cluster_id, cluster.to_csv_info(), cluster_stat.to_csv_info());
    }
    let stat_path = project_folder.to_string() + "/" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}

pub async fn write_assigned(
    assigned: &HashMap<usize, usize>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64,
) {

    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
            table_id: table.to_string(),
            key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
            schema: TableSchema::new(vec![
            TableFieldSchema::new("time", FieldType::Timestamp),
            TableFieldSchema::new("id", FieldType::Integer),
            TableFieldSchema::new("cluster_id", FieldType::Integer),
        ]),
        partition_field: Some("time".to_string())
    })} else {
        None
    };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();

    for (external_id, cluster_id) in assigned.iter() {
        let _ = data.add_row(None, json!({"time": time, "id": *external_id, "cluster_id": *cluster_id}));
    }

    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }
    let mut result = format!("id;cluster_id\n");
    for (ext_id, cluster_id) in assigned.iter() {
        result = format!("{}{};{}\n", result, ext_id, cluster_id);
    }
    let stat_path = project_folder.to_string() + "/" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}

pub async fn _write_pair_map(
    assigned: &HashMap<(usize, usize), usize>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64,
) {

    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
            table_id: table.to_string(),
            key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
        schema: TableSchema::new(vec![
            TableFieldSchema::new("time", FieldType::Timestamp),
            TableFieldSchema::new("hour_id", FieldType::Integer),
            TableFieldSchema::new("day_id", FieldType::Integer),
            TableFieldSchema::new("cluster_pair_id", FieldType::Integer),
        ]),
        partition_field: Some("time".to_string())
    })} else {
        None
    };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();

    for (id, cluster_id) in assigned.iter() {
        let _ = data.add_row(None, json!({"time": time, "hour_id": id.0, "day_id": id.1, "cluster_pair_id": *cluster_id}));
    }

    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }
    let mut result = format!("hour_id;day_id;cluster_pair_id\n");
    for ((hour_id, day_id), cluster_id) in assigned.iter() {
        result = format!("{}{};{};{}\n", result, hour_id, day_id, cluster_id);
    }
    let stat_path = project_folder.to_string() + "/" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}

pub async fn _write_group_map(
    clusters: &HashMap<(String, usize), Vec<usize>>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64,
) {

    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
            table_id: table.to_string(),
            key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
            schema: TableSchema::new(vec![
                TableFieldSchema::new("time", FieldType::Timestamp),
                TableFieldSchema::new("cluster_group_id", FieldType::String),
                TableFieldSchema::new("cluster_pair_id", FieldType::Integer),
            ]),
            partition_field: Some("time".to_string())
        })} else {
            None
        };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();

    for (id, cluster_id) in clusters.iter() {
        let _ = data.add_row(None, json!({"time": time, "cluster_group_id": id.0.to_string() + "_" + id.1.to_string().as_str(), "cluster_pair_id": *cluster_id}));
    }

    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }
    let mut result = format!("cluster_group_id;cluster_pair_id\n");
    for ((cluster_grouping, cluster_group_id), clusters_pairs) in clusters.iter() {
        for pair_id in clusters_pairs.iter() {
            result = format!("{}{};{}\n", result, cluster_grouping.to_string() + "_" + cluster_group_id.to_string().as_str(), pair_id);
        }
    }
    let stat_path = project_folder.to_string() + "/" + "group_map_" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}

pub async fn _write_clusters_base_group(
    clusters: &HashMap<String, Vec<ClusterStatistic>>,
    bq_pre_context: Option<BQPreContext>,
    project_folder: &String,
    table: &String,
    time: i64
) {
    let bq_context = 
    if bq_pre_context.is_some() {
        Some(BQContext {
            project_id: bq_pre_context.as_ref().unwrap().project_id.clone(),
            dataset_id: bq_pre_context.as_ref().unwrap().dataset_id.clone(),
            table_id: table.to_string(),
            key_path: bq_pre_context.as_ref().unwrap().key_path.clone(), // TODO NEED REMOVE TO ARGS
            schema: TableSchema::new(vec![
            TableFieldSchema::new("time", FieldType::Timestamp),
            TableFieldSchema::new("id", FieldType::String),
            TableFieldSchema::new("score", FieldType::Float),
            TableFieldSchema::new("sigma", FieldType::Float),
            TableFieldSchema::new("class", FieldType::String),
            TableFieldSchema::new("au", FieldType::Integer),
            TableFieldSchema::new("au_rate", FieldType::Float),
            TableFieldSchema::new("pu", FieldType::Integer),
            TableFieldSchema::new("revenue", FieldType::Float),
            TableFieldSchema::new("seabeast_pu", FieldType::Integer),
            TableFieldSchema::new("seabeast_revenue", FieldType::Float),
            TableFieldSchema::new("pu_rate", FieldType::Float),
            TableFieldSchema::new("seabeast_pu_rate", FieldType::Float),
            TableFieldSchema::new("seabeast_pu_rate_f_au", FieldType::Float),
            TableFieldSchema::new("arpu", FieldType::Float),
            TableFieldSchema::new("seabeast_arpu", FieldType::Float),
            TableFieldSchema::new("not_seabeast_arpu", FieldType::Float),
            TableFieldSchema::new("arppu", FieldType::Float),
            TableFieldSchema::new("seabeast_arppu", FieldType::Float),
            TableFieldSchema::new("not_seabeast_arppu", FieldType::Float)
        ]),
        partition_field: Some("time".to_string())
    })} else {
        None
    };
    let mut data: TableDataInsertAllRequest = TableDataInsertAllRequest::new();
    for (cluster_group, cluster_stat) in clusters.iter() {
        for stat in cluster_stat.iter() {
            let mut maps: Map<String, Value> = Map::new();
            maps.insert("time".to_string(), Value::Number(Number::from(time)));
            maps.insert("id".to_string(), Value::String(cluster_group.to_string() + "_" + stat.cluster_id.to_string().as_str()));
            maps.insert("au".to_string(), Value::Number(Number::from_f64(stat.au).unwrap()));
            maps.insert("au_rate".to_string(), Value::Number(Number::from_f64(stat.au_rate).unwrap()));
            maps.insert("pu".to_string(), Value::Number(Number::from_f64(stat.pu).unwrap()));
            maps.insert("revenue".to_string(), Value::Number(Number::from_f64(stat.revenue).unwrap()));
            maps.insert("seabeast_pu".to_string(), Value::Number(Number::from_f64(stat.seabeast_pu).unwrap()));
            maps.insert("seabeast_revenue".to_string(), Value::Number(Number::from_f64(stat.seabeast_revenue).unwrap()));
            maps.insert("pu_rate".to_string(), Value::Number(Number::from_f64(stat.pu_rate).unwrap()));
            maps.insert("seabeast_pu_rate".to_string(), Value::Number(Number::from_f64(stat.seabeast_pu_rate).unwrap()));
            maps.insert("seabeast_pu_rate_f_au".to_string(), Value::Number(Number::from_f64(stat.seabeast_pu_rate_f_au).unwrap()));
            maps.insert("arpu".to_string(), Value::Number(Number::from_f64(stat.arpu).unwrap()));
            maps.insert("seabeast_arpu".to_string(), Value::Number(Number::from_f64(stat.seabeast_arpu).unwrap()));
            maps.insert("not_seabeast_arpu".to_string(), Value::Number(Number::from_f64(stat.not_seabeast_arpu).unwrap()));
            maps.insert("arppu".to_string(), Value::Number(Number::from_f64(stat.arppu).unwrap()));
            maps.insert("seabeast_arppu".to_string(), Value::Number(Number::from_f64(stat.seabeast_arppu).unwrap()));
            maps.insert("not_seabeast_arppu".to_string(), Value::Number(Number::from_f64(stat.not_seabeast_arppu).unwrap()));

            let json = Value::Object(maps);
            let _ = data.add_row(None, json);
        }
    }
    
    if bq_context.is_some() {
        send_data(&bq_context.unwrap(), data, time).await;
    }
    let mut result = format!("id;au;au_rate;pu;revenue;seabeast_pu;seabeast_revenue;pu_rate;seabeast_pu_rate;seabeast_pu_rate_f_au;arpu;seabeast_arpu;not_seabeast_arpu;arppu;seabeast_arppu;not_seabeast_arppu\n");
    for (cluster_group, cluster) in clusters.iter() {
        for cl_stat in cluster.iter() {
            result = format!("{}{};{}\n", result, cluster_group.to_string() + "_" + cl_stat.cluster_id.to_string().as_str(), cl_stat.to_csv_info());
        }
    }
    let stat_path = project_folder.to_string() + "/" + table.as_str() + ".csv";
    let data_file = tokio::fs::File::create(stat_path).await.unwrap();
    let mut data_file = BufWriter::new(data_file);
    data_file.write_all(result.as_bytes()).await.unwrap();
    data_file.flush().await.unwrap();
}