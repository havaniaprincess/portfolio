# leaderboard-v2-8

MMR leaderboard recalculation pipeline — version 2, dataset slice 8.

## Overview

This binary reads a userstat cluster dataset (one JSON-like row per user per session) and
recalculates MMR values for every player using the v2 algorithm (`LeaderboardV2::proc_session`).
Processing is fully streaming: rows are grouped on the fly by `session_id` and each completed
session is processed without loading the entire dataset into memory.

Unlike the v1 pipeline, all input paths are hardcoded to dataset slice 8 and input is read
from pre-split cluster files (`data/clusters/<cl_id>.json`).

Four background Tokio workers run concurrently with the main streaming loop:

| Worker | Purpose |
|---|---|
| `statistic_aggregate` | Merges per-session `Statistic` payloads into a single board map |
| `statistic_check` | Accumulates empirical win-rate counters bucketed by MMR delta (step 200) |
| `write_change` | Streams per-user MMR change records to `data/changes/` |
| `session_class_aggreg` | Persists team-composition classification flags to `data/leaderboard_v2/session_classification_8` |

All inter-task communication uses lock-free [flume](https://crates.io/crates/flume) channels.

## Input files

| Flag / Path | Contents |
|---|---|
| `--user-team` | Per-user team/victory metadata. Each line: `{"user_id":1,"session_id":2,"team":1,"victory":true}` |
| `--session-mode` | Session-to-mode mapping. Each line: `{"session_id":2,"mode":"ranked"}` |
| `--user-faction` | User faction mapping. Each line: `{"user_id":1,"faction":"newbie"}` |
| `data/clusters/<cl_id>.json` | Main userstat cluster files, sorted by `session_id` (hardcoded) |

## Output files

| Path | Contents |
|---|---|
| `data/statistic_v2_8` | Aggregated board statistics (one entry per logical board key) |
| `data/changes/0`, `data/changes/1` | Per-user MMR change records split by classifier id |
| `data/leaderboard_v2/session_classification_8` | Per-session team composition flags |
| `data/csv/<cl_id>.csv` | Debug CSV dump of every processed row |
| Session memory & leaderboard snapshot | Written via `SessionMemory::write` and `LeaderboardV2::write` |


## Usage

```bash
cargo run --release -- \
  --user-team     <path/to/user_team>     \
  --session-mode  <path/to/session_mode>  \
  --user-faction  <path/to/user_faction>  \
  --data          <path/to/userstat>      \
  --leaderboard   <path/to/leaderboard>
```

### Arguments

| Flag | Description |
|---|---|
| `--user-team` | File with `(user_id, session_id) -> (team, victory)` mappings. Each line is a JSON-like object: `{"user_id":1,"session_id":2,"team":1,"victory":true}` |
| `--session-mode` | File mapping `session_id` to session mode: `{"session_id":2,"mode":"ranked"}` |
| `--user-faction` | File mapping `user_id` to faction: `{"user_id":1,"faction":"newbie"}` |
| `--data` | Main userstat dataset — one row per user per session, sorted by `session_id` |
| `--leaderboard` | Path to an existing leaderboard snapshot used as the initial state |

## Key differences from v1

| Aspect | v1 | v2 |
|---|---|---|
| CLI arguments | Yes (via `clap`) | No — paths are hardcoded |
| Session processing | `Leaderboard::make_session` | `LeaderboardV2::proc_session` |
| Input source | Single flat file (`--data`) | Cluster files (`data/clusters/<id>.json`) |
| Leaderboard type | `Leaderboard` | `LeaderboardV2` |

## Dependencies

- [`tokio`](https://crates.io/crates/tokio) — async runtime (multi-thread)
- [`flume`](https://crates.io/crates/flume) — multi-producer multi-consumer channels
- [`clap`](https://crates.io/crates/clap) — CLI argument parsing (available, not used for paths)
- [`mmr_libs`](../mmr-libs) — shared types, math, dataset helpers and writer utilities

## Build

```bash
cargo build --release
```

Requires Rust 2021 edition or later.
