use serde::{Deserialize, Serialize};

use crate::config::SimulationConfig;
use crate::model::ModelParameters;

const ADMISSIBLE_MARGIN_FRACTION: f64 = 0.05;
const REDUCED_RESPONSE_TARGET_FRACTION: f64 = 0.12;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AdmissibleStatus {
    pub below_energy_min: bool,
    pub above_energy_max: bool,
    pub below_temperature_min: bool,
    pub above_temperature_max: bool,
    pub above_abs_y_max: bool,
    pub above_abs_v_max: bool,
    pub near_boundary: bool,
    pub outside_region: bool,
    pub margin_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StabilitySummary {
    pub target_proxy: String,
    pub v_initial: f64,
    pub v_final: f64,
    pub v_max: f64,
    pub d_v_positive_fraction: f64,
    pub local_stability_margin: f64,
    pub first_positive_d_v_time_s: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FigureMetadata {
    pub scenario: String,
    pub y_label: String,
    pub y_interpretation_note: String,
    pub mechanical_work_interpretation_note: String,
    pub recharge_interpretation_note: String,
    pub burst_windows: Vec<BurstWindow>,
    pub thresholds: ThresholdMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BurstWindow {
    pub label: String,
    pub start_s: f64,
    pub end_s: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdMetadata {
    pub energy_min_j: f64,
    pub energy_threshold_j: f64,
    pub energy_max_j: f64,
    pub temperature_min_k: f64,
    pub temperature_max_k: f64,
    pub y_abs_max: f64,
    pub v_abs_max: f64,
}

pub fn admissible_status(
    params: &ModelParameters,
    ep_j: f64,
    temperature_k: f64,
    y_m: f64,
    v_mps: f64,
) -> AdmissibleStatus {
    let energy_min = params.pulse_energy_min_j;
    let energy_max = params.pulse_energy_max_j;
    let temperature_min = params.ambient_temperature_k;
    let temperature_max = params.thermal_limit_k;
    let y_abs_max = params.max_displacement_m;
    let v_abs_max = params.max_velocity_m_per_s;

    let below_energy_min = ep_j < energy_min;
    let above_energy_max = ep_j > energy_max;
    let below_temperature_min = temperature_k < temperature_min;
    let above_temperature_max = temperature_k > temperature_max;
    let above_abs_y_max = y_m.abs() > y_abs_max;
    let above_abs_v_max = v_mps.abs() > v_abs_max;

    let energy_margin = normalized_margin(ep_j, energy_min, energy_max);
    let temperature_margin = normalized_margin(temperature_k, temperature_min, temperature_max);
    let y_margin = normalized_margin(y_m.abs(), 0.0, y_abs_max);
    let v_margin = normalized_margin(v_mps.abs(), 0.0, v_abs_max);
    let margin_fraction = energy_margin
        .min(temperature_margin)
        .min(y_margin)
        .min(v_margin);

    AdmissibleStatus {
        below_energy_min,
        above_energy_max,
        below_temperature_min,
        above_temperature_max,
        above_abs_y_max,
        above_abs_v_max,
        near_boundary: margin_fraction <= ADMISSIBLE_MARGIN_FRACTION,
        outside_region: below_energy_min
            || above_energy_max
            || below_temperature_min
            || above_temperature_max
            || above_abs_y_max
            || above_abs_v_max,
        margin_fraction,
    }
}

pub fn reduced_response_target(command_fraction: f64, params: &ModelParameters) -> f64 {
    command_fraction.max(0.0) * params.max_displacement_m * REDUCED_RESPONSE_TARGET_FRACTION
}

pub fn lyapunov_value(
    mechanical_mass_kg: f64,
    k_eff_n_per_m: f64,
    error_m: f64,
    error_rate_mps: f64,
) -> f64 {
    0.5 * mechanical_mass_kg.max(1.0) * error_rate_mps * error_rate_mps
        + 0.5 * k_eff_n_per_m.max(0.0) * error_m * error_m
}

pub fn normalized_authority_utilization(
    params: &ModelParameters,
    gain_n: f64,
    command_fraction: f64,
    delivered_ratio: f64,
) -> f64 {
    let gain_fraction = gain_n / params.reference_actuator_force_n.max(1.0);
    (gain_fraction * command_fraction.max(0.0) * delivered_ratio).clamp(0.0, 1.5)
}

pub fn stability_summary(
    v_series: &[f64],
    dv_dt_series: &[f64],
    time_series: &[f64],
) -> StabilitySummary {
    let v_initial = v_series.first().copied().unwrap_or(0.0);
    let v_final = v_series.last().copied().unwrap_or(0.0);
    let v_max = v_series.iter().copied().fold(0.0, f64::max);
    let positive_count = dv_dt_series.iter().filter(|value| **value > 0.0).count();
    let d_v_positive_fraction = if dv_dt_series.is_empty() {
        0.0
    } else {
        positive_count as f64 / dv_dt_series.len() as f64
    };
    let mean_dv_dt = if dv_dt_series.is_empty() {
        0.0
    } else {
        dv_dt_series.iter().sum::<f64>() / dv_dt_series.len() as f64
    };
    let local_stability_margin = -mean_dv_dt / v_max.max(1.0);
    let first_positive_d_v_time_s = dv_dt_series
        .iter()
        .zip(time_series.iter())
        .find(|(value, _)| **value > 0.0)
        .map(|(_, time_s)| *time_s);

    StabilitySummary {
        target_proxy: "command-scaled reduced maneuver response proxy".to_string(),
        v_initial,
        v_final,
        v_max,
        d_v_positive_fraction,
        local_stability_margin,
        first_positive_d_v_time_s,
    }
}

pub fn build_figure_metadata(config: &SimulationConfig) -> FigureMetadata {
    let burst_windows = config
        .scenario
        .segments
        .iter()
        .filter(|segment| segment.label.contains("burst") || segment.demand_fraction >= 0.75)
        .map(|segment| BurstWindow {
            label: segment.label.clone(),
            start_s: segment.start_s,
            end_s: segment.end_s,
        })
        .collect();

    FigureMetadata {
        scenario: config.scenario.name.clone(),
        y_label: "Reduced Maneuver Response y".to_string(),
        y_interpretation_note: "The state y is a reduced-order maneuver response proxy coupled to energy and thermal state. It is not intended to represent literal full-body mech displacement.".to_string(),
        mechanical_work_interpretation_note: "Delivered mechanical work in this crate should be read as reduced-order architecture evidence for authority delivery, not as a full rigid-body maneuver validation.".to_string(),
        recharge_interpretation_note: "Recharge time depends on the actual depleted energy in a run. A short burst-refill tail and a longer 3 GJ recharge case are consistent because they correspond to different refill depths.".to_string(),
        burst_windows,
        thresholds: ThresholdMetadata {
            energy_min_j: config.model.pulse_energy_min_j,
            energy_threshold_j: config.model.low_energy_threshold_j,
            energy_max_j: config.model.pulse_energy_max_j,
            temperature_min_k: config.model.ambient_temperature_k,
            temperature_max_k: config.model.thermal_limit_k,
            y_abs_max: config.model.max_displacement_m,
            v_abs_max: config.model.max_velocity_m_per_s,
        },
    }
}

fn normalized_margin(value: f64, min: f64, max: f64) -> f64 {
    let span = (max - min).abs().max(1.0e-9);
    let lower = (value - min) / span;
    let upper = (max - value) / span;
    lower.min(upper)
}
