use gcp_bigquery_client::{model::{dataset::Dataset, query_request::QueryRequest, table::Table, table_data_insert_all_request::TableDataInsertAllRequest, table_schema::TableSchema, time_partitioning::TimePartitioning}, Client};



#[derive(Clone)]
pub struct BQPreContext{
    pub project_id: String,
    pub dataset_id: String,
    pub key_path: String
}
pub struct BQContext{
    pub project_id: String,
    pub dataset_id: String,
    pub table_id: String,
    pub key_path: String,
    pub schema: TableSchema,
    pub partition_field: Option<String>,
}

pub async fn send_data(
    context: &BQContext,
    data: TableDataInsertAllRequest,
    timestamp: i64,
) {
    let project_id = context.project_id.as_str();
    let dataset_id = context.dataset_id.as_str();
    let table_id = context.table_id.as_str();

    let client = Client::from_service_account_key_file(context.key_path.as_str()).await.unwrap();

    if client.dataset().get(project_id, dataset_id).await.is_err() {
        let dataset = Dataset::new(project_id, dataset_id);
        client.dataset().create(dataset).await.unwrap();
        println!("Create dataset: {}", dataset_id);
    }

    let schema = &context.schema;

    if client.table().get(project_id, dataset_id, table_id, None).await.is_err() {
        let table = match context.partition_field.clone() {
            Some(field) => {
                let time_partitioning = TimePartitioning::new("DAY".to_string()).field(field.as_str());
                Table::new(project_id, dataset_id, table_id, schema.clone()).time_partitioning(time_partitioning)
            },
            None => {
                Table::new(project_id, dataset_id, table_id, schema.clone())
            }
        };
        match client.table().create(table).await{
            Ok(_rr) => {
                //dbg!(rr);
            },
            Err(ee) => {
                dbg!(ee);
            }
        };
        println!("Table created: {}", table_id);
    }
    let query = format!(
        "DELETE FROM `{}.{}.{}` WHERE {}",
        project_id, dataset_id, table_id, format!("date(time) = date(timestamp_seconds({}))", timestamp)
    );

    match client.job().query(project_id, QueryRequest::new(&query)).await {
        Ok(_) => println!("Rows deleted successfully."),
        Err(e) => eprintln!("Error deleting rows: {:?}", e),
    }
    
    match client.tabledata().insert_all(project_id, dataset_id, table_id, data).await {
        Ok(_rr) => {
            println!("Data inserted successfully into {}.{}.{}", project_id, dataset_id, table_id);
        },
        Err(ee) => {
            dbg!(ee);
        }
    };
    println!("Data successfully loaded into the table!");
}