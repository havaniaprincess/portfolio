use std::time::UNIX_EPOCH;
use std::io::{Write, BufWriter};

use clap::Parser;
use rdev::{listen, Button, Event, Key};

/// Command-line arguments accepted by the application.
///
/// `--test` / `-t` specifies the session/test name that is appended to the
/// output CSV file name: `data/source/<test>_control.csv`.
// CLI arguments for selecting output file suffix/test name.
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub test: String,
}


/// Debug callback for printing raw keyboard and mouse events to stdout.
///
/// Prints the Unix timestamp in milliseconds alongside the event type and key/button.
/// Currently unused in production — kept for local debugging purposes.
// Debug callback (currently unused) that prints incoming keyboard/mouse events.
fn _callback(event: Event) {
    //let datetime: DateTime<Utc> = event.time.into();
    //let date_formated = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
    let unix_time = event.time.duration_since(UNIX_EPOCH).unwrap().as_millis();
    match event.event_type {
        rdev::EventType::KeyPress(key) => {
            println!("[{:?}]Received event: {:?}", unix_time.to_string(), key);
        },
        rdev::EventType::ButtonPress(key) => {
            println!("[{:?}]Received event: {:?}", unix_time.to_string(), key);
        },
        _ => {}
    };
}

/// Appends a single control event to the semicolon-delimited CSV log file.
///
/// Opens the file in append mode (creating it if it does not exist), writes one
/// row in the format `timestamp;key`, and flushes the buffer immediately so that
/// events are not lost if the process is killed.
///
/// # Arguments
/// * `file`      – Absolute or relative path to the target CSV file.
/// * `key`       – String label for the key or button event (e.g. `"1"`, `"left"`).
/// * `timestamp` – Unix timestamp of the event in milliseconds.
// Appends one control event line to CSV in format: timestamp;key
fn add_key(file: &String, key: &str, timestamp: u128) {
        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file)
            .unwrap();
        let mut data_file = BufWriter::new(data_file);
        let result = format!("{};{}\n", timestamp, &key);
        let _ = data_file.write_all(result.as_bytes());
        data_file.flush().unwrap();

}

/// Application entry point.
///
/// 1. Parses the `--test` CLI argument to determine the output file name.
/// 2. Creates `data/source/<test>_control.csv` with a `timestamp;key` header.
/// 3. Installs a global input listener that captures selected keyboard keys
///    (`0`–`3`, `Backspace`, `Space`, `F12`) and mouse button releases
///    (`Left`, `Right`, `Middle`), writing each event as a timestamped CSV row.
///
/// The listener blocks the main thread until an error occurs or the process is
/// terminated. Any listener error is printed to stdout.
fn main() {
    // Parse CLI arguments.
    let args = Args::parse();

    // Build destination CSV path for this run.
    let result_data = "data/source/".to_string();
    let stat_path = result_data.to_string() + args.test.as_str() + "_control.csv";

    // Create output file and write CSV header.
    let data_file = std::fs::File::create(&stat_path).unwrap();
    let mut data_file = BufWriter::new(data_file);
    let result = format!("timestamp;key\n");
    let _ = data_file.write_all(result.as_bytes());
    data_file.flush().unwrap(); 

    // Global input listener callback.
    // Captures selected keyboard and mouse events and writes them to CSV.
    let clback = move |event: Event| {
        //let datetime: DateTime<Utc> = event.time.into();
        //let date_formated = datetime.format("%Y-%m-%d %H:%M:%S").to_string();
        //dbg!(&args);
        let unix_time = event.time.duration_since(UNIX_EPOCH).unwrap().as_millis();
        match event.event_type {
            rdev::EventType::KeyPress(key) => {
                // Store only specific keys used by control logic.
                match key {
                    Key::Num1 => add_key(&stat_path, "1", unix_time),
                    Key::Num2 => add_key(&stat_path, "2", unix_time),
                    Key::Num3 => add_key(&stat_path, "3", unix_time),
                    Key::Num0 => add_key(&stat_path, "0", unix_time),
                    Key::Backspace => add_key(&stat_path, "backspace", unix_time),
                    Key::Space => add_key(&stat_path, "space", unix_time),
                    Key::F12 => add_key(&stat_path, "f12", unix_time),

                    _ => {}
                }
                //println!("[{:?}]Received event: {:?}", unix_time.to_string(), key);
            },
            rdev::EventType::ButtonRelease(key) => {
                // Store mouse button releases as control events.
                match key {
                    Button::Left => add_key(&stat_path, "left", unix_time),
                    Button::Right => add_key(&stat_path, "right", unix_time),
                    Button::Middle => add_key(&stat_path, "middle", unix_time),
                    _ => {}
                }
                //println!("[{:?}]Received event: {:?}", unix_time.to_string(), key);
            },
            _ => {}
        };
    };

    // Start global input listening loop.
    if let Err(error) = listen(clback) {
        println!("Error with input listener: {:?}", error);
    }
}