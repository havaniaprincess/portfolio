# MMR — Match-Making Rating Pipeline

A Rust workspace for recalculating player MMR (Match-Making Rating) from raw userstat
datasets. Two independent algorithm variants are provided (v1 and v2), each implemented
as a standalone binary that streams session data and maintains a persistent leaderboard
snapshot on disk.

## Workspace structure

```
mmr/
├── mmr-libs/          # Shared library: types, algorithms, I/O helpers
├── leaderboard-v1-8/  # Binary: ELO-based recalculation, dataset slice 8
├── leaderboard-v2-8/  # Binary: Pool redistribution recalculation, dataset slice 8
└── Cargo.toml         # Workspace manifest
```

## Crates

### [`mmr-libs`](mmr-libs/README.md)
Shared library consumed by both pipeline binaries. Provides:
- Core data types (`Leaderboard`, `LeaderboardV2`, `LeaderboardRow`, `MMRType`)
- **v1 algorithm** — ELO-based with a 6-battle calibration phase (`leaderboard_v1`)
- **v2 algorithm** — Session-wide MMR pool redistribution with sigmoid terms (`leaderboard_v2`)
- Math helpers: power curves, sigmoid, weighted average (`math`)
- Per-session statistics and win-rate counters (`statistic`)
- Distribution analytics: MMR spread, battle spread, country spread (`spread`)
- Dataset loaders: session modes, player registrations, faction assignments (`datasets`)
- Async file writers, line-level parsers and session memory buffer

### [`leaderboard-v1-8`](leaderboard-v1-8/README.md)
Pipeline binary using the v1 ELO algorithm on dataset slice 8.  
Accepts all input paths via CLI arguments (`--data`, `--user-team`, `--session-mode`,
`--user-faction`, `--leaderboard`).

### [`leaderboard-v2-8`](leaderboard-v2-8/README.md)
Pipeline binary using the v2 pool algorithm on dataset slice 8.  
Input paths are hardcoded to `data/` subdirectories; reads from pre-split cluster files
(`data/clusters/<id>.json`).

## Algorithm overview

### v1 — ELO-based calibration

| Battle count | MMR update |
|---|---|
| 1 – 5 | Provisional: running average of accumulated battle score |
| 6 | Bootstrap: nearest-neighbor estimate from `battle_score_hash` |
| 7+ | Classic ELO delta via `diff_mmr` |

`diff_mmr` components:
1. Score curve: $\frac{50}{e^{10^6 / score^2}}$
2. Matchup pressure: sigmoid activated when MMR gap > 250 (±35 pts)
3. Situational modifiers: early quit −20, top-20 percent +20

### v2 — Pool redistribution

All players in a session share a single MMR pool. Each player's net change is:

$$\Delta MMR = inc\_mmr - dec\_mmr$$

where:
- **Confidence coefficient** $k = (\sqrt{2})^{\min(0,\,battles-6)}$
- **Decrease** scales with `dec_k * mmr / k` (higher MMR players contribute more to the pool)
- **Increase** scales with `inc_k * pool_share / k` (weighted by adjusted battle score)
- **Bank terms** (`bank_give`, `bank_get`) redistribute additional MMR via sigmoid curves based on the player's absolute rating

## Shared processing pattern

Both pipelines follow the same concurrent architecture:

```
 Main thread (streaming loop)
      │  session rows (flume channels)
      ├─► statistic_aggregate  →  data/statistic_v*_8
      ├─► statistic_check      →  (in-memory win-rate buckets)
      ├─► write_change         →  data/changes/0, data/changes/1
      └─► session_class_aggreg →  data/leaderboard_v*/session_classification_8
```

Sessions with fewer than 5 players per team or in the `newbie_common` mode are skipped.

## Output files

| Path | Contents |
|---|---|
| `data/leaderboard_v*/base` | Final leaderboard snapshot (one `LeaderboardRow` per line) |
| `data/leaderboard_v*/battle_faction` | Per-user faction battle counters |
| `data/statistic_v*_8` | Aggregated board statistics |
| `data/changes/0`, `data/changes/1` | Per-user MMR change records by classifier |
| `data/leaderboard_v*/session_classification_8` | Per-session team composition flags |
| `data/csv/<id>.csv` | Debug CSV dump of processed session rows |

## Build

```bash
cargo build --release
```

## Run

**v1:**
```bash
cargo run -p leaderboard-v1-8 --release -- \
  --user-team     data/user_team_8.json     \
  --session-mode data/session_mode_8.json \
  --user-faction  data/user_faction.json    \
  --data          data/userstat_8.json      \
  --leaderboard   data/leaderboard_v1/base
```

**v2:**
```bash
cargo run -p leaderboard-v2-8 --release
```

## Dependencies

- [`tokio`](https://crates.io/crates/tokio) — async runtime (multi-thread)
- [`flume`](https://crates.io/crates/flume) — lock-free multi-producer multi-consumer channels
- [`clap`](https://crates.io/crates/clap) — CLI argument parsing (v1 pipeline)

Requires Rust 2021 edition or later.
