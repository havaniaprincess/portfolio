pub fn reader(line: &String) -> std::collections::BTreeMap<String, String> {
    let hash_line: std::collections::BTreeMap<String, String> = line.replace("{", "").replace("}", "").replace("\"", "").split(",").filter_map(|item: &str| {
        let splited: Vec<String> = item.split(":").map(|it| it.to_string()).collect();
        if splited.len() == 2 {
            Some((splited[0].clone(), splited[1].clone()))
        } else {
            None
        }
    }).collect();

    hash_line
}