use std::collections::HashMap;
use std::fmt::Debug;
use std::{fs, path::Path};
use std::{fs::OpenOptions, io::{Write, BufWriter}};


// Checks whether directory/path exists.
// If `need_create` is true, creates missing directories.
pub fn dir_exists<P: AsRef<Path>>(path: P, need_create: bool) -> Result<(), String> {
    if !path.as_ref().exists() {
        if need_create {
            let _ = fs::create_dir_all(&path);
            return Ok(());
        }
        return Err("Dir is not found".to_string());
    } else {
        return Ok(());
    }
}

// Returns file names (not full paths) for direct children of the directory.
pub fn list_files<P: AsRef<Path>>(path: P) -> Result<Vec<String>, String> {
    let files: Vec<String> = fs::read_dir(path).unwrap().filter_map(|entry|{
        let entry = entry.unwrap();
        let path = entry.path();
        //dbg!(&path.is_file());
        if path.is_file() {
            //dbg!(&path.is_file());
            return Some(path.file_name().unwrap().to_str().unwrap().to_string());
        }
        None
    }).collect::<Vec<String>>();
    Ok(files)
}

// Ensures CSV file exists; creates it and writes header when missing.
pub fn cvs_file_exists<P: AsRef<Path>>(path: P, header: &String) -> Result<(), String>
where 
    P: Debug
{
        if !path.as_ref().exists() {
            let data_file = std::fs::File::create(&path).unwrap();
            let mut data_file = BufWriter::new(data_file);
            let _ = data_file.write_all(header.as_bytes());
            data_file.flush().unwrap();
        }
        Ok(())
}

    // Appends one CSV row/string to existing file.
pub fn cvs_file_adding<P: AsRef<Path>>(path: P, str: &String) -> Result<(), String>
where 
    P: Debug
{
        if path.as_ref().exists() {
            let data_file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&path)
                .unwrap();
            let mut data_file = BufWriter::new(data_file);
            let _ = data_file.write_all(str.as_bytes());
            data_file.flush().unwrap();
        }
        Ok(())
}