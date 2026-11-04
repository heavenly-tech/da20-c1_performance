# DA20-C1 Eclipse Performance Planner

**Disclaimer:** This application is **not** an official Diamond Aircraft or DGAC-approved source. It is **not** approved for operational flight planning. Digitized values are approximate when present; during early development the app uses **`SYNTHETIC`** fixture data only. The approved Aircraft Flight Manual (AFM) aboard the aircraft always governs.

Performance planning helper for the **Diamond DA20-C1 Eclipse**, implementing AFM **DA202-C1 Chapter 5** architecture:

- Takeoff (Figure 5.4)
- Climb (Figures 5.5 / 5.6 / 5.8 via Figure 5.2 equivalent altitude)
- Cruise (Table 3)
- Landing (Table 4)
- Wind components and corrections (Figure 5.3)

## Stack

| Layer | Technology |
|-------|------------|
| Core | Rust (`da20-core`) — interpolation, altitudes, performance lookups |
| UI | Leptos → WASM (`da20-ui`) — numeric + hand-plot modes |
| Server | Single binary (`da20-performance`) — Axum, embedded `dist/` assets |
| Container | Multi-stage Alpine Dockerfile (optional `scratch` target) |

Intended host: `da20.heavenly.cl` behind **nginx-proxy** (`VIRTUAL_HOST`), no bundled nginx.

## Data files

| Path | Purpose |
|------|---------|
| `data/schema/performance-data.schema.json` | Documented JSON schema with citation fields |
| `data/fixtures/SYNTHETIC_performance_data.json` | **SYNTHETIC — NOT FOR FLIGHT USE** (tests & dev) |
| `data/TODO_performance_data.json` | **Placeholder** for production digitized AFM data |

Official Diamond AFM/AMM download URLs will be wired when provided (`TODO` in meta).

## SCTB defaults

- Field elevation 2130 ft, QNH 1013 hPa, OAT 20 °C  
- Runway 19, wind 200° / 8 kt  
- Weight 1764 lb, cruise 6500 ft PA @ 2600 RPM  

## Modes

1. **Numeric entry** — direct inputs, core calculations in WASM  
2. **Hand plot** — vector-redrawn charts (not AFM scans) with draggable crosshair and guide lines  

Both modes share `da20-core`. Saved readings: unlimited entries in **localStorage**, list / compare / delete, export **CSV** or **JSON**.

## Build & run locally

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk --locked
cd crates/da20-ui && trunk build --release
cd ../..
cargo run -p da20-server
# open http://127.0.0.1:8080
```

Docker:

```bash
docker build -t da20-performance .
docker run --rm -e VIRTUAL_HOST=da20.heavenly.cl -p 8080:8080 da20-performance
```

## How it was validated

| Stage | Status |
|-------|--------|
| Unit tests (interpolation, altitudes, wind components) | Implemented — `cargo test -p da20-core` |
| Integration against **SYNTHETIC** fixture | Implemented — smoke paths only |
| POH worked-example regression | **Harness ready** — `crates/da20-core/tests/poh_harness.rs` awaits `validation.worked_examples` + tolerances in production JSON |
| Cross-check vs approved AFM | **Blocked** — digitized chart data not yet available |

### Screenshot placeholders

<!-- TODO: add planner numeric mode screenshot -->
<!-- TODO: add hand-plot mode screenshot -->
<!-- TODO: add provenance / methods manual screenshot -->

## Provenance in the UI

Every result shows AFM document number, revision, date, and per-output figure/table citation (from the data file). A built-in **methods manual** documents formulas, interpolation, clamping, and limitations.

## License

MIT — see repository license. **Do not commit AFM/AMM PDFs or chart scans.**
