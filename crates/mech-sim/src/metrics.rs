use serde::{Deserialize, Serialize};

use crate::config::SimulationConfig;
use crate::state::{DerivedMetricRecord, EventRecord, LimbBufferRecord, TimeSeriesRecord};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunSummary {
    pub run_name: String,
    pub scenario: String,
    pub success: bool,
    pub failure_reasons: Vec<String>,
    pub duration_s: f64,
    pub dt_s: f64,
    pub seed: u64,
    pub min_ep_j: f64,
    pub max_ep_j: f64,
    pub final_ep_j: f64,
    pub peak_temperature_k: f64,
    pub final_temperature_k: f64,
    pub time_below_energy_threshold_s: f64,
    pub time_above_thermal_threshold_s: f64,
    pub max_actuator_demand_w: f64,
    pub max_delivered_actuator_power_w: f64,
    pub saturation_count: usize,
    pub recharge_time_s: Option<f64>,
    pub delivered_mechanical_work_j: f64,
    pub effective_duty_cycle: f64,
    pub local_imbalance_max_j: f64,
    pub local_imbalance_rms_j: f64,
    pub max_abs_y_m: f64,
    pub max_abs_v_mps: f64,
    pub min_gain: f64,
    pub min_delivered_ratio: f64,
    pub event_count: usize,
    pub energy_breach: bool,
    pub thermal_breach: bool,
    pub local_buffer_breach: bool,
    pub saturation_breach: bool,
    pub first_energy_breach_s: Option<f64>,
    pub first_thermal_breach_s: Option<f64>,
    pub first_local_buffer_breach_s: Option<f64>,
    pub first_saturation_breach_s: Option<f64>,
    pub time_to_any_threshold_s: Option<f64>,
}

pub fn summarize(
    config: &SimulationConfig,
    time_series: &[TimeSeriesRecord],
    limb_buffers: &[LimbBufferRecord],
    events: &[EventRecord],
) -> RunSummary {
    let dt_s = config.solver.dt_s;
    let min_ep_j = time_series
        .iter()
        .map(|record| record.ep_j)
        .fold(f64::INFINITY, f64::min);
    let max_ep_j = time_series
        .iter()
        .map(|record| record.ep_j)
        .fold(f64::NEG_INFINITY, f64::max);
    let final_ep_j = time_series.last().map(|record| record.ep_j).unwrap_or(0.0);
    let peak_temperature_k = time_series
        .iter()
        .map(|record| record.temperature_k)
        .fold(f64::NEG_INFINITY, f64::max);
    let final_temperature_k = time_series
        .last()
        .map(|record| record.temperature_k)
        .unwrap_or(config.model.thermal_initial_k);
    let time_below_energy_threshold_s = time_series
        .iter()
        .filter(|record| record.ep_j < config.model.low_energy_threshold_j)
        .count() as f64
        * dt_s;
    let time_above_thermal_threshold_s = time_series
        .iter()
        .filter(|record| record.temperature_k >= config.model.thermal_limit_k)
        .count() as f64
        * dt_s;
    let max_actuator_demand_w = time_series
        .iter()
        .map(|record| record.requested_actuator_power_w)
        .fold(0.0, f64::max);
    let max_delivered_actuator_power_w = time_series
        .iter()
        .map(|record| record.delivered_actuator_power_w)
        .fold(0.0, f64::max);
    let saturation_count = time_series
        .iter()
        .filter(|record| record.saturation_fraction > 0.0)
        .count();
    let delivered_mechanical_work_j = time_series
        .iter()
        .map(|record| record.mechanical_power_w.max(0.0) * dt_s)
        .sum();
    let effective_duty_cycle = if config.solver.duration_s > 0.0 {
        time_series
            .iter()
            .filter(|record| record.command_fraction > 0.2 && record.saturation_fraction < 0.05)
            .count() as f64
            * dt_s
            / config.solver.duration_s
    } else {
        0.0
    };
    let max_abs_y_m = time_series
        .iter()
        .map(|record| record.y_m.abs())
        .fold(0.0, f64::max);
    let max_abs_v_mps = time_series
        .iter()
        .map(|record| record.v_mps.abs())
        .fold(0.0, f64::max);
    let min_gain = time_series
        .iter()
        .filter(|record| record.time_s > 0.0 || record.gain > 0.0)
        .map(|record| record.gain)
        .fold(f64::INFINITY, f64::min);
    let min_delivered_ratio = time_series
        .iter()
        .filter(|record| record.time_s > 0.0 || record.delivered_ratio < 1.0)
        .map(|record| record.delivered_ratio)
        .fold(f64::INFINITY, f64::min);

    let (local_imbalance_max_j, local_imbalance_rms_j) = local_imbalance(limb_buffers);
    let recharge_time_s = recharge_time(time_series, config.model.pulse_energy_max_j);

    let first_energy_breach_s = events
        .iter()
        .find(|event| event.event_type == "energy_low")
        .map(|event| event.time_s);
    let first_thermal_breach_s = events
        .iter()
        .find(|event| event.event_type == "thermal_high")
        .map(|event| event.time_s);
    let first_local_buffer_breach_s = events
        .iter()
        .find(|event| event.event_type == "local_buffer_low")
        .map(|event| event.time_s);
    let first_saturation_breach_s = events
        .iter()
        .find(|event| event.event_type == "actuator_saturation")
        .map(|event| event.time_s);
    let time_to_any_threshold_s = [
        first_energy_breach_s,
        first_thermal_breach_s,
        first_local_buffer_breach_s,
        first_saturation_breach_s,
    ]
    .into_iter()
    .flatten()
    .fold(None::<f64>, |acc, value| match acc {
        Some(current) => Some(current.min(value)),
        None => Some(value),
    });

    let energy_breach = first_energy_breach_s.is_some();
    let thermal_breach = first_thermal_breach_s.is_some();
    let local_buffer_breach = first_local_buffer_breach_s.is_some();
    let saturation_breach = first_saturation_breach_s.is_some();
    let mut failure_reasons = Vec::new();
    if energy_breach {
        failure_reasons.push("pulse energy dropped below threshold".to_string());
    }
    if thermal_breach {
        failure_reasons.push("aggregate thermal state exceeded threshold".to_string());
    }
    if local_buffer_breach {
        failure_reasons.push("local limb buffer dropped below threshold".to_string());
    }
    if saturation_breach {
        failure_reasons.push("actuator delivery saturated".to_string());
    }

    RunSummary {
        run_name: config.name.clone(),
        scenario: config.scenario.name.clone(),
        success: failure_reasons.is_empty(),
        failure_reasons,
        duration_s: config.solver.duration_s,
        dt_s,
        seed: config.seed,
        min_ep_j,
        max_ep_j,
        final_ep_j,
        peak_temperature_k,
        final_temperature_k,
        time_below_energy_threshold_s,
        time_above_thermal_threshold_s,
        max_actuator_demand_w,
        max_delivered_actuator_power_w,
        saturation_count,
        recharge_time_s,
        delivered_mechanical_work_j,
        effective_duty_cycle,
        local_imbalance_max_j,
        local_imbalance_rms_j,
        max_abs_y_m,
        max_abs_v_mps,
        min_gain: if min_gain.is_finite() { min_gain } else { 0.0 },
        min_delivered_ratio: if min_delivered_ratio.is_finite() {
            min_delivered_ratio
        } else {
            1.0
        },
        event_count: events.len(),
        energy_breach,
        thermal_breach,
        local_buffer_breach,
        saturation_breach,
        first_energy_breach_s,
        first_thermal_breach_s,
        first_local_buffer_breach_s,
        first_saturation_breach_s,
        time_to_any_threshold_s,
    }
}

pub fn derived_metrics(summary: &RunSummary) -> Vec<DerivedMetricRecord> {
    let mut metrics = vec![
        metric("min_ep_j", summary.min_ep_j, "J"),
        metric("max_ep_j", summary.max_ep_j, "J"),
        metric("final_ep_j", summary.final_ep_j, "J"),
        metric("peak_temperature_k", summary.peak_temperature_k, "K"),
        metric(
            "time_below_energy_threshold_s",
            summary.time_below_energy_threshold_s,
            "s",
        ),
        metric(
            "time_above_thermal_threshold_s",
            summary.time_above_thermal_threshold_s,
            "s",
        ),
        metric("max_actuator_demand_w", summary.max_actuator_demand_w, "W"),
        metric(
            "max_delivered_actuator_power_w",
            summary.max_delivered_actuator_power_w,
            "W",
        ),
        metric("saturation_count", summary.saturation_count as f64, "count"),
        metric(
            "delivered_mechanical_work_j",
            summary.delivered_mechanical_work_j,
            "J",
        ),
        metric("effective_duty_cycle", summary.effective_duty_cycle, "fraction"),
        metric("local_imbalance_max_j", summary.local_imbalance_max_j, "J"),
        metric("local_imbalance_rms_j", summary.local_imbalance_rms_j, "J"),
        metric("max_abs_y_m", summary.max_abs_y_m, "m"),
        metric("max_abs_v_mps", summary.max_abs_v_mps, "m/s"),
        metric("min_gain", summary.min_gain, "N"),
        metric("min_delivered_ratio", summary.min_delivered_ratio, "fraction"),
    ];
    if let Some(value) = summary.recharge_time_s {
        metrics.push(metric("recharge_time_s", value, "s"));
    }
    if let Some(value) = summary.time_to_any_threshold_s {
        metrics.push(metric("time_to_any_threshold_s", value, "s"));
    }
    metrics
}

fn metric(name: &str, value: f64, unit: &str) -> DerivedMetricRecord {
    DerivedMetricRecord {
        metric: name.to_string(),
        value,
        unit: unit.to_string(),
    }
}

fn local_imbalance(limb_buffers: &[LimbBufferRecord]) -> (f64, f64) {
    let mut spreads = Vec::new();
    for chunk in limb_buffers.chunks_exact(4) {
        let min_value = chunk
            .iter()
            .map(|record| record.buffer_energy_j)
            .fold(f64::INFINITY, f64::min);
        let max_value = chunk
            .iter()
            .map(|record| record.buffer_energy_j)
            .fold(f64::NEG_INFINITY, f64::max);
        spreads.push(max_value - min_value);
    }
    if spreads.is_empty() {
        return (0.0, 0.0);
    }
    let max_spread = spreads.iter().copied().fold(0.0, f64::max);
    let rms = (spreads.iter().map(|spread| spread * spread).sum::<f64>() / spreads.len() as f64).sqrt();
    (max_spread, rms)
}

fn recharge_time(time_series: &[TimeSeriesRecord], pulse_energy_max_j: f64) -> Option<f64> {
    let mut min_index = None;
    let mut min_ep = f64::INFINITY;
    for (index, record) in time_series.iter().enumerate() {
        if record.ep_j < min_ep {
            min_ep = record.ep_j;
            min_index = Some(index);
        }
    }
    let min_index = min_index?;
    let target = pulse_energy_max_j * 0.95;
    let min_time = time_series[min_index].time_s;
    time_series[min_index..]
        .iter()
        .find(|record| record.ep_j >= target)
        .map(|record| record.time_s - min_time)
}
