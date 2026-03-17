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
    pub mean_delivered_ratio: f64,
    pub mean_authority_utilization: f64,
    pub reduced_response_efficiency: f64,
    pub effective_duty_cycle: f64,
    pub local_imbalance_max_j: f64,
    pub local_imbalance_rms_j: f64,
    pub min_local_buffer_j: f64,
    pub max_abs_y_m: f64,
    pub max_abs_v_mps: f64,
    pub min_gain: f64,
    pub min_delivered_ratio: f64,
    pub energy_depleted_j: f64,
    pub energy_depleted_gj: f64,
    pub recharge_fraction_of_full_reserve: f64,
    pub ideal_refill_time_s: Option<f64>,
    pub recharge_readiness_fraction: f64,
    pub successful_burst_fraction: f64,
    pub degraded_state_fraction: f64,
    pub percent_time_thermal_limited: f64,
    pub authority_loss_at_thermal_breach: Option<f64>,
    pub event_count: usize,
    pub energy_breach: bool,
    pub thermal_breach: bool,
    pub local_buffer_breach: bool,
    pub saturation_breach: bool,
    pub first_admissible_breach_s: Option<f64>,
    pub admissible_breach_count: usize,
    pub ep_clamped_count: usize,
    pub t_clamped_count: usize,
    pub y_clamped_count: usize,
    pub ydot_clamped_count: usize,
    pub percent_time_outside_admissible_region: f64,
    pub first_energy_breach_s: Option<f64>,
    pub first_thermal_breach_s: Option<f64>,
    pub first_local_buffer_breach_s: Option<f64>,
    pub first_saturation_breach_s: Option<f64>,
    pub stability_target_proxy: String,
    pub v_initial: f64,
    pub v_final: f64,
    pub v_max: f64,
    pub d_v_positive_fraction: f64,
    pub local_stability_margin: f64,
    pub first_positive_d_v_time_s: Option<f64>,
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
    let percent_time_thermal_limited = if config.solver.duration_s > 0.0 {
        100.0 * time_above_thermal_threshold_s / config.solver.duration_s
    } else {
        0.0
    };
    let max_actuator_demand_w = time_series
        .iter()
        .map(|record| record.requested_actuator_power_w)
        .fold(0.0, f64::max);
    let max_delivered_actuator_power_w = time_series
        .iter()
        .map(|record| record.delivered_actuator_power_w)
        .fold(0.0, f64::max);
    let mean_delivered_ratio = mean(time_series.iter().map(|record| record.delivered_ratio));
    let mean_authority_utilization = mean(
        time_series
            .iter()
            .map(|record| record.authority_utilization),
    );
    let saturation_count = time_series
        .iter()
        .filter(|record| record.saturation_fraction > 0.0)
        .count();
    let delivered_mechanical_work_j = time_series
        .iter()
        .map(|record| record.mechanical_power_w.max(0.0) * dt_s)
        .sum();
    let total_requested_energy_j: f64 = time_series
        .iter()
        .map(|record| record.requested_actuator_power_w.max(0.0) * dt_s)
        .sum();
    let reduced_response_efficiency = if total_requested_energy_j > 1.0 {
        delivered_mechanical_work_j / total_requested_energy_j
    } else {
        0.0
    };
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

    let (local_imbalance_max_j, local_imbalance_rms_j, min_local_buffer_j) =
        local_imbalance(limb_buffers);
    let recharge_time_s = recharge_time(time_series, config.model.pulse_energy_max_j);
    let energy_depleted_j = (config.model.pulse_energy_initial_j - min_ep_j).max(0.0);
    let energy_depleted_gj = energy_depleted_j / 1.0e9;
    let recharge_fraction_of_full_reserve =
        energy_depleted_j / config.model.pulse_energy_max_j.max(1.0);
    let ideal_refill_time_s =
        if config.model.recharge_efficiency * config.model.continuous_power_w > 1.0 {
            Some(
                energy_depleted_j
                    / (config.model.recharge_efficiency * config.model.continuous_power_w),
            )
        } else {
            None
        };
    let recharge_readiness_fraction = recharge_readiness_fraction(
        time_series,
        limb_buffers,
        config.model.pulse_energy_max_j,
        config.model.local_buffer_energy_max_j,
    );
    let successful_burst_fraction = successful_burst_fraction(config, time_series);
    let degraded_state_fraction = degraded_state_fraction(config, time_series);

    let first_energy_breach_s = events
        .iter()
        .find(|event| event.event_type == "energy_low")
        .map(|event| event.time_s);
    let first_thermal_breach_s = events
        .iter()
        .find(|event| event.event_type == "thermal_high")
        .map(|event| event.time_s);
    let authority_loss_at_thermal_breach = first_thermal_breach_s.and_then(|time_s| {
        time_series
            .iter()
            .find(|record| record.time_s >= time_s)
            .map(|record| 1.0 - record.authority_utilization)
    });
    let first_local_buffer_breach_s = events
        .iter()
        .find(|event| event.event_type == "local_buffer_low")
        .map(|event| event.time_s);
    let first_saturation_breach_s = events
        .iter()
        .find(|event| event.event_type == "actuator_saturation")
        .map(|event| event.time_s);
    let first_admissible_breach_s = events
        .iter()
        .find(|event| event.event_type == "admissible_region_breach")
        .map(|event| event.time_s);
    let admissible_breach_count = events
        .iter()
        .filter(|event| event.event_type == "admissible_region_breach")
        .count();
    let ep_clamped_count = time_series
        .iter()
        .filter(|record| record.ep_clamped)
        .count();
    let t_clamped_count = time_series
        .iter()
        .filter(|record| record.temperature_clamped)
        .count();
    let y_clamped_count = time_series.iter().filter(|record| record.y_clamped).count();
    let ydot_clamped_count = time_series
        .iter()
        .filter(|record| record.ydot_clamped)
        .count();
    let percent_time_outside_admissible_region = percent(
        time_series
            .iter()
            .filter(|record| record.outside_admissible_region)
            .count(),
        time_series.len(),
    );
    let time_to_any_threshold_s = [
        first_energy_breach_s,
        first_thermal_breach_s,
        first_local_buffer_breach_s,
        first_saturation_breach_s,
        first_admissible_breach_s,
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
        mean_delivered_ratio,
        mean_authority_utilization,
        reduced_response_efficiency,
        effective_duty_cycle,
        local_imbalance_max_j,
        local_imbalance_rms_j,
        min_local_buffer_j,
        max_abs_y_m,
        max_abs_v_mps,
        min_gain: if min_gain.is_finite() { min_gain } else { 0.0 },
        min_delivered_ratio: if min_delivered_ratio.is_finite() {
            min_delivered_ratio
        } else {
            1.0
        },
        energy_depleted_j,
        energy_depleted_gj,
        recharge_fraction_of_full_reserve,
        ideal_refill_time_s,
        recharge_readiness_fraction,
        successful_burst_fraction,
        degraded_state_fraction,
        percent_time_thermal_limited,
        authority_loss_at_thermal_breach,
        event_count: events.len(),
        energy_breach,
        thermal_breach,
        local_buffer_breach,
        saturation_breach,
        first_admissible_breach_s,
        admissible_breach_count,
        ep_clamped_count,
        t_clamped_count,
        y_clamped_count,
        ydot_clamped_count,
        percent_time_outside_admissible_region,
        first_energy_breach_s,
        first_thermal_breach_s,
        first_local_buffer_breach_s,
        first_saturation_breach_s,
        stability_target_proxy: "command-scaled reduced maneuver response proxy".to_string(),
        v_initial: time_series
            .first()
            .map(|record| record.lyapunov_v)
            .unwrap_or(0.0),
        v_final: time_series
            .last()
            .map(|record| record.lyapunov_v)
            .unwrap_or(0.0),
        v_max: time_series
            .iter()
            .map(|record| record.lyapunov_v)
            .fold(0.0, f64::max),
        d_v_positive_fraction: ratio(
            time_series
                .iter()
                .filter(|record| record.lyapunov_dv_dt > 0.0)
                .count(),
            time_series.len(),
        ),
        local_stability_margin: stability_margin(time_series),
        first_positive_d_v_time_s: time_series
            .iter()
            .find(|record| record.lyapunov_dv_dt > 0.0)
            .map(|record| record.time_s),
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
        metric(
            "mean_delivered_ratio",
            summary.mean_delivered_ratio,
            "fraction",
        ),
        metric(
            "mean_authority_utilization",
            summary.mean_authority_utilization,
            "fraction",
        ),
        metric(
            "reduced_response_efficiency",
            summary.reduced_response_efficiency,
            "fraction",
        ),
        metric(
            "effective_duty_cycle",
            summary.effective_duty_cycle,
            "fraction",
        ),
        metric("local_imbalance_max_j", summary.local_imbalance_max_j, "J"),
        metric("local_imbalance_rms_j", summary.local_imbalance_rms_j, "J"),
        metric("min_local_buffer_j", summary.min_local_buffer_j, "J"),
        metric("max_abs_y_m", summary.max_abs_y_m, "m"),
        metric("max_abs_v_mps", summary.max_abs_v_mps, "m/s"),
        metric("min_gain", summary.min_gain, "N"),
        metric(
            "min_delivered_ratio",
            summary.min_delivered_ratio,
            "fraction",
        ),
        metric("energy_depleted_j", summary.energy_depleted_j, "J"),
        metric("energy_depleted_gj", summary.energy_depleted_gj, "GJ"),
        metric(
            "recharge_fraction_of_full_reserve",
            summary.recharge_fraction_of_full_reserve,
            "fraction",
        ),
        metric(
            "recharge_readiness_fraction",
            summary.recharge_readiness_fraction,
            "fraction",
        ),
        metric(
            "successful_burst_fraction",
            summary.successful_burst_fraction,
            "fraction",
        ),
        metric(
            "degraded_state_fraction",
            summary.degraded_state_fraction,
            "fraction",
        ),
        metric(
            "percent_time_thermal_limited",
            summary.percent_time_thermal_limited,
            "percent",
        ),
        metric(
            "admissible_breach_count",
            summary.admissible_breach_count as f64,
            "count",
        ),
        metric("ep_clamped_count", summary.ep_clamped_count as f64, "count"),
        metric("t_clamped_count", summary.t_clamped_count as f64, "count"),
        metric("y_clamped_count", summary.y_clamped_count as f64, "count"),
        metric(
            "ydot_clamped_count",
            summary.ydot_clamped_count as f64,
            "count",
        ),
        metric(
            "percent_time_outside_admissible_region",
            summary.percent_time_outside_admissible_region,
            "percent",
        ),
        metric("v_initial", summary.v_initial, "J"),
        metric("v_final", summary.v_final, "J"),
        metric("v_max", summary.v_max, "J"),
        metric(
            "d_v_positive_fraction",
            summary.d_v_positive_fraction,
            "fraction",
        ),
        metric(
            "local_stability_margin",
            summary.local_stability_margin,
            "1/s",
        ),
    ];
    if let Some(value) = summary.recharge_time_s {
        metrics.push(metric("recharge_time_s", value, "s"));
    }
    if let Some(value) = summary.ideal_refill_time_s {
        metrics.push(metric("ideal_refill_time_s", value, "s"));
    }
    if let Some(value) = summary.authority_loss_at_thermal_breach {
        metrics.push(metric(
            "authority_loss_at_thermal_breach",
            value,
            "fraction",
        ));
    }
    if let Some(value) = summary.time_to_any_threshold_s {
        metrics.push(metric("time_to_any_threshold_s", value, "s"));
    }
    if let Some(value) = summary.first_admissible_breach_s {
        metrics.push(metric("first_admissible_breach_s", value, "s"));
    }
    if let Some(value) = summary.first_positive_d_v_time_s {
        metrics.push(metric("first_positive_d_v_time_s", value, "s"));
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

fn local_imbalance(limb_buffers: &[LimbBufferRecord]) -> (f64, f64, f64) {
    let mut spreads = Vec::new();
    let min_local_buffer_j = limb_buffers
        .iter()
        .map(|record| record.buffer_energy_j)
        .fold(f64::INFINITY, f64::min);
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
        return (0.0, 0.0, 0.0);
    }
    let max_spread = spreads.iter().copied().fold(0.0, f64::max);
    let rms =
        (spreads.iter().map(|spread| spread * spread).sum::<f64>() / spreads.len() as f64).sqrt();
    (
        max_spread,
        rms,
        if min_local_buffer_j.is_finite() {
            min_local_buffer_j
        } else {
            0.0
        },
    )
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

fn recharge_readiness_fraction(
    time_series: &[TimeSeriesRecord],
    limb_buffers: &[LimbBufferRecord],
    pulse_energy_max_j: f64,
    local_buffer_energy_max_j: f64,
) -> f64 {
    if time_series.is_empty() {
        return 0.0;
    }
    let ep_ready_threshold = pulse_energy_max_j * 0.90;
    let local_ready_threshold = local_buffer_energy_max_j * 0.85;
    let mut ready_count = 0usize;
    for (record, chunk) in time_series.iter().zip(limb_buffers.chunks_exact(4)) {
        let local_ready = chunk
            .iter()
            .all(|limb| limb.buffer_energy_j >= local_ready_threshold);
        if record.ep_j >= ep_ready_threshold && local_ready {
            ready_count += 1;
        }
    }
    ratio(ready_count, time_series.len())
}

fn successful_burst_fraction(config: &SimulationConfig, time_series: &[TimeSeriesRecord]) -> f64 {
    let burst_segments: Vec<_> = config
        .scenario
        .segments
        .iter()
        .filter(|segment| segment.label.contains("burst") || segment.demand_fraction >= 0.75)
        .collect();
    if burst_segments.is_empty() {
        return 0.0;
    }
    let successful = burst_segments
        .iter()
        .filter(|segment| {
            let samples: Vec<_> = time_series
                .iter()
                .filter(|record| record.time_s >= segment.start_s && record.time_s <= segment.end_s)
                .collect();
            !samples.is_empty()
                && samples.iter().all(|record| {
                    record.delivered_ratio >= 0.90
                        && !record.outside_admissible_region
                        && record.temperature_k < config.model.thermal_limit_k
                        && record.ep_j >= config.model.pulse_energy_min_j
                })
        })
        .count();
    ratio(successful, burst_segments.len())
}

fn degraded_state_fraction(config: &SimulationConfig, time_series: &[TimeSeriesRecord]) -> f64 {
    ratio(
        time_series
            .iter()
            .filter(|record| {
                record.delivered_ratio < 0.90
                    || record.authority_utilization < 0.70
                    || record.temperature_k >= config.model.thermal_soft_limit_k
                    || record.ep_j < config.model.low_energy_threshold_j
            })
            .count(),
        time_series.len(),
    )
}

fn stability_margin(time_series: &[TimeSeriesRecord]) -> f64 {
    if time_series.is_empty() {
        return 0.0;
    }
    let mean_dv_dt = mean(time_series.iter().map(|record| record.lyapunov_dv_dt));
    let v_max = time_series
        .iter()
        .map(|record| record.lyapunov_v)
        .fold(0.0, f64::max);
    -mean_dv_dt / v_max.max(1.0)
}

fn mean(values: impl Iterator<Item = f64>) -> f64 {
    let (sum, count) = values.fold((0.0, 0usize), |(sum, count), value| {
        (sum + value, count + 1)
    });
    if count == 0 { 0.0 } else { sum / count as f64 }
}

fn ratio(numerator: usize, denominator: usize) -> f64 {
    if denominator == 0 {
        0.0
    } else {
        numerator as f64 / denominator as f64
    }
}

fn percent(numerator: usize, denominator: usize) -> f64 {
    ratio(numerator, denominator) * 100.0
}
