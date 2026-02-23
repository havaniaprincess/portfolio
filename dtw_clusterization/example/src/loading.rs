use std::{collections::HashMap, io::Read};

use serde_json::Value;
use kmeans_tw::data_type::timewrap::TimeWrap;

pub async fn load_data(path: &String, x_axis_name: &String, y_axis_name: &String, id_field: &String) -> Option<HashMap<usize, TimeWrap>> {
    if path.find(".json") == None {
      return None;
    }
    dbg!(&path);
    let mut file = std::fs::File::open(path).unwrap();
    let mut contents = String::new();
    file.read_to_string(&mut contents).unwrap();
    let v: Value = serde_json::from_str(&contents).unwrap(); 
 
    let mut tables: HashMap<usize, TimeWrap> = HashMap::new();
    (
        match &v {
          Value::Array(data) => data.clone(),
          _ => Vec::new()
        }
    ).iter().for_each(|item: &Value| {
        let id = match &item[id_field] {
            Value::String(data) => data.parse::<usize>().unwrap(),
            _ => 0
        };

        let user = tables.get_mut(&id);

        if user == None {
            tables.insert(id, TimeWrap::from_json_bq(item, x_axis_name, y_axis_name));
            return;
        } 
        let user = user.unwrap();

        user.add_hour(item, x_axis_name, y_axis_name);
    });
    Some(tables)
}