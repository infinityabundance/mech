use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use clap::ValueEnum;
use serde::{Deserialize, Serialize};

use crate::model::ModelParameters;

pub const DEFAULT_OUTPUT_ROOT: &str = "output-mech-sim";
pub const LIMB_COUNT: usize = 4;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum ScenarioPreset {
    Burst,
    Recharge,
    DutyCycle,
    Hover,
    Stress,
    ConstraintViolation,
}

impl ScenarioPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Burst => "burst",
            Self::Recharge => "recharge",
            Self::DutyCycle => "duty-cycle",
            Self::Hover => "hover",
            Self::Stress => "stress",
            Self::ConstraintViolation => "constraint-violation",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum SweepPreset {
    Baseline,
    ThermalDutyMatrix,
    LimbAllocationComparison,
}

impl SweepPreset {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Baseline => "baseline",
            Self::ThermalDutyMatrix => "thermal-duty-matrix",
            Self::LimbAllocationComparison => "limb-allocation-comparison",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, ValueEnum)]
#[serde(rename_all = "kebab-case")]
pub enum AllocationStrategy {
    Equal,
    FrontBiased,
    RearBiased,
    DiagonalBias,
}

impl AllocationStrategy {
    pub fn normalized_weights(self) -> [f64; LIMB_COUNT] {
        let raw = match self {
            Self::Equal => [1.0, 1.0, 1.0, 1.0],
            Self::FrontBiased => [1.35, 1.35, 0.65, 0.65],
            Self::RearBiased => [0.65, 0.65, 1.35, 1.35],
            Self::DiagonalBias => [1.30, 0.70, 0.70, 1.30],
        };
        let total = raw.iter().sum::<f64>();
        raw.map(|value| value / total)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum IntegratorKind {
    SemiImplicitEuler,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SolverConfig {
    pub dt_s: f64,
    pub duration_s: f64,
    pub integrator: IntegratorKind,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioSegment {
    pub label: String,
    pub start_s: f64,
    pub end_s: f64,
    pub demand_fraction: f64,
    pub disturbance_n: f64,
    pub allocation_strategy: Option<AllocationStrategy>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioProfile {
    pub preset: ScenarioPreset,
    pub name: String,
    pub description: String,
    pub idle_command: f64,
    pub baseline_allocation: AllocationStrategy,
    pub seeded_command_wobble: f64,
    pub seeded_disturbance_n: f64,
    pub segments: Vec<ScenarioSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationConfig {
    pub name: String,
    pub description: String,
    pub seed: u64,
    pub solver: SolverConfig,
    pub model: ModelParameters,
    pub scenario: ScenarioProfile,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ScenarioOverrides {
    pub continuous_power_mw: Option<f64>,
    pub pulse_energy_gj: Option<f64>,
    pub initial_ep_gj: Option<f64>,
    pub duration_s: Option<f64>,
    pub dt_s: Option<f64>,
    pub thermal_rejection_mw_per_k: Option<f64>,
    pub burst_power_mw: Option<f64>,
    pub burst_duration_s: Option<f64>,
    pub actuator_demand_scale: Option<f64>,
    pub allocation_strategy: Option<AllocationStrategy>,
    pub local_buffer_energy_mj: Option<f64>,
    pub damping_scale: Option<f64>,
    pub stiffness_scale: Option<f64>,
    pub seeded_command_wobble: Option<f64>,
    pub seeded_disturbance_n: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCaseMetadata {
    pub case_id: String,
    pub group: String,
    pub note: String,
    pub continuous_power_mw: f64,
    pub burst_power_mw: f64,
    pub burst_duration_s: f64,
    pub pulse_energy_gj: f64,
    pub initial_ep_gj: f64,
    pub thermal_rejection_mw_per_k: f64,
    pub actuator_demand_scale: f64,
    pub damping_scale: f64,
    pub stiffness_scale: f64,
    pub burst_cadence_s: Option<f64>,
    pub allocation_strategy: Option<AllocationStrategy>,
}

#[derive(Debug, Clone)]
pub struct SweepCase {
    pub metadata: SweepCaseMetadata,
    pub config: SimulationConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OutputLayout {
    pub output_root: PathBuf,
}

impl Default for OutputLayout {
    fn default() -> Self {
        Self {
            output_root: PathBuf::from(DEFAULT_OUTPUT_ROOT),
        }
    }
}

impl OutputLayout {
    pub fn output_root(&self) -> &Path {
        &self.output_root
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "kebab-case")]
pub enum RunConfig {
    Scenario {
        preset: ScenarioPreset,
        seed: Option<u64>,
        output_root: Option<PathBuf>,
        overrides: Option<ScenarioOverrides>,
    },
    Sweep {
        preset: SweepPreset,
        seed: Option<u64>,
        output_root: Option<PathBuf>,
        overrides: Option<ScenarioOverrides>,
    },
}

#[derive(Debug, Clone)]
pub enum ResolvedRunConfig {
    Scenario {
        config: SimulationConfig,
        output_layout: OutputLayout,
    },
    Sweep {
        preset: SweepPreset,
        cases: Vec<SweepCase>,
        output_layout: OutputLayout,
    },
}

impl ResolvedRunConfig {
    pub fn from_run_config(run_config: RunConfig, base_dir: Option<&Path>) -> Result<Self> {
        match run_config {
            RunConfig::Scenario {
                preset,
                seed,
                output_root,
                overrides,
            } => {
                let seed = seed.unwrap_or(1);
                let config = crate::scenarios::build_scenario_config(
                    preset,
                    overrides.unwrap_or_default(),
                    seed,
                )?;
                Ok(Self::Scenario {
                    config,
                    output_layout: OutputLayout {
                        output_root: resolve_output_root(base_dir, output_root),
                    },
                })
            }
            RunConfig::Sweep {
                preset,
                seed,
                output_root,
                overrides,
            } => {
                let seed = seed.unwrap_or(1);
                let cases = crate::scenarios::build_sweep_cases(
                    preset,
                    overrides.unwrap_or_default(),
                    seed,
                )?;
                Ok(Self::Sweep {
                    preset,
                    cases,
                    output_layout: OutputLayout {
                        output_root: resolve_output_root(base_dir, output_root),
                    },
                })
            }
        }
    }
}

fn resolve_output_root(_base_dir: Option<&Path>, output_root: Option<PathBuf>) -> PathBuf {
    match output_root {
        Some(path) => path,
        None => PathBuf::from(DEFAULT_OUTPUT_ROOT),
    }
}

impl SimulationConfig {
    pub fn validate(&self) -> Result<()> {
        if self.solver.dt_s <= 0.0 {
            anyhow::bail!("solver dt_s must be positive");
        }
        if self.solver.duration_s <= 0.0 {
            anyhow::bail!("scenario duration must be positive");
        }
        if self.model.pulse_energy_max_j <= 0.0 {
            anyhow::bail!("pulse_energy_max_j must be positive");
        }
        if self.model.local_buffer_count != LIMB_COUNT {
            anyhow::bail!("local_buffer_count must equal {LIMB_COUNT}");
        }
        if self
            .scenario
            .segments
            .iter()
            .any(|segment| segment.end_s < segment.start_s)
        {
            anyhow::bail!("scenario segment end time must be >= start time");
        }
        Ok(())
    }

    pub fn apply_overrides(&mut self, overrides: &ScenarioOverrides) -> Result<()> {
        if let Some(value) = overrides.continuous_power_mw {
            self.model.continuous_power_w = mw_to_w(value);
        }
        if let Some(value) = overrides.pulse_energy_gj {
            let pulse_max_j = gj_to_j(value);
            let ratio = if self.model.pulse_energy_max_j > 0.0 {
                self.model.pulse_energy_min_j / self.model.pulse_energy_max_j
            } else {
                0.05
            };
            self.model.pulse_energy_max_j = pulse_max_j;
            self.model.pulse_energy_min_j = pulse_max_j * ratio;
            self.model.low_energy_threshold_j = pulse_max_j * 0.15;
            self.model.pulse_energy_initial_j = self
                .model
                .pulse_energy_initial_j
                .min(self.model.pulse_energy_max_j);
        }
        if let Some(value) = overrides.initial_ep_gj {
            self.model.pulse_energy_initial_j = gj_to_j(value);
        }
        if let Some(value) = overrides.duration_s {
            self.solver.duration_s = value;
        }
        if let Some(value) = overrides.dt_s {
            self.solver.dt_s = value;
        }
        if let Some(value) = overrides.thermal_rejection_mw_per_k {
            self.model.thermal_rejection_w_per_k = mw_to_w(value);
        }
        if let Some(value) = overrides.burst_power_mw {
            self.model.actuator_peak_power_w = mw_to_w(value);
        }
        if let Some(value) = overrides.burst_duration_s {
            for segment in &mut self.scenario.segments {
                if segment.label.contains("burst") {
                    segment.end_s = segment.start_s + value;
                }
            }
        }
        if let Some(value) = overrides.actuator_demand_scale {
            self.model.actuator_demand_scale = value;
        }
        if let Some(value) = overrides.allocation_strategy {
            self.scenario.baseline_allocation = value;
        }
        if let Some(value) = overrides.local_buffer_energy_mj {
            let energy_j = value * 1.0e6;
            self.model.local_buffer_energy_max_j = energy_j;
            self.model.local_buffer_initial_j = energy_j;
            self.model.local_buffer_low_threshold_j = energy_j * 0.20;
        }
        if let Some(value) = overrides.damping_scale {
            self.model.damping_scale = value;
        }
        if let Some(value) = overrides.stiffness_scale {
            self.model.stiffness_scale = value;
        }
        if let Some(value) = overrides.seeded_command_wobble {
            self.scenario.seeded_command_wobble = value;
        }
        if let Some(value) = overrides.seeded_disturbance_n {
            self.scenario.seeded_disturbance_n = value;
        }
        self.model.pulse_energy_initial_j = self
            .model
            .pulse_energy_initial_j
            .clamp(0.0, self.model.pulse_energy_max_j);
        self.validate()
            .context("post-override config validation failed")
    }
}

pub fn mw_to_w(value_mw: f64) -> f64 {
    value_mw * 1.0e6
}

pub fn gj_to_j(value_gj: f64) -> f64 {
    value_gj * 1.0e9
}

pub fn w_to_mw(value_w: f64) -> f64 {
    value_w / 1.0e6
}

pub fn j_to_gj(value_j: f64) -> f64 {
    value_j / 1.0e9
}
