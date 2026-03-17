pub mod config;
pub mod errors;
pub mod integrator;
pub mod metrics;
pub mod model;
pub mod outputs;
pub mod plots;
pub mod scenarios;
pub mod state;
pub mod sweep;

use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;

use crate::config::{
    OutputLayout, ResolvedRunConfig, RunConfig, ScenarioOverrides, ScenarioPreset, SweepPreset,
};
use crate::integrator::simulate;
use crate::outputs::{prepare_run_root, write_run_outputs, write_sweep_outputs};
use crate::scenarios::{build_scenario_config, build_sweep_cases};
use crate::sweep::run_sweep;

pub fn run_scenario_preset(
    preset: ScenarioPreset,
    overrides: ScenarioOverrides,
    output_root: impl AsRef<Path>,
    seed: u64,
) -> Result<PathBuf> {
    let output_root = output_root.as_ref();
    let run_root = prepare_run_root(output_root)?;
    let config = build_scenario_config(preset, overrides, seed)?;
    let result = simulate(config)?;
    write_run_outputs(&run_root, &result)?;
    Ok(run_root)
}

pub fn run_sweep_preset(
    preset: SweepPreset,
    overrides: ScenarioOverrides,
    output_root: impl AsRef<Path>,
    seed: u64,
) -> Result<PathBuf> {
    let output_root = output_root.as_ref();
    let run_root = prepare_run_root(output_root)?;
    let cases = build_sweep_cases(preset, overrides, seed)?;
    let aggregate = run_sweep(preset, cases, &run_root)?;
    write_sweep_outputs(&run_root, &aggregate)?;
    Ok(run_root)
}

pub fn run_config_file(path: impl AsRef<Path>) -> Result<PathBuf> {
    run_config_file_with_overrides(path, None, None)
}

pub fn run_config_file_with_overrides(
    path: impl AsRef<Path>,
    output_root_override: Option<&Path>,
    seed_override: Option<u64>,
) -> Result<PathBuf> {
    let path = path.as_ref();
    let raw = fs::read_to_string(path)?;
    let mut run_config: RunConfig = serde_json::from_str(&raw)?;
    apply_cli_overrides(&mut run_config, output_root_override, seed_override);
    match ResolvedRunConfig::from_run_config(run_config, path.parent())? {
        ResolvedRunConfig::Scenario {
            config,
            output_layout,
        } => {
            let run_root = prepare_run_root(output_layout.output_root())?;
            let result = simulate(config)?;
            write_run_outputs(&run_root, &result)?;
            Ok(run_root)
        }
        ResolvedRunConfig::Sweep {
            preset,
            cases,
            output_layout,
        } => {
            let run_root = prepare_run_root(output_layout.output_root())?;
            let aggregate = run_sweep(preset, cases, &run_root)?;
            write_sweep_outputs(&run_root, &aggregate)?;
            Ok(run_root)
        }
    }
}

pub fn default_output_layout() -> OutputLayout {
    OutputLayout::default()
}

fn apply_cli_overrides(
    run_config: &mut RunConfig,
    output_root_override: Option<&Path>,
    seed_override: Option<u64>,
) {
    match run_config {
        RunConfig::Scenario {
            seed,
            output_root,
            ..
        }
        | RunConfig::Sweep {
            seed,
            output_root,
            ..
        } => {
            if let Some(value) = output_root_override {
                *output_root = Some(value.to_path_buf());
            }
            if let Some(value) = seed_override {
                *seed = Some(value);
            }
        }
    }
}
