use sysinfo::{RefreshKind, System};
use chrono::Local;
use std::collections::HashSet;
use std::{collections::HashMap, thread, time::Duration};
use std::io::{Write, BufWriter};
use clap::Parser;

/// Command-line arguments parsed by clap.
///
/// - `out_name`  — base name for the output CSV file (written to `data/<out_name>.csv`).
/// - `pname`     — name of the target process to monitor (matched against running processes).
/// - `interval`  — polling period in seconds between successive snapshots (default: 1).
#[derive(Parser, Debug)]
#[command(author, version, about)]
struct Args {
    #[arg(short, long)]
    pub out_name: String,
    #[arg(short, long)]
    pub pname: String,

    #[arg(short, long, default_value_t = 1)]
    pub interval: u64,
}

/// Entry point for the process-memory monitoring tool.
///
/// Workflow:
/// 1. Parse CLI arguments (`out_name`, `pname`, `interval`).
/// 2. Initialise a `sysinfo::System` instance, perform a full refresh, and record
///    the number of logical CPUs available on the host.
/// 3. Create (or truncate) the output CSV file at `data/<out_name>.csv` and write
///    the header row: `timestamp;pids;memory;cpu;cpus;cpu_time;was_in_time;virtual_memory`.
/// 4. Enter an infinite polling loop that repeats every `interval` seconds:
///    a. Refresh all system process data from the OS.
///    b. Find all currently running processes whose name matches `pname`.
///       For each match, update `last_time` — a persistent map from PID to the tuple
///       `(resident_bytes, cpu_usage_%, accumulated_cpu_time_ms, virtual_bytes)` —
///       and collect the set of currently alive PIDs (`was`).
///    c. Iterate over every PID ever seen in `last_time` and append one CSV row per PID.
///       PIDs absent from `was` (i.e. the process has exited) are written with zero
///       metrics and `was_in_time = false`.
///    d. Accumulate totals across alive PIDs and print a one-line summary showing
///       resident memory (MB), virtual memory (MB), raw CPU %, and per-core CPU %.
#[tokio::main]
async fn main() {
    let args = Args::parse();

    // Convert the polling interval from seconds to a std Duration.
    let interval = Duration::from_secs(args.interval);
    // Initialise the sysinfo System object and perform a complete first-time refresh
    // to populate CPU, memory, and process data before entering the loop.
    let mut system = System::new();
    system.refresh_specifics(RefreshKind::everything());
    // Record the number of logical CPU cores; used to normalise per-core CPU usage.
    let cpus = system.cpus().len();

    println!("CPU {}...", cpus);

    // Build the output CSV file path and create (or truncate) it.
    let stat_path = "data/".to_string() + args.out_name.as_str() + ".csv";
    let data_file = std::fs::File::create(&stat_path).unwrap();
    let mut data_file = BufWriter::new(data_file);
    // Write the CSV header so consumers know the column layout.
    let result = format!("timestamp;pids;memory;cpu;cpus;cpu_time;was_in_time;virtual_memory\n");
    let _ = data_file.write_all(result.as_bytes());
    data_file.flush().unwrap();
    // Persistent store of the most recently observed stats for every PID ever seen.
    // Key: PID (u32).  Value: (resident_bytes, cpu_%, accumulated_cpu_time_ms, virtual_bytes).
    // Entries are never evicted so that a final zero-row is emitted for processes that exit.
    let mut last_time: HashMap<u32, (u64, f32, u64, u64)> = HashMap::new();
    loop {
        // Pull the latest process list and resource counters from the OS.
        system.refresh_all();
        // Collect an iterator over all processes whose name matches the target.
        let tasks = system.processes_by_name(args.pname.as_ref());
        // Capture the wall-clock time for this snapshot; used as the CSV timestamp.
        let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
        // Re-open the CSV file in append mode so rows accumulate across iterations.
        let data_file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&stat_path)
            .unwrap();
        let mut data_file = BufWriter::new(data_file);
        // Update the cached stats for every currently alive matching process and
        // build 'was' — the set of PIDs that are alive in this snapshot.
        let was: HashSet<u32> = tasks.map(|t_process| {

                last_time.insert(t_process.pid().as_u32(), (t_process.memory(), t_process.cpu_usage(), t_process.accumulated_cpu_time(), t_process.virtual_memory()));
                t_process.pid().as_u32()
            }).collect();
        // Fold over every PID ever seen: write a CSV row for each and sum totals.
        // For PIDs no longer in 'was' all metric fields are 0, was_in_time = false.
        let (memory, cpu_u, v_mem) = last_time.iter()
            .fold((0, 0.0, 0), |acc, (pid, (mem, cpu, time, vmem))| {
                let result = format!("{};{:?};{};{};{};{:?};{};{}\n",timestamp, pid, if was.contains(pid) {*mem} else {0}, if was.contains(pid) {*cpu} else {0.0}, cpus, time, was.contains(pid), if was.contains(pid) {*vmem} else {0});
                let _ = data_file.write_all(result.as_bytes());
                (acc.0 + if was.contains(pid) {*mem} else {0}, acc.1 + if was.contains(pid) {*cpu} else {0.0}, acc.2 + if was.contains(pid) {*vmem} else {0})
            });
        data_file.flush().unwrap();
        // Print aggregate totals: resident memory (MB), virtual memory (MB),
        // combined CPU %, and CPU % divided by the number of logical cores.
        println!("{}: memory = {:.2} MB; virt_memory = {:.2}; cpu={:.2}/{:.2}", timestamp, (memory as f32)/(1024.0*1024.0), (v_mem as f32)/(1024.0*1024.0), cpu_u, cpu_u/(cpus as f32));
        // Pause until the next polling interval.
        thread::sleep(interval);
    }
}
