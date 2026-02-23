use crate::{conv::ConvLayer, traits::ConfigFS};

//use serde_json::Value;
use tokio::{fs::File, io::AsyncReadExt};
use tokio::io::{AsyncWriteExt, BufWriter};


impl ConfigFS for ConvLayer {
    type OutType = Self;
    async fn read(path: &String) -> Option<Self::OutType> {
        if path.find(".json") == None {
            return None;
        }
        let mut file = File::open(path).await.unwrap();
        let mut contents = String::new();
        file.read_to_string(&mut contents).await.unwrap();
        let conv: Self::OutType = serde_json::from_str(&contents).expect("JSON was not well-formatted");
        
        Some(conv)
    }
    async fn save(&self, path: &String) {
        
        let pretty_json = serde_json::to_string_pretty(self).unwrap();

        
        let data_file = File::create(path).await.unwrap();
        let mut data_file = BufWriter::new(data_file);
        data_file.write_all(pretty_json.as_bytes()).await.unwrap();
        data_file.flush().await.unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    //#[tokio::test]
    async fn conv_read_write_test() {
        let v = vec![1,2,3,4,5,6,7,8,9,10];
        v.windows(10).for_each(|x| {dbg!(x);});
        let result = ConvLayer::new((3,224,224), (7,7), 64, (2,2), (3, 3), crate::conv::ConvMethod::FFT);
        dbg!(result.get_weight_count());
        let path = "../data/conv_1/conv_1.json".to_string();
        result.save(&path).await;
        let result = ConvLayer::read(&path).await.unwrap();
        let path = "../data/conv_1/conv_2.json".to_string();
        result.save(&path).await;
        //dbg!(result);
        assert_eq!(2, 4);
    }
}