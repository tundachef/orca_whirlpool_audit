# Ordered audit queue — on-chain contracts + codebases

**Mode:** Sequential deep (finish each before next).  
**Workspace git:** tip advances per program close-out.

| # | Status | Program | Program ID | Codebase / artifact |
|---|--------|---------|------------|---------------------|
| 1 | **COMPLETE** | Token Swap V1 | `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1` | `audit_work/dumps/orca_v1.so` + SPL token-swap-v2.0.0 lineage |
| 2 | **COMPLETE** | Token Swap V2 | `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP` | `orca_v2.so` + SPL lineage |
| 3 | **PHASE-COMPLETE** | Aquafarm | `82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ` | dump + typescript-sdk / aquafarm-sdk |
| 4 | **PHASE-COMPLETE** @ e5f089b | Whirlpools | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | `sources/whirlpools` @ `e5f089b`; math fuzz 1.31M/0 crashes |
| 5 | **IN PROGRESS** | Wavebreak | `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF` | dump + `orca_wavebreak` client (`fuzz/wavebreak/`) |
| 6 | pending | xORCA | `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT` | `sources/xorca` |
| 7 | pending | Whirlpools Immutable | `iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN` | diff vs #4; auth burned |

Per-program DoD: identity → source map → manual review → full fuzz → `reports/0N_*_AUDIT.md` → git commit.
