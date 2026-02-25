# Portfolio

A collection of Rust projects spanning data analysis, game-backend tooling, transit simulation, and computer vision.

---

## Projects

### [dtw_clusterization](dtw_clusterization/README.md)
**Time-series clustering library using Dynamic Time Warping**

K-Means clustering optimised for time-series data, with DTW (Dynamic Time Warping) as the distance metric. Supports windowed DTW, Euclidean distance, K-Means++ initialisation, DTW Barycenter Averaging (DBA) for centroid calculation, and quality-based recursive refinement with outlier detection. Includes a CLI tool with CSV and BigQuery export, and business-metric calculation (ARPU, ARPPU).

**Stack:** Rust · Rayon · K-Means++ · DTW / DBA

---

### [mmr](mmr/README.md)
**Match-Making Rating recalculation pipeline**

Streaming pipeline that recalculates player MMR from raw userstat session data and writes a persistent leaderboard snapshot to disk. Two algorithm variants:

- **v1** — ELO-based with a 6-battle calibration phase (provisional → bootstrap → classic ELO delta).
- **v2** — Session-wide MMR pool redistribution with sigmoid confidence scaling.

Both binaries follow a concurrent architecture driven by `flume` channels. Outputs per-user change records, aggregated statistics, session classification flags, and debug CSV dumps.

**Stack:** Rust · async I/O · ELO · sigmoid-weighted pool redistribution

---

### [nimby_timetable](nimby_timetable/README.md)
**Week-long transit timetable and train-roster scheduler**

Given line definitions and a fleet of rolling stock, generates second-accurate departure schedules for a full 7-day week, builds driver shifts, assigns every shift to a specific train, and manages a four-tier maintenance countdown (A / B / C / D services). Configuration and output use the RON (Rusty Object Notation) format.

**Stack:** Rust · RON · discrete-event simulation

---

### [text_reader](text_reader/README.md)
**OCR-based fishing-game screenshot analyser**

Automated pipeline that captures in-game catch data (fish name, mass, length, EXP breakdown) from screenshots using Tesseract OCR, cross-validates results against a local statistics database, and produces normalised CSV reports. Includes an image preprocessing library (grayscale, Lanczos resampling, region isolation), a keyboard/mouse event logger for device-preset tracking, LSTM fine-tuning tooling, and catch-statistics aggregation.

**Stack:** Rust · Tesseract OCR · PNG processing · LSTM fine-tuning

---

### [proc_testing](proc_testing/README.md)
**Process-level CPU and memory monitor**

Continuously polls one or more running processes by name and records per-PID resource usage to a semicolon-delimited CSV file. Each snapshot captures resident and virtual memory, instantaneous CPU %, accumulated CPU time, and an alive/exited flag — so processes that die mid-run are still preserved in the history with zeroed metrics. The polling interval and output file name are configurable via CLI flags.

**Stack:** Rust · sysinfo · clap · chrono

---

## Build

Each project is an independent Cargo workspace. Build any of them with:

```bash
cd <project_dir>
cargo build --release
```
