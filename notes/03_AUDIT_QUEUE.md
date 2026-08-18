# Ordered audit queue — on-chain contracts + codebases

**Mode:** Sequential deep (finish each before next).  
**Workspace git:** tip advances per program close-out.  
**Queue status:** **ALL COMPLETE** (2026-08-18).

| # | Status | Program | Program ID | Codebase / artifact |
|---|--------|---------|------------|---------------------|
| 1 | **COMPLETE** | Token Swap V1 | `DjVE6JNiYqPL2QXyCUUh8rNjHrbz9hXHNYt99MQ59qw1` | dump + SPL lineage |
| 2 | **COMPLETE** | Token Swap V2 | `9W959DqEETiGZocYWCQPaJ6sBmUzgfxXfqGeTEdp3aQP` | dump + SPL lineage |
| 3 | **PHASE-COMPLETE** | Aquafarm | `82yxjeMsvaURa4MbZZ7WZZHfobirZYkH1zF8fmeGtyaQ` | dump + SDKs |
| 4 | **PHASE-COMPLETE** @ e5f089b | Whirlpools | `whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc` | math fuzz 1.31M/0 |
| 5 | **PHASE-COMPLETE** | Wavebreak | `waveQX2yP3H1pVU8djGvEHmYg8uamQ84AuyGtpsrXTF` | client math fuzz 805k/0 |
| 6 | **PHASE-COMPLETE** | xORCA | `StaKE6XNKVVhG8Qu9hDJBqCW3eRe7MDGLz17nJZetLT` | math fuzz 765k/0 |
| 7 | **PHASE-COMPLETE** | Whirlpools Immutable | `iwhrLHdsgrvmnwU8GF2FSmyabSMjfHwFGJAX2ufJ3ZN` | dump; ≈ e5f089b; auth burned |

Per-program DoD: identity → source map → manual review → full fuzz → `reports/0N_*_AUDIT.md` → git commit.
