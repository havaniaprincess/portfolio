# mmr-libs

Shared library crate for the MMR (Match-Making Rating) recalculation pipelines.  
Provides all data structures, algorithms, I/O helpers and analytics utilities used by
the `leaderboard-v1-*` and `leaderboard-v2-*` binaries.

## Module overview

| Module | Description |
|---|---|
| `types` | Core data types: `Leaderboard`, `LeaderboardV2`, `LeaderboardRow`, `MMRType`, change structs, team descriptors |
| `leaderboard_v1` | v1 ELO-based leaderboard — calibration model, `make_session`, `set_changes` |
| `leaderboard_v2` | v2 pool-based leaderboard — sigmoid redistribution, `proc_session`, `set_change` |
| `leaderboard_row` | Serialization / deserialization of `LeaderboardRow` (flat key:value format) |
| `math` | Pure math helpers: power curves, sigmoid, `avg_3`, `diff_mmr` (v1 delta formula) |
| `statistic` | `Statistic` struct and `proc_statistic` — per-session win-rate and disbalance counters |
| `spread` | Distribution analytics: `mmr_spread`, `battle_spread`, `country_spread` |
| `datasets` | Auxiliary dataset loaders: `SessionMode`, `Registrations`, `UserFaction` |
| `memory` | `SessionMemory` — in-memory session row buffer; `read_lines` file helper |
| `userstat` | `UserBattleRow` parser — converts raw JSON-like lines into typed structs |
| `reader` | Lightweight key:value line parser used across multiple modules |
| `writer` | Async file writers for MMR change records (`write_change`, `write_change_v2`) |

## Key types

### `MMRType`
Three-variant enum representing a player's rating state:
- `MMR(u32)` — fully calibrated (6+ battles)
- `NotEnought(u32)` — provisional value (1–5 battles)
- `None` — player has never appeared in a processed session

### `Leaderboard` / `LeaderboardV2`
In-memory leaderboard state with:
- `users: HashMap<u64, LeaderboardRow>` — current ratings
- `battle_score_hash: BTreeMap<(avg_score, user_id), mmr>` — index for bootstrap estimates (v1)
- `battle_faction_hash: HashMap<(user_id, faction), battles>` — per-faction battle counters
- `sets: Vec<LeaderboardChangeV*>` — pending change buffer

### `LeaderboardRow`
Per-user aggregate stored across sessions:
`user_id`, `mmr`, `battles`, `victories`, `early_quites`, `top_20`, `battle_score`, `last_session`

## Algorithms

### v1 — ELO-based calibration (`leaderboard_v1`)
- Battles 1–5: provisional MMR = running average of battle score
- Battle 6: bootstrap MMR from nearest neighbors in `battle_score_hash` via `get_mmr_for_new`
- Battles 7+: classic delta update via `math::diff_mmr`

`diff_mmr` delta = score curve + matchup pressure sigmoid (activated when MMR gap > 250) + situational modifiers (early quit −20, top-20 +20)

### v2 — Pool redistribution (`leaderboard_v2`)
All players in a session contribute to a shared MMR pool. Each player's gain/loss is determined by:
- **Confidence coefficient** $k = (\sqrt{2})^{\min(0,\,battles-6)}$ — scales newer players' impact
- **Decrease coefficient** — sigmoid on player MMR vs. session average
- **Increase coefficient** — sigmoid on player MMR vs. session average (inverted)
- **Bank terms** — `bank_give` and `bank_get` sigmoid redistribution from high-MMR players to the pool

### Distribution analytics (`spread`)
Three functions for offline analysis:
- `mmr_spread` — player count and total MMR per (faction, mmr_bucket)
- `battle_spread` — player count per (faction, battles_bucket)
- `country_spread` — player count per (faction, country, mmr_bucket)

Faction classification: a player is `faction_1` / `faction_2` when ≥ 65 % of their battles were played in that faction, otherwise `mixed`.

## Persistence format

### Leaderboard base (`data/leaderboard_v*/base`)
One `LeaderboardRow` per line in flat key:value format:
```
user_id:123,mmr:1500,battles:42,victories:20,early_quites:1,top_20:5,battle_score:84000,last_session:1700000000
```

### Faction counters (`data/leaderboard_v*/battle_faction`)
```
user_id:123,faction:faction_1,battles:30
```

## Dependencies

- [`tokio`](https://crates.io/crates/tokio) — async file I/O
- [`flume`](https://crates.io/crates/flume) — multi-producer multi-consumer channels for inter-task communication

## Build

```bash
cargo build --release
```

This crate is a library (`lib`) and is not meant to be run directly.  
It is consumed by `leaderboard-v1-8` and `leaderboard-v2-8` via a local path dependency:

```toml
mmr_libs = { path = "../mmr-libs" }
```
