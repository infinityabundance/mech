use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::config::SimulationConfig;
use crate::metrics::{RunSummary, derived_metrics, summarize};
use crate::model::step_state;
use crate::monitor::{
    FigureMetadata, StabilitySummary, admissible_status, build_figure_metadata, lyapunov_value,
    normalized_authority_utilization, reduced_response_target, stability_summary,
};
use crate::scenarios::sample_control;
use crate::state::{
    DerivedMetricRecord, EventLatch, EventRecord, LimbBufferRecord, StepDiagnostics, SystemState,
    TimeSeriesRecord,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SimulationResult {
    pub config: SimulationConfig,
    pub time_series: Vec<TimeSeriesRecord>,
    pub limb_buffers: Vec<LimbBufferRecord>,
    pub events: Vec<EventRecord>,
    pub summary: RunSummary,
    pub derived_metrics: Vec<DerivedMetricRecord>,
    pub stability_summary: StabilitySummary,
    pub figure_metadata: FigureMetadata,
}

pub fn simulate(config: SimulationConfig) -> Result<SimulationResult> {
    config.validate()?;

    let mut state = SystemState::new(
        config.model.pulse_energy_initial_j,
        config.model.thermal_initial_k,
        0.0,
        0.0,
        config.model.local_buffer_initial_j,
    );
    let mut time_series = Vec::new();
    let mut limb_buffers = Vec::new();
    let mut events = Vec::new();
    let mut latch = EventLatch::default();
    let mut previous_segment = "initial".to_string();
    let mut previous_lyapunov = None;

    push_records(
        &config,
        &mut time_series,
        &mut limb_buffers,
        &state,
        &StepDiagnostics::zero(),
        &mut previous_lyapunov,
    );

    while state.time_s < config.solver.duration_s - 1.0e-12 {
        let remaining = config.solver.duration_s - state.time_s;
        let dt_s = remaining.min(config.solver.dt_s);
        let input = sample_control(&config.scenario, config.seed, state.time_s);
        let outcome = step_state(&config.model, &state, &input, dt_s);

        if outcome.diagnostics.active_segment != previous_segment {
            events.push(EventRecord {
                time_s: outcome.next_state.time_s,
                event_type: "segment_transition".to_string(),
                severity: "info".to_string(),
                message: format!(
                    "Transitioned from '{}' to '{}'",
                    previous_segment, outcome.diagnostics.active_segment
                ),
                value: outcome.diagnostics.command_fraction,
                threshold: 0.0,
            });
            previous_segment = outcome.diagnostics.active_segment.clone();
        }

        update_constraint_events(
            &config,
            &outcome.next_state,
            &outcome.diagnostics,
            &mut latch,
            &mut events,
        );

        state = outcome.next_state;
        push_records(
            &config,
            &mut time_series,
            &mut limb_buffers,
            &state,
            &outcome.diagnostics,
            &mut previous_lyapunov,
        );
    }

    let summary = summarize(&config, &time_series, &limb_buffers, &events);
    let derived_metrics = derived_metrics(&summary);
    let stability_summary = stability_summary(
        &time_series
            .iter()
            .map(|record| record.lyapunov_v)
            .collect::<Vec<_>>(),
        &time_series
            .iter()
            .map(|record| record.lyapunov_dv_dt)
            .collect::<Vec<_>>(),
        &time_series
            .iter()
            .map(|record| record.time_s)
            .collect::<Vec<_>>(),
    );
    let figure_metadata = build_figure_metadata(&config);

    Ok(SimulationResult {
        config,
        time_series,
        limb_buffers,
        events,
        summary,
        derived_metrics,
        stability_summary,
        figure_metadata,
    })
}

fn push_records(
    config: &SimulationConfig,
    time_series: &mut Vec<TimeSeriesRecord>,
    limb_buffers: &mut Vec<LimbBufferRecord>,
    state: &SystemState,
    diagnostics: &StepDiagnostics,
    previous_lyapunov: &mut Option<(f64, f64)>,
) {
    let reduced_response_target_y_m =
        reduced_response_target(diagnostics.command_fraction, &config.model);
    let reduced_response_target_rate_mps = 0.0;
    let reduced_response_error_m = state.y_m - reduced_response_target_y_m;
    let reduced_response_error_rate_mps = state.v_mps - reduced_response_target_rate_mps;
    let lyapunov_v = lyapunov_value(
        config.model.mechanical_mass_kg,
        diagnostics.stiffness,
        reduced_response_error_m,
        reduced_response_error_rate_mps,
    );
    let lyapunov_dv_dt = previous_lyapunov
        .map(|(previous_time_s, previous_v)| {
            let dt_s = (state.time_s - previous_time_s).max(1.0e-9);
            (lyapunov_v - previous_v) / dt_s
        })
        .unwrap_or(0.0);
    *previous_lyapunov = Some((state.time_s, lyapunov_v));

    let authority_utilization = normalized_authority_utilization(
        &config.model,
        diagnostics.gain,
        diagnostics.command_fraction,
        diagnostics.delivered_ratio,
    );
    let admissible = admissible_status(
        &config.model,
        state.ep_j,
        state.temperature_k,
        state.y_m,
        state.v_mps,
    );

    time_series.push(TimeSeriesRecord {
        time_s: state.time_s,
        ep_j: state.ep_j,
        ep_gj: state.ep_j / 1.0e9,
        temperature_k: state.temperature_k,
        temperature_c: state.temperature_k - 273.15,
        y_m: state.y_m,
        v_mps: state.v_mps,
        command_fraction: diagnostics.command_fraction,
        active_segment: diagnostics.active_segment.clone(),
        requested_actuator_power_w: diagnostics.requested_actuator_power_w,
        delivered_actuator_power_w: diagnostics.delivered_actuator_power_w,
        central_transfer_power_w: diagnostics.central_transfer_power_w,
        commanded_recharge_power_w: diagnostics.commanded_recharge_power_w,
        parasitic_loss_w: diagnostics.parasitic_loss_w,
        heat_generation_w: diagnostics.heat_generation_w,
        heat_rejection_w: diagnostics.heat_rejection_w,
        gain: diagnostics.gain,
        damping: diagnostics.damping,
        stiffness: diagnostics.stiffness,
        mechanical_force_n: diagnostics.mechanical_force_n,
        acceleration_mps2: diagnostics.acceleration_mps2,
        delivered_ratio: diagnostics.delivered_ratio,
        saturation_fraction: diagnostics.saturation_fraction,
        ep_dot_j_per_s: diagnostics.ep_dot_j_per_s,
        temperature_dot_k_per_s: diagnostics.temperature_dot_k_per_s,
        mechanical_power_w: diagnostics.mechanical_power_w,
        authority_utilization,
        reduced_response_target_y_m,
        reduced_response_target_rate_mps,
        reduced_response_error_m,
        reduced_response_error_rate_mps,
        lyapunov_v,
        lyapunov_dv_dt,
        raw_next_ep_j: diagnostics.raw_next_ep_j,
        raw_next_temperature_k: diagnostics.raw_next_temperature_k,
        raw_next_y_m: diagnostics.raw_next_y_m,
        raw_next_v_mps: diagnostics.raw_next_v_mps,
        ep_clamped: diagnostics.ep_clamped,
        temperature_clamped: diagnostics.temperature_clamped,
        y_clamped: diagnostics.y_clamped,
        ydot_clamped: diagnostics.v_clamped,
        near_admissible_boundary: admissible.near_boundary,
        outside_admissible_region: admissible.outside_region,
        admissible_margin_fraction: admissible.margin_fraction,
    });
    for (index, limb_flow) in diagnostics.limb_flows.iter().enumerate() {
        limb_buffers.push(LimbBufferRecord {
            time_s: state.time_s,
            limb: limb_flow.limb.clone(),
            buffer_energy_j: state.local_buffers_j[index],
            buffer_energy_mj: state.local_buffers_j[index] / 1.0e6,
            requested_power_w: limb_flow.requested_power_w,
            transfer_power_w: limb_flow.transfer_power_w,
            delivered_power_w: limb_flow.delivered_power_w,
            saturation: limb_flow.saturation,
        });
    }
}

fn update_constraint_events(
    config: &SimulationConfig,
    state: &SystemState,
    diagnostics: &StepDiagnostics,
    latch: &mut EventLatch,
    events: &mut Vec<EventRecord>,
) {
    let admissible = admissible_status(
        &config.model,
        state.ep_j,
        state.temperature_k,
        state.y_m,
        state.v_mps,
    );
    let low_energy = state.ep_j < config.model.low_energy_threshold_j;
    let high_temperature = state.temperature_k >= config.model.thermal_limit_k;
    let local_buffer_low = state
        .local_buffers_j
        .iter()
        .any(|energy| *energy < config.model.local_buffer_low_threshold_j);
    let saturated = diagnostics.saturation_fraction > 0.0;
    let admissible_outside = admissible.outside_region;
    let admissible_margin = admissible.near_boundary && !admissible.outside_region;

    transition_event(
        events,
        state.time_s,
        "energy_low",
        "warning",
        "Pulse-layer energy dropped below the configured threshold.",
        state.ep_j,
        config.model.low_energy_threshold_j,
        latch.low_energy,
        low_energy,
    );
    transition_event(
        events,
        state.time_s,
        "thermal_high",
        "failure",
        "Aggregate thermal state exceeded the configured threshold.",
        state.temperature_k,
        config.model.thermal_limit_k,
        latch.high_temperature,
        high_temperature,
    );
    transition_event(
        events,
        state.time_s,
        "local_buffer_low",
        "warning",
        "At least one limb-local energy buffer dropped below the configured threshold.",
        state
            .local_buffers_j
            .iter()
            .copied()
            .fold(f64::INFINITY, f64::min),
        config.model.local_buffer_low_threshold_j,
        latch.local_buffer_low,
        local_buffer_low,
    );
    transition_event(
        events,
        state.time_s,
        "actuator_saturation",
        "warning",
        "Local transfer limits and buffer state reduced delivered actuator power below request.",
        diagnostics.delivered_ratio,
        1.0,
        latch.saturated,
        saturated,
    );
    transition_event(
        events,
        state.time_s,
        "admissible_region_breach",
        "failure",
        "Reduced-order state exited the configured admissible operating region.",
        admissible.margin_fraction,
        0.0,
        latch.admissible_outside,
        admissible_outside,
    );
    transition_event(
        events,
        state.time_s,
        "admissible_margin_warning",
        "warning",
        "Reduced-order state approached the admissible operating region boundary.",
        admissible.margin_fraction,
        0.05,
        latch.admissible_margin,
        admissible_margin,
    );

    latch.low_energy = low_energy;
    latch.high_temperature = high_temperature;
    latch.local_buffer_low = local_buffer_low;
    latch.saturated = saturated;
    latch.admissible_outside = admissible_outside;
    latch.admissible_margin = admissible_margin;
}

fn transition_event(
    events: &mut Vec<EventRecord>,
    time_s: f64,
    event_type: &str,
    severity: &str,
    message: &str,
    value: f64,
    threshold: f64,
    previous: bool,
    current: bool,
) {
    if !previous && current {
        events.push(EventRecord {
            time_s,
            event_type: event_type.to_string(),
            severity: severity.to_string(),
            message: message.to_string(),
            value,
            threshold,
        });
    }
    if previous && !current {
        events.push(EventRecord {
            time_s,
            event_type: format!("{event_type}_recovered"),
            severity: "info".to_string(),
            message: format!("{message} Condition recovered."),
            value,
            threshold,
        });
    }
}
