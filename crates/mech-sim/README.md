[![Open In Colab](https://colab.research.google.com/assets/colab-badge.svg)](https://colab.research.google.com/github/infinityabundance/mech/blob/main/crates/mech-sim/notebooks/mech_sim_colab.ipynb)

# mech-sim

`mech-sim` is a deterministic Rust crate for reduced-order architecture validation of the paper:

`Gigawatt-Class Terrestrial Legged Vehicles: A Nuclear-Thermal, Pulse-Power, and Electrohydraulic Systems Architecture`

It exists to fill the exact gap the paper calls out: there was no reduced-order simulation layer validating whether the architecture's pulse-power, recharge, thermal, and local actuation claims stay coherent when forced into a reproducible numerical model.

This crate is deliberately not a full mech simulator. It is a serious proof-of-life simulator for the paper's minimal multi-timescale architecture.

## Why this crate exists

The paper argues that gigawatt-class terrestrial legged vehicles are constrained by an energy-power-bandwidth incompatibility:

- continuous sources such as nuclear-thermal systems can supply large average power but not the full instantaneous maneuver bandwidth,
- pulse layers can cover burst demand but must recharge on slower timescales,
- local actuation layers need even faster short-horizon buffering and distribution,
- thermal rejection ultimately determines sustained maneuver feasibility.

Those statements are compelling architecturally, but they needed a deterministic reduced-order validation layer. `mech-sim` is that layer.

## What it implements

The simulator directly encodes the reduced-order model discussed in the paper:

- `Ep(t)` pulse-layer stored energy
- `T(t)` aggregate thermal state
- `y(t)` reduced maneuver response / reduced mechanical output
- `v(t)` reduced response rate
- `Elocal_i(t)` for four limb-local energy buffers

The implemented scaffold is:

```text
dEp/dt = eta_c * Pc - Ptransfer_total - Pl(Ep, T)
Ct * dT/dt = Qg(Pdelivered, Ptransfer_total, Pl) - Qr(T)
M * y_ddot + D(T) * y_dot + K(y, T) * y = G(Ep, T) * u + d(t)
```

with a limb-local extension:

```text
dElocal_i/dt = Ptransfer_i - Pdraw_i - Plocal_loss
Pdelivered_i = min(Pdraw_i, Ptransfer_i + Elocal_i / dt)
```

The point is not high-fidelity electrohydraulic detail. The point is to make the paper's architecture simulation-ready with inspectable units, explicit parameters, deterministic behavior, and hard constraint events.

## Interpreting `y` and mechanical work

`y` is a reduced-order maneuver-response variable. It is intentionally coupled to pulse energy, thermal state, and actuator authority, but it should not be read as literal full-body mech displacement unless that interpretation is justified independently.

This means the crate is strongest as validation of:

- layered energy architecture,
- recharge behavior,
- thermal limitation,
- authority degradation,
- local-buffer collapse and recovery.

Delivered mechanical work is therefore best read as reduced-order authority-delivery evidence rather than full rigid-body maneuver realism. Small values do not undercut the architecture-level conclusions; they simply reflect the scope of the reduced model.

## Scenario library

Built-in presets:

- `burst`
  1 GW-class, 1 second burst followed by recharge and thermal decay.
- `recharge`
  3 GJ recharge interpretation case with `Pc = 50 MW` and approximately 60 second refill behavior.
- `duty-cycle`
  Repeated burst / coast / recharge pattern for duty-cycle-limited behavior.
- `hover`
  350-500 MW sustained maneuver case calibrated so the aggregate thermal state becomes the dominant limiter.
- `stress`
  High limb-force request with biased local allocation, local-buffer depletion, and gain degradation.
- `constraint-violation`
  Explicit failure case that emits low-energy, local-buffer, saturation, and related flags.

The baseline sweep expands those into reproducible parameter studies across:

- continuous power `Pc`
- burst power
- burst duration
- pulse storage reserve
- thermal rejection coefficient
- actuator demand scale
- damping / stiffness response scaling

Additional sweep presets extend that baseline conservatively:

- `thermal-duty-matrix`
  Thermal rejection versus repeated-burst cadence for heatmap-style duty-cycle interpretation.
- `limb-allocation-comparison`
  Stress-case comparison across allocation policies to expose local-buffer collapse, imbalance, and delivered/requested authority differences.

## Code structure

```text
crates/mech-sim/
├── Cargo.toml
├── README.md
├── configs/
├── docs/
├── examples/
├── notebooks/
├── src/
└── tests/
```

Important modules:

- `src/model.rs`
  The reduced-order equations, actuator demand, losses, thermal balance, gain degradation, and local buffer flow rules.
- `src/integrator.rs`
  Fixed-step deterministic simulation loop.
- `src/scenarios.rs`
  Paper-aligned scenario presets and sweep presets.
- `src/metrics.rs`
  Run-level proof-of-life metrics, interpretation-aware recharge fields, admissible-region summaries, and reduced-order Lyapunov monitor outputs.
- `src/monitor.rs`
  Admissible-region monitoring, reduced-response target proxy, figure metadata, and stability helpers.
- `src/outputs.rs`
  Timestamped output directory creation plus additive CSV/JSON writing for monitor and figure metadata.
- `src/plots.rs`
  Rust-native PNG plot generation.

Additional docs:

- [`docs/math.md`](/home/one/mech/crates/mech-sim/docs/math.md)
- [`docs/architecture.md`](/home/one/mech/crates/mech-sim/docs/architecture.md)

## CLI

Run from the workspace root:

```bash
cargo run -p mech-sim -- scenario burst
cargo run -p mech-sim -- scenario recharge --pc-mw 50 --ep-gj 3
cargo run -p mech-sim -- scenario hover
cargo run -p mech-sim -- sweep baseline
cargo run -p mech-sim -- sweep thermal-duty-matrix
cargo run -p mech-sim -- sweep limb-allocation-comparison
cargo run -p mech-sim -- config crates/mech-sim/configs/baseline.json
```

Main modes:

- `scenario <preset>`
- `sweep <preset>`
- `config <path>`

Useful flags:

- `--output-root <path>`
- `--seed <u64>`
- `--pc-mw <f64>`
- `--ep-gj <f64>`
- `--initial-ep-gj <f64>`
- `--duration-s <f64>`
- `--dt-s <f64>`
- `--thermal-rejection-mw-per-k <f64>`
- `--burst-power-mw <f64>`
- `--burst-duration-s <f64>`
- `--actuator-demand-scale <f64>`
- `--allocation-strategy <equal|front-biased|rear-biased|diagonal-bias>`
- `--local-buffer-mj <f64>`
- `--damping-scale <f64>`
- `--stiffness-scale <f64>`

Full CLI help:

```bash
cargo run -p mech-sim -- --help
```

## JSON config mode

Example scenario config:

```json
{
  "mode": "scenario",
  "preset": "burst",
  "seed": 1,
  "output_root": "output-mech-sim",
  "overrides": {
    "continuous_power_mw": 50.0,
    "pulse_energy_gj": 4.0,
    "initial_ep_gj": 4.0,
    "burst_power_mw": 1000.0,
    "burst_duration_s": 1.0
  }
}
```

Included examples:

- [`configs/baseline.json`](/home/one/mech/crates/mech-sim/configs/baseline.json)
- [`configs/hover_thermal.json`](/home/one/mech/crates/mech-sim/configs/hover_thermal.json)
- [`configs/baseline_sweep.json`](/home/one/mech/crates/mech-sim/configs/baseline_sweep.json)

## Outputs

Every single run writes:

- `time_series.csv`
- `limb_buffers.csv`
- `events.csv`
- `summary.json`
- `stability_summary.json`
- `figure_metadata.json`
- `params.json`
- `derived_metrics.csv`
- `plots/*.png`

Every sweep root writes:

- `sweep_summary.csv`
- `sweep_summary.json`
- `plots/*.png`
- nested per-case directories containing the full single-run artifact set

Additional sweep presets also emit additive exports such as:

- `thermal_duty_matrix.csv`
- `thermal_duty_matrix.json`
- `thermal_duty_heatmap.csv`
- `limb_allocation_comparison.csv`
- `limb_allocation_comparison.json`

The default output root is:

```text
output-mech-sim/
```

Each invocation creates a timestamped subdirectory using:

```text
YYYY-MM-DD_HH-MM-SS
```

Example:

```text
output-mech-sim/2026-03-17_11-21-03/
```

This directory policy is enforced so runs do not overwrite one another.

## Metrics

The summary layer computes:

- min / max / final `Ep`
- time below energy threshold
- peak `T`
- time above thermal threshold
- max actuator demand
- saturation counts
- recharge time
- delivered mechanical work
- success / failure flags
- effective duty cycle
- local limb imbalance metrics
- first-threshold timestamps
- mean delivered/requested ratio and normalized authority utilization
- reduced-response efficiency
- partial-refill interpretation fields such as `energy_depleted_j`, `recharge_fraction_of_full_reserve`, and `ideal_refill_time_s`
- admissible-region breach counts and percent time outside the admissible region
- clamp counts for `Ep`, `T`, `y`, and `ydot`
- reduced-order Lyapunov monitor fields `V`, `dV/dt`, and local stability-margin summaries

These metrics are exposed both in JSON and in flat CSV form.

The time-series output now also carries additive monitor columns for:

- reduced-response target and error,
- Lyapunov candidate `V` and numerical `dV/dt`,
- authority utilization,
- raw proposed next-state values,
- clamp flags,
- admissible-boundary proximity flags.

## Proof-of-life figures

The Rust crate generates PNG figures for:

- `Ep vs time`
- `T vs time`
- actuator requested vs delivered power
- reduced output `y(t)`
- recharge curve
- normalized burst overlays
- local limb buffer trajectories
- `Ep-T` phase portrait
- actuator draw vs `Ep`

It also emits additive paper-support PNGs when the relevant scenario or sweep is run:

- `burst_ep_vs_time.png`
- `hover_temperature_vs_time.png`
- `stress_limb_buffers.png`
- `sweep_pc_vs_recharge.png`
- `phase_portrait.png`
- `thermal_duty_heatmap.png`
- `limb_allocation_comparison.png`

The baseline sweep also generates:

- `Pc vs recharge time`
- thermal rejection vs peak temperature
- burst power vs time-to-threshold
- pulse storage vs effective duty cycle
- actuator demand scale vs saturation count

The Colab notebook regenerates the figures independently from CSV/JSON outputs and produces the PDF report / downloadable zip bundle.

The upgraded notebook adds a conservative interactive analysis layer on top of the existing crate workflow:

- a SciPy `solve_ivp` reference implementation for the reduced `(Ep, T, y, v)` state,
- Rust-vs-SciPy trajectory cross-checks using exported crate CSVs,
- `ipywidgets` sliders for `eta_c`, `thrust_area`, `T_max`, `qr_gain`, `Pc_MW`, `Ep0_GJ`, and `burst_MW`,
- three publication-ready figures generated in the notebook:
  `coupled evolution`, `recharge duty cycle`, and `authority map`,
- centerpiece figures for burst coupled evolution, thermal-limited hover behavior, and stress-case authority collapse,
- paper-support Figures 7–11 and compact summary-table artifacts,
- explicit Rust-vs-SciPy agreement artifacts and explanation blocks,
- a one-click PDF report / bundle step that pulls directly from CSV and JSON outputs.

The authority map should be read as the visual counterpart of the reduced mathematical relation `M(x, u) = G(Ep, T) u`: it shows how maneuver authority contracts across the admissible operating region as pulse energy falls or thermal state rises.

## Reproducibility

- the solver is deterministic,
- built-in presets have no hidden randomness,
- any seed-driven modulation must be enabled explicitly,
- all resolved parameters are written to `params.json`,
- all metrics are derived from the exported run history rather than hidden in-memory state.

This crate prioritizes reproducibility over model complexity.

## Limitations

What this simulator does validate:

- pulse discharge vs recharge coherence
- thermal rise and rejection trends over repeated or sustained maneuvers
- actuator demand vs pulse-layer depletion
- local limb buffer depletion and recovery
- explicit energy / thermal / local-buffer / saturation events
- parameter sensitivity across continuous power, storage, rejection, and demand

What it does not validate:

- multibody leg dynamics
- terrain interaction
- detailed hydraulic spool dynamics
- nuclear thermal primary-loop transients
- structural loads or fatigue
- closed-loop gait control

It should be cited as a reduced-order architecture-validation layer, not as a full vehicle performance simulator.

## Notebook and report workflow

The notebook lives at:

- [`notebooks/mech_sim_colab.ipynb`](/home/one/mech/crates/mech-sim/notebooks/mech_sim_colab.ipynb)

It:

- installs Rust if needed,
- clones or reuses the repository,
- rebuilds `mech-sim`,
- runs baseline scenarios and the baseline sweep,
- runs the thermal-duty and limb-allocation comparison sweeps,
- runs a notebook-side SciPy reference model for the reduced state,
- exposes interactive sensitivity sliders without requiring code edits,
- loads outputs with pandas,
- regenerates publication-style figures,
- exports the coupled evolution figure, recharge duty-cycle figure, and authority map as both PNG and PDF,
- compares notebook SciPy trajectories against Rust crate outputs when available,
- writes a PDF report,
- creates a zip archive with outputs and notebook artifacts.

Notebook-generated artifacts are written additively under the burst run's timestamped output tree in:

- `notebook_artifacts/figures/*.png`
- `notebook_artifacts/figures/*.pdf`
- `notebook_artifacts/data/interactive_reference_timeseries.csv`
- `notebook_artifacts/data/interactive_recharge_cycles.csv`
- `notebook_artifacts/data/authority_map.csv`
- `notebook_artifacts/data/authority_map_metadata.json`
- `notebook_artifacts/data/interactive_params.json`
- `notebook_artifacts/data/interactive_metrics.json`
- `notebook_artifacts/data/rust_vs_scipy_burst_comparison.csv`
- `notebook_artifacts/data/rust_vs_scipy_burst_metrics.json`
- `notebook_artifacts/data/rust_scipy_agreement.csv`
- `notebook_artifacts/data/rust_scipy_agreement.json`
- `notebook_artifacts/data/paper_summary_table.csv`
- `notebook_artifacts/data/paper_summary_table.md`
- `notebook_artifacts/data/paper_summary_table.tex`
- `notebook_artifacts/data/paper_results_table.csv`
- `notebook_artifacts/data/paper_results_table.md`
- `notebook_artifacts/data/paper_results_table.tex`
- `notebook_artifacts/data/paper_support_manifest.json`
- `notebook_artifacts/data/notebook_summary.json`
- `notebook_artifacts/reports/mech_sim_report.pdf`
- `notebook_artifacts/artifact_bundle.zip`

## Validation targets covered by the crate

The current implementation directly produces the missing proof-of-life layer for:

- a 1 GW pulse discharge vs recharge tail,
- a 3 GJ recharge interpretation under 50 MW-class continuous power,
- duty-cycle-limited burst/coast behavior,
- a sustained maneuver case where thermal limits dominate,
- local-limb buffer depletion and recovery,
- explicit machine-readable constraint breaches.

## Recharge interpretation note

The burst scenario and the dedicated recharge scenario are intentionally different.

- `burst` measures a partial refill tail after a single pulse discharge.
- `recharge` measures a much deeper refill case centered on a 3 GJ reserve at `Pc = 50 MW`.

Those times should therefore not be compared as if they were the same refill depth.

## Quick start

Baseline scenario:

```bash
cargo run -p mech-sim -- scenario burst
```

Baseline sweep:

```bash
cargo run -p mech-sim -- sweep baseline
```

Notebook:

Open [`notebooks/mech_sim_colab.ipynb`](/home/one/mech/crates/mech-sim/notebooks/mech_sim_colab.ipynb) in Colab from the badge at the top of this README.
