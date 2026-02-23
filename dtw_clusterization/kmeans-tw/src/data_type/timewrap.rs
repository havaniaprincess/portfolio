use std::{collections::{BTreeMap, HashMap}, fmt::Debug};

use serde_json::Value;



#[derive(Clone, Debug, PartialEq)]
pub struct TimeWrap(pub HashMap<usize, f64>);

impl TimeWrap {
    pub fn from_json_bq(row: &Value, x_axis_name: &String, y_axis_name: &String) -> Self {
        //dbg!(row);
        let mut result = HashMap::new();
        result.insert(
            match &row[x_axis_name] {
                Value::String(data) => data.parse::<usize>().unwrap(),
                _ => panic!()
              }, 
              match &row[y_axis_name] {
                Value::Number(data) => data.as_f64().unwrap(),
                Value::String(data) => data.parse::<f64>().unwrap(),
                _ => panic!()
              }
        );
        TimeWrap(result)
    }
    pub fn add_hour(&mut self, row: &Value, x_axis_name: &String, y_axis_name: &String){
        self.0.insert(
            match &row[x_axis_name] {
                Value::String(data) => data.parse::<usize>().unwrap(),
                _ => panic!()
              }, 
              match &row[y_axis_name] {
                Value::Number(data) => data.as_f64().unwrap(),
                Value::String(data) => data.parse::<f64>().unwrap(),
                _ => panic!()
              }
        );
    }
    pub fn to_zero(&self) -> Self {
      Self(self.0.iter().map(|(id, _)| (*id, 0.0)).collect())
    }
    pub fn to_btree(&self) -> BTreeMap<usize, f64> {
      self.0.clone().into_iter().map(|obj| obj).collect()
    }
    pub fn to_sort_vec(&self) -> Vec<f64> {
      self.to_btree().into_iter().map(|obj| obj.1).collect()
    }
}


#[derive(Clone, Debug, PartialEq)]
pub struct PaymentUser(pub f64, pub bool);

impl PaymentUser {
    pub fn from_json_bq(row: &Value) -> Self {
        let revenue = match &row["revenue"] {
                Value::Number(data) => data.as_f64().unwrap(),
                Value::String(data) => data.parse::<f64>().unwrap(),
                _ => 0.0
              };
        let sea_beast = match &row["player_type"] {
                Value::Bool(data) => *data,
                Value::String(data) => data.parse::<bool>().unwrap(),
                _ => false
              };
        PaymentUser(revenue, sea_beast)
    }
}


pub fn hash_map_to_btree(hm: &HashMap<usize, f64>) -> BTreeMap<usize, f64> {
  hm.clone().into_iter().map(|obj| obj).collect()
}