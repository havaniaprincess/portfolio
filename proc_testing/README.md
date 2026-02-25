# proc_testing

A Rust workspace for process-level resource monitoring. The primary tool — `pid_memory` — continuously polls one or more running processes by name and records their CPU and memory usage to a CSV file.

---

## Workspace layout

```
proc_testing/
├── Cargo.toml          # Workspace manifest (members: pid_memory)
├── src/
│   └── lib.rs          # Root crate (integration test stubs)
└── pid_memory/
    ├── Cargo.toml
    └── src/
        └── main.rs     # CLI binary
```

---

## pid_memory

### What it does

1. Resolves every running process whose name matches the value supplied via `--pname`.
2. On each tick it records per-PID stats (resident memory, virtual memory, CPU %, accumulated CPU time) into `data/<out_name>.csv`.
3. Processes that have exited since the previous tick are still written to the file with all metric fields set to `0` and `was_in_time = false`, preserving a complete history.
4. Prints a one-line summary to **stdout** after every snapshot.

### CSV output format

The file is semicolon-delimited with the following columns:

| Column | Type | Description |
|---|---|---|
| `timestamp` | `YYYY-MM-DD HH:MM:SS` | Wall-clock time of the snapshot |
| `pids` | `u32` | Process ID |
| `memory` | `u64` (bytes) | Resident set size; `0` if the process has exited |
| `cpu` | `f32` (%) | CPU usage at the time of the snapshot; `0` if exited |
| `cpus` | `usize` | Total logical CPU cores on the host |
| `cpu_time` | `u64` (ms) | Accumulated CPU time since process start |
| `was_in_time` | `bool` | `true` if the process was alive during this snapshot |
| `virtual_memory` | `u64` (bytes) | Virtual address space size; `0` if exited |

### Prerequisites

- Rust toolchain ≥ 1.85 (edition 2024)
- A `data/` directory must exist in the working directory before launching

```bash
mkdir data
```

### Build

```bash
# from the workspace root
cargo build --release -p pid_memory
```

### Run

```bash
cargo run --release -p pid_memory -- \
  --out-name <output_file_base> \
  --pname <process_name> \
  [--interval <seconds>]
```

| Flag | Short | Default | Description |
|---|---|---|---|
| `--out-name` | `-o` | *(required)* | Base name for the CSV file (`data/<out-name>.csv`) |
| `--pname` | `-p` | *(required)* | Process name to monitor (substring match against running processes) |
| `--interval` | `-i` | `1` | Polling interval in seconds |

#### Example

```bash
# Monitor all "firefox" processes, write to data/firefox_run.csv, poll every 2 s
cargo run --release -p pid_memory -- -o firefox_run -p firefox -i 2
```

Console output:

```
CPU 16...
2026-02-25 14:01:00: memory = 512.34 MB; virt_memory = 2048.00; cpu=3.20/0.20
2026-02-25 14:01:02: memory = 515.10 MB; virt_memory = 2050.12; cpu=2.95/0.18
```

---

## Dependencies

| Crate | Version | Purpose |
|---|---|---|
| `sysinfo` | 0.36 | Cross-platform process and system info |
| `chrono` | 0.4 | Timestamp formatting |
| `clap` | 4.5 | CLI argument parsing |
| `tokio` | 1.x | Async runtime |

---
