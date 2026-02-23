//! Filesystem helpers for the `ts_learning` training pipeline.
//!
//! Provides directory validation, filtered file listing, and a diagnostic
//! reader for Tesseract `.lstmf` binary training-sample files.

use std::{fmt::Debug, fs::{self, File}, io::BufReader, path::Path};
use std::io::{Read};
use byteorder::{LittleEndian, ReadBytesExt};

/// Checks whether a directory (or any path) exists.
///
/// When `need_create` is `true` and the path is absent, all missing directory
/// components are created recursively via `fs::create_dir_all`.
///
/// # Arguments
/// * `path`        – path to check.
/// * `need_create` – if `true`, create the directory tree when missing.
///
/// # Returns
/// `Ok(())` on success, or `Err(String)` when the path does not exist and
/// `need_create` is `false`.
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

/// Lists file stems of all direct file children in a directory,
/// optionally filtered by extension.
///
/// Only immediate files are returned — subdirectories are skipped.
/// When `ext` is `Some(e)`, only files whose extension matches `e`
/// (case-sensitive) are included. The returned strings are file stems
/// (file name without the extension).
///
/// # Arguments
/// * `in_path` – directory to list.
/// * `ext`     – optional file extension filter (e.g. `Some("lstmf".to_string())`).
///
/// # Returns
/// `Ok(Vec<String>)` of matching file stems in unspecified order.
pub fn list_files<P: AsRef<Path>>(in_path: P, ext: Option<String>) -> Result<Vec<String>, String> 
where 
    P: Debug
{
    dbg!(&in_path);
    let files: Vec<String> = fs::read_dir(in_path).unwrap().filter_map(|entry|{
        let entry = entry.unwrap();
        let path = entry.path();
        //dbg!(&path.is_file());
        if path.is_file() {
            if let Some(ex) = &ext {
                if let Some(fex) = path.extension() {
                    if ex.to_string() == fex.to_str().unwrap().to_string() {
                        return Some(path.file_stem().unwrap().to_str().unwrap().to_string());
                    }
                }
                return None;
            }
            //dbg!(&path.is_file());
            return Some(path.file_stem().unwrap().to_str().unwrap().to_string());
        }
        None
    }).collect::<Vec<String>>();
    Ok(files)
}

/// Lists file stems **and extensions** of all direct file children in a
/// directory, optionally filtered by extension.
///
/// Behaves identically to [`list_files`] except each entry is a
/// `(stem, extension)` tuple instead of a bare stem. Useful when downstream
/// code needs to reconstruct the full file name or branch on the extension.
///
/// # Arguments
/// * `in_path` – directory to list.
/// * `ext`     – optional file extension filter.
///
/// # Returns
/// `Ok(Vec<(stem, extension)>)` of matching files in unspecified order.
pub fn list_files_name<P: AsRef<Path>>(in_path: P, ext: Option<String>) -> Result<Vec<(String, String)>, String> 
where 
    P: Debug
{
    dbg!(&in_path);
    let files: Vec<(String, String)> = fs::read_dir(in_path).unwrap().filter_map(|entry|{
        let entry = entry.unwrap();
        let path = entry.path();
        //dbg!(&path.is_file());
        if path.is_file() {
            if let Some(ex) = &ext {
                if let Some(fex) = path.extension() {
                    if ex.to_string() == fex.to_str().unwrap().to_string() {
                        return Some((path.file_stem().unwrap().to_str().unwrap().to_string(), path.extension().unwrap().to_str().unwrap().to_string()));
                    }
                }
                return None;
            }
            //dbg!(&path.is_file());
            return Some((path.file_stem().unwrap().to_str().unwrap().to_string(), path.extension().unwrap().to_str().unwrap().to_string()));
        }
        None
    }).collect::<Vec<(String, String)>>();
    Ok(files)
}


/// Reads and prints diagnostic information from a Tesseract `.lstmf` file.
///
/// `.lstmf` files are binary training samples used by the Tesseract LSTM
/// trainer. Each file encodes:
/// 1. A 4-byte little-endian length prefix followed by the source image path.
/// 2. A 4-byte little-endian length prefix followed by the ground-truth text.
///
/// This function reads both fields and prints them to stdout, which is useful
/// for verifying that a generated `.lstmf` file contains the expected content
/// before starting a training run.
///
/// # Arguments
/// * `path` – path to the `.lstmf` file to inspect.
///
/// # Panics
/// Panics if the file cannot be opened or if the binary structure is
/// malformed (unexpected EOF or invalid UTF-8).
pub fn check_lstmf(path: &Path) {
    let file = File::open(path).unwrap();
    let mut reader = BufReader::new(file);

    // 1. Read the byte length of the image path string.
    let img_path_len = reader.read_u32::<LittleEndian>().unwrap() as usize;

    // 2. Read the image path bytes and decode as UTF-8.
    let mut img_path_buf = vec![0; img_path_len];
    reader.read_exact(&mut img_path_buf).unwrap();
    let img_path = String::from_utf8_lossy(&img_path_buf);

    // 3. Read the byte length of the ground-truth text string.
    let txt_len = reader.read_u32::<LittleEndian>().unwrap() as usize;

    // 4. Read the ground-truth text bytes and decode as UTF-8.
    let mut text_buf = vec![0; txt_len];
    reader.read_exact(&mut text_buf).unwrap();
    let text = String::from_utf8_lossy(&text_buf);

    println!("Image path: {}", img_path);
    println!("Text: {}", text);
    println!("Length (in bytes): {}", txt_len);

}