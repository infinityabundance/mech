use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::{SweepCase, SweepPreset};
use crate::integrator::simulate;
use crate::outputs::write_run_outputs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepCaseSummary {
    pub case_id: String,
    pub group: String,
    pub note: String,
    pub scenario: String,
    pub output_dir: String,
    pub success: bool,
    pub continuous_power_mw: f64,
    pub burst_power_mw: f64,
    pub burst_duration_s: f64,
    pub pulse_energy_gj: f64,
    pub initial_ep_gj: f64,
    pub thermal_rejection_mw_per_k: f64,
    pub actuator_demand_scale: f64,
    pub damping_scale: f64,
    pub stiffness_scale: f64,
    pub min_ep_gj: f64,
    pub peak_temperature_k: f64,
    pub peak_temperature_c: f64,
    pub recharge_time_s: Option<f64>,
    pub time_to_any_threshold_s: Option<f64>,
    pub effective_duty_cycle: f64,
    pub saturation_count: usize,
    pub delivered_mechanical_work_j: f64,
    pub energy_breach: bool,
    pub thermal_breach: bool,
    pub local_buffer_breach: bool,
    pub saturation_breach: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SweepAggregate {
    pub preset: String,
    pub root_dir: PathBuf,
    pub cases_dir: PathBuf,
    pub case_summaries: Vec<SweepCaseSummary>,
}

pub fn run_sweep(preset: SweepPreset, cases: Vec<SweepCase>, run_root: &Path) -> Result<SweepAggregate> {
    let cases_dir = run_root.join("sweeps").join(preset.as_str());
    fs::create_dir_all(&cases_dir)?;

    let mut case_summaries = Vec::new();
    for case in cases {
        let case_dir = cases_dir.join(&case.metadata.case_id);
        fs::create_dir_all(&case_dir)?;
        let result = simulate(case.config.clone())?;
        write_run_outputs(&case_dir, &result)?;

        case_summaries.push(SweepCaseSummary {
            case_id: case.metadata.case_id.clone(),
            group: case.metadata.group.clone(),
            note: case.metadata.note.clone(),
            scenario: result.config.scenario.name.clone(),
            output_dir: case_dir.to_string_lossy().to_string(),
            success: result.summary.success,
            continuous_power_mw: case.metadata.continuous_power_mw,
            burst_power_mw: case.metadata.burst_power_mw,
            burst_duration_s: case.metadata.burst_duration_s,
            pulse_energy_gj: case.metadata.pulse_energy_gj,
            initial_ep_gj: case.metadata.initial_ep_gj,
            thermal_rejection_mw_per_k: case.metadata.thermal_rejection_mw_per_k,
            actuator_demand_scale: case.metadata.actuator_demand_scale,
            damping_scale: case.metadata.damping_scale,
            stiffness_scale: case.metadata.stiffness_scale,
            min_ep_gj: result.summary.min_ep_j / 1.0e9,
            peak_temperature_k: result.summary.peak_temperature_k,
            peak_temperature_c: result.summary.peak_temperature_k - 273.15,
            recharge_time_s: result.summary.recharge_time_s,
            time_to_any_threshold_s: result.summary.time_to_any_threshold_s,
            effective_duty_cycle: result.summary.effective_duty_cycle,
            saturation_count: result.summary.saturation_count,
            delivered_mechanical_work_j: result.summary.delivered_mechanical_work_j,
            energy_breach: result.summary.energy_breach,
            thermal_breach: result.summary.thermal_breach,
            local_buffer_breach: result.summary.local_buffer_breach,
            saturation_breach: result.summary.saturation_breach,
        });
    }

    Ok(SweepAggregate {
        preset: preset.as_str().to_string(),
        root_dir: run_root.to_path_buf(),
        cases_dir,
        case_summaries,
    })
}
