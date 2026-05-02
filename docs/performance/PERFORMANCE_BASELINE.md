# Performance Baseline

Captured: 2026-05-01 for the FeOxDB-only `1.0.0` cleanup

Historical note: these numbers describe the last recorded baseline. Rerun the ignored backend and frontend performance baselines before treating them as release evidence for a new candidate.

## Commands

```powershell
Push-Location backend
cargo test --test performance_baseline -- --ignored --nocapture
Pop-Location

$env:RUN_PERF_BASELINE = "1"
$env:PERF_BASELINE_JSON = ".tmp\performance-baseline-frontend.json"
node scripts\run-bun.mjs run test -- frontend/tests/performance-baseline.test.tsx
```

Generated JSON output stays under `.tmp/` and is not committed.

## Backend

The backend baseline measures FeOxDB `load_entries`, a `query_inventory` equivalent that loads entries then counts, filters, sorts, and pages, and in-memory query filtering/sorting after entries are already loaded.

| Dataset | Entries | `load_entries` median / p95 | Query equivalent median / p95 | In-memory query median / p95 |
| --- | ---: | ---: | ---: | ---: |
| Current synthetic FeOx fixture | 500 | rerun required | rerun required | rerun required |
| Synthetic 1k | 1,000 | 13.196 ms / 16.490 ms | 17.017 ms / 18.295 ms | 3.637 ms / 4.193 ms |
| Synthetic 10k | 10,000 | 141.043 ms / 144.067 ms | 216.612 ms / 250.413 ms | 55.854 ms / 63.877 ms |

The backend harness no longer imports any legacy database fixture. Rerun the ignored benchmark before publishing fresh numbers for `1.0.0`.

## Frontend

The frontend baseline measures local search/filter/sort, virtualized table initial render, scroll update handling, and process heap after each dataset.

| Dataset | Entries | Local filter/sort median / p95 | Table render median / p95 | Scroll update median / p95 | Heap after dataset |
| --- | ---: | ---: | ---: | ---: | ---: |
| Current mock data | 14 | 0.009 ms / 0.031 ms | 9.815 ms / 33.745 ms | 0.029 ms / 0.121 ms | 107.697 MB |
| Synthetic 1k | 1,000 | 0.632 ms / 1.122 ms | 15.123 ms / 22.100 ms | 5.118 ms / 6.674 ms | 99.692 MB |
| Synthetic 10k | 10,000 | 3.953 ms / 6.564 ms | 7.172 ms / 10.752 ms | 4.946 ms / 5.964 ms | 111.599 MB |

## Decision

The 10k local-load baseline is below the 500 ms threshold, and 10k search/filter/sort is below the 100 ms threshold in both frontend local filtering and backend in-memory query filtering. Server-backed paged table queries, cached normalized search text, and app-managed FeOxDB secondary query indexes are deferred in this cleanup branch.

Keep `queryInventory` available and keep the benchmark harnesses so the indexed path can be added later if real inventory data or slower machines cross the thresholds.
