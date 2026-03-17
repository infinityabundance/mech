# mech-sim Architecture

## Purpose

`mech-sim` is not a full robot or vehicle simulator. It is a deterministic research crate that validates whether the paper's claimed energy-storage, recharge, thermal, and local-actuation architecture behaves coherently in a reduced-order setting.

## Module map

- `src/config.rs`
  Defines scenario presets, CLI/config overrides, sweep metadata, and the serializable run schema.
- `src/model.rs`
  Implements the reduced-order energy, thermal, actuator, and local-buffer equations.
- `src/state.rs`
  Holds simulation state, per-step diagnostics, CSV row types, and event latches.
- `src/integrator.rs`
  Runs the fixed-step deterministic simulation loop and produces a `SimulationResult`.
- `src/metrics.rs`
  Reduces time series into run-level metrics and machine-readable proof-of-life summaries.
- `src/scenarios.rs`
  Encodes the built-in burst, recharge, duty-cycle, hover, stress, and constraint-violation cases, plus the baseline sweep library.
- `src/outputs.rs`
  Creates timestamped run roots and writes CSV/JSON artifacts.
- `src/plots.rs`
  Generates Rust-native PNG figures from single runs and sweeps.
- `src/sweep.rs`
  Executes multi-case sweeps and aggregates case summaries.
- `src/main.rs`
  Exposes the CLI.

## Execution flow

### Single scenario

1. Resolve a preset or JSON config into a `SimulationConfig`.
2. Create `output-mech-sim/YYYY-MM-DD_HH-MM-SS/`.
3. Run the deterministic fixed-step integrator.
4. Emit:
   - `time_series.csv`
   - `limb_buffers.csv`
   - `events.csv`
   - `summary.json`
   - `params.json`
   - `derived_metrics.csv`
   - `plots/*.png`

### Sweep

1. Build the sweep case list from the baseline preset.
2. Create a single timestamped run root.
3. Create nested case directories under `sweeps/baseline/<case-id>/`.
4. Run each case and emit the full single-run artifact set in its own directory.
5. Emit top-level aggregate sweep files:
   - `sweep_summary.csv`
   - `sweep_summary.json`
   - `plots/*.png`

## Output layout

Single scenario:

```text
output-mech-sim/2026-03-17_11-20-15/
├── time_series.csv
├── limb_buffers.csv
├── events.csv
├── summary.json
├── params.json
├── derived_metrics.csv
└── plots/
```

Sweep:

```text
output-mech-sim/2026-03-17_11-21-03/
├── sweep_summary.csv
├── sweep_summary.json
├── plots/
└── sweeps/
    └── baseline/
        ├── recharge_pc_50mw/
        ├── burst_power_1000mw/
        └── ...
```

## Determinism

- No hidden randomness is used in the built-in presets.
- A `seed` is still part of the run schema so deterministic command wobble or deterministic disturbance injection can be enabled explicitly.
- All outputs are fully reproducible for the same config and seed.

## Colab notebook role

The Rust crate is the authoritative simulator and artifact producer.

The Colab notebook in `notebooks/mech_sim_colab.ipynb` is the analysis/report layer:

- installs Rust if needed,
- rebuilds the crate from scratch,
- reruns baseline scenarios and a sweep,
- reloads CSV/JSON outputs with pandas,
- regenerates figures,
- writes a PDF report,
- zips the artifacts for download.
