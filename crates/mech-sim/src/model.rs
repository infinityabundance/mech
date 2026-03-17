use serde::{Deserialize, Serialize};

use crate::config::{AllocationStrategy, LIMB_COUNT};
use crate::state::{ControlInput, LIMB_NAMES, LimbFlow, StepDiagnostics, SystemState};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelParameters {
    pub ambient_temperature_k: f64,
    pub thermal_initial_k: f64,
    pub thermal_capacity_j_per_k: f64,
    pub thermal_soft_limit_k: f64,
    pub thermal_limit_k: f64,
    pub thermal_rejection_w_per_k: f64,
    pub thermal_rejection_quadratic_w_per_k2: f64,
    pub recharge_efficiency: f64,
    pub continuous_power_w: f64,
    pub pulse_energy_max_j: f64,
    pub pulse_energy_initial_j: f64,
    pub pulse_energy_min_j: f64,
    pub low_energy_threshold_j: f64,
    pub actuator_peak_power_w: f64,
    pub actuator_idle_power_w: f64,
    pub actuator_velocity_power_coeff_w_per_mps: f64,
    pub actuator_position_power_coeff_w_per_m: f64,
    pub actuator_heat_fraction: f64,
    pub transfer_heat_fraction: f64,
    pub loss_idle_w: f64,
    pub loss_storage_coeff_w: f64,
    pub loss_thermal_coeff_w_per_k: f64,
    pub mechanical_mass_kg: f64,
    pub damping_n_s_per_m: f64,
    pub stiffness_n_per_m: f64,
    pub reference_actuator_force_n: f64,
    pub damping_temp_coeff: f64,
    pub stiffness_temp_softening: f64,
    pub stiffness_position_coeff: f64,
    pub min_gain_fraction: f64,
    pub energy_gain_soft_zone_j: f64,
    pub thermal_gain_soft_zone_k: f64,
    pub max_displacement_m: f64,
    pub max_velocity_m_per_s: f64,
    pub local_buffer_count: usize,
    pub local_buffer_energy_max_j: f64,
    pub local_buffer_initial_j: f64,
    pub local_buffer_transfer_limit_w: f64,
    pub local_buffer_low_threshold_j: f64,
    pub local_buffer_recovery_tau_s: f64,
    pub local_buffer_target_fraction: f64,
    pub local_buffer_loss_w: f64,
    pub actuator_demand_scale: f64,
    pub damping_scale: f64,
    pub stiffness_scale: f64,
}

impl Default for ModelParameters {
    fn default() -> Self {
        Self {
            ambient_temperature_k: 293.15,
            thermal_initial_k: 293.15,
            thermal_capacity_j_per_k: 4.0e8,
            thermal_soft_limit_k: 326.15,
            thermal_limit_k: 338.15,
            thermal_rejection_w_per_k: 4.5e6,
            thermal_rejection_quadratic_w_per_k2: 4.0e4,
            recharge_efficiency: 0.97,
            continuous_power_w: 50.0e6,
            pulse_energy_max_j: 4.0e9,
            pulse_energy_initial_j: 4.0e9,
            pulse_energy_min_j: 0.25e9,
            low_energy_threshold_j: 0.60e9,
            actuator_peak_power_w: 1.0e9,
            actuator_idle_power_w: 5.0e6,
            actuator_velocity_power_coeff_w_per_mps: 45.0e6,
            actuator_position_power_coeff_w_per_m: 20.0e6,
            actuator_heat_fraction: 0.33,
            transfer_heat_fraction: 0.02,
            loss_idle_w: 1.5e6,
            loss_storage_coeff_w: 2.4e6,
            loss_thermal_coeff_w_per_k: 0.12e6,
            mechanical_mass_kg: 4.0e5,
            damping_n_s_per_m: 2.4e6,
            stiffness_n_per_m: 7.0e6,
            reference_actuator_force_n: 5.0e6,
            damping_temp_coeff: 1.1,
            stiffness_temp_softening: 0.32,
            stiffness_position_coeff: 0.08,
            min_gain_fraction: 0.22,
            energy_gain_soft_zone_j: 1.2e9,
            thermal_gain_soft_zone_k: 14.0,
            max_displacement_m: 8.0,
            max_velocity_m_per_s: 6.5,
            local_buffer_count: LIMB_COUNT,
            local_buffer_energy_max_j: 180.0e6,
            local_buffer_initial_j: 180.0e6,
            local_buffer_transfer_limit_w: 225.0e6,
            local_buffer_low_threshold_j: 36.0e6,
            local_buffer_recovery_tau_s: 12.0,
            local_buffer_target_fraction: 0.92,
            local_buffer_loss_w: 0.25e6,
            actuator_demand_scale: 1.0,
            damping_scale: 1.0,
            stiffness_scale: 1.0,
        }
    }
}

pub struct StepOutcome {
    pub next_state: SystemState,
    pub diagnostics: StepDiagnostics,
}

pub fn step_state(
    params: &ModelParameters,
    state: &SystemState,
    input: &ControlInput,
    dt_s: f64,
) -> StepOutcome {
    let command_fraction = (input.command_fraction * params.actuator_demand_scale).max(0.0);
    let weights = allocation_weights(input.allocation_strategy);
    let requested_actuator_power_w = actuator_power_draw(params, state, command_fraction);
    let parasitic_loss_w = parasitic_loss(params, state.ep_j, state.temperature_k);

    let target_buffer_j = params.local_buffer_energy_max_j * params.local_buffer_target_fraction;
    let mut preliminary_transfer = [0.0; LIMB_COUNT];
    let mut requested_by_limb = [0.0; LIMB_COUNT];
    let mut recharge_by_limb = [0.0; LIMB_COUNT];
    let mut total_transfer_request_w = 0.0;
    let mut commanded_recharge_power_w = 0.0;

    for index in 0..LIMB_COUNT {
        requested_by_limb[index] = requested_actuator_power_w * weights[index];
        recharge_by_limb[index] = ((target_buffer_j - state.local_buffers_j[index]).max(0.0)
            / params.local_buffer_recovery_tau_s.max(1.0e-6))
        .max(0.0);
        preliminary_transfer[index] = (requested_by_limb[index] + recharge_by_limb[index])
            .min(params.local_buffer_transfer_limit_w);
        total_transfer_request_w += preliminary_transfer[index];
        commanded_recharge_power_w += recharge_by_limb[index];
    }

    let available_central_energy_j =
        (state.ep_j + params.recharge_efficiency * params.continuous_power_w * dt_s)
            - parasitic_loss_w * dt_s;
    let central_scale = if total_transfer_request_w > 0.0 {
        (available_central_energy_j / (total_transfer_request_w * dt_s))
            .clamp(0.0, 1.0)
    } else {
        1.0
    };

    let mut next_local_buffers_j = [0.0; LIMB_COUNT];
    let mut limb_flows: [LimbFlow; LIMB_COUNT] = std::array::from_fn(|index| LimbFlow {
        limb: LIMB_NAMES[index].to_string(),
        requested_power_w: 0.0,
        transfer_power_w: 0.0,
        delivered_power_w: 0.0,
        buffer_energy_before_j: state.local_buffers_j[index],
        buffer_energy_after_j: state.local_buffers_j[index],
        saturation: false,
    });

    let mut central_transfer_power_w = 0.0;
    let mut delivered_actuator_power_w = 0.0;
    let mut saturation_count = 0.0;

    for index in 0..LIMB_COUNT {
        let transfer_power_w = preliminary_transfer[index] * central_scale;
        let available_from_buffer_w = state.local_buffers_j[index] / dt_s.max(1.0e-9);
        let delivered_power_w =
            requested_by_limb[index].min(transfer_power_w + available_from_buffer_w);
        let saturation = delivered_power_w + 1.0 < requested_by_limb[index];
        let next_buffer = (state.local_buffers_j[index]
            + (transfer_power_w - delivered_power_w - params.local_buffer_loss_w) * dt_s)
            .clamp(0.0, params.local_buffer_energy_max_j);

        limb_flows[index] = LimbFlow {
            limb: LIMB_NAMES[index].to_string(),
            requested_power_w: requested_by_limb[index],
            transfer_power_w,
            delivered_power_w,
            buffer_energy_before_j: state.local_buffers_j[index],
            buffer_energy_after_j: next_buffer,
            saturation,
        };
        next_local_buffers_j[index] = next_buffer;
        central_transfer_power_w += transfer_power_w;
        delivered_actuator_power_w += delivered_power_w;
        if saturation {
            saturation_count += 1.0;
        }
    }

    let energy_factor = energy_factor(params, state.ep_j);
    let thermal_factor = thermal_factor(params, state.temperature_k);
    let gain = params.reference_actuator_force_n * energy_factor * thermal_factor;
    let damping = damping(params, state.temperature_k);
    let stiffness = stiffness(params, state.y_m, state.temperature_k);
    let delivered_ratio = if requested_actuator_power_w > 1.0 {
        (delivered_actuator_power_w / requested_actuator_power_w).clamp(0.0, 1.0)
    } else {
        1.0
    };
    let mechanical_force_n = gain * command_fraction * delivered_ratio;
    let acceleration_mps2 = (mechanical_force_n + input.disturbance_n
        - damping * state.v_mps
        - stiffness * state.y_m)
        / params.mechanical_mass_kg.max(1.0);
    let next_v_mps =
        (state.v_mps + acceleration_mps2 * dt_s).clamp(-params.max_velocity_m_per_s, params.max_velocity_m_per_s);
    let next_y_m =
        (state.y_m + next_v_mps * dt_s).clamp(-params.max_displacement_m, params.max_displacement_m);
    let mechanical_power_w = mechanical_force_n * next_v_mps;

    let heat_generation_w = heat_generation(
        params,
        delivered_actuator_power_w,
        central_transfer_power_w,
        parasitic_loss_w,
    );
    let heat_rejection_w = heat_rejection(params, state.temperature_k);
    let ep_dot_j_per_s =
        params.recharge_efficiency * params.continuous_power_w - central_transfer_power_w - parasitic_loss_w;
    let temperature_dot_k_per_s =
        (heat_generation_w - heat_rejection_w) / params.thermal_capacity_j_per_k.max(1.0);

    let next_ep_j = (state.ep_j + ep_dot_j_per_s * dt_s).clamp(0.0, params.pulse_energy_max_j);
    let next_temperature_k =
        (state.temperature_k + temperature_dot_k_per_s * dt_s).max(params.ambient_temperature_k);

    let next_state = SystemState {
        time_s: state.time_s + dt_s,
        ep_j: next_ep_j,
        temperature_k: next_temperature_k,
        y_m: next_y_m,
        v_mps: next_v_mps,
        local_buffers_j: next_local_buffers_j,
    };

    let diagnostics = StepDiagnostics {
        command_fraction,
        disturbance_n: input.disturbance_n,
        active_segment: input.active_segment.clone(),
        allocation_strategy: input.allocation_strategy,
        requested_actuator_power_w,
        delivered_actuator_power_w,
        central_transfer_power_w,
        commanded_recharge_power_w,
        parasitic_loss_w,
        heat_generation_w,
        heat_rejection_w,
        gain,
        damping,
        stiffness,
        mechanical_force_n,
        acceleration_mps2,
        delivered_ratio,
        energy_factor,
        thermal_factor,
        saturation_fraction: saturation_count / LIMB_COUNT as f64,
        ep_dot_j_per_s,
        temperature_dot_k_per_s,
        mechanical_power_w,
        limb_flows,
    };

    StepOutcome {
        next_state,
        diagnostics,
    }
}

pub fn actuator_power_draw(
    params: &ModelParameters,
    state: &SystemState,
    command_fraction: f64,
) -> f64 {
    let command_term = params.actuator_peak_power_w * command_fraction.powf(1.15);
    let kinematic_term = params.actuator_velocity_power_coeff_w_per_mps * state.v_mps.abs()
        + params.actuator_position_power_coeff_w_per_m * state.y_m.abs();
    params.actuator_idle_power_w + command_term + kinematic_term
}

pub fn parasitic_loss(params: &ModelParameters, ep_j: f64, temperature_k: f64) -> f64 {
    let soc = (ep_j / params.pulse_energy_max_j.max(1.0)).clamp(0.0, 1.5);
    let thermal_excess = (temperature_k - params.ambient_temperature_k).max(0.0);
    params.loss_idle_w + params.loss_storage_coeff_w * soc * soc + params.loss_thermal_coeff_w_per_k * thermal_excess
}

pub fn heat_generation(
    params: &ModelParameters,
    delivered_actuator_power_w: f64,
    central_transfer_power_w: f64,
    parasitic_loss_w: f64,
) -> f64 {
    params.actuator_heat_fraction * delivered_actuator_power_w
        + params.transfer_heat_fraction * central_transfer_power_w
        + parasitic_loss_w
}

pub fn heat_rejection(params: &ModelParameters, temperature_k: f64) -> f64 {
    let delta_k = (temperature_k - params.ambient_temperature_k).max(0.0);
    params.thermal_rejection_w_per_k * delta_k
        + params.thermal_rejection_quadratic_w_per_k2 * delta_k * delta_k
}

pub fn damping(params: &ModelParameters, temperature_k: f64) -> f64 {
    let temp_fraction = normalized_temperature_fraction(params, temperature_k);
    params.damping_n_s_per_m * params.damping_scale * (1.0 + params.damping_temp_coeff * temp_fraction)
}

pub fn stiffness(params: &ModelParameters, y_m: f64, temperature_k: f64) -> f64 {
    let temp_fraction = normalized_temperature_fraction(params, temperature_k);
    let thermal_softening = (1.0 - params.stiffness_temp_softening * temp_fraction).max(0.35);
    let geometric_hardening = 1.0 + params.stiffness_position_coeff * y_m * y_m;
    params.stiffness_n_per_m * params.stiffness_scale * thermal_softening * geometric_hardening
}

pub fn energy_factor(params: &ModelParameters, ep_j: f64) -> f64 {
    let normalized = ((ep_j - params.low_energy_threshold_j) / params.energy_gain_soft_zone_j.max(1.0))
        .clamp(0.0, 1.0);
    params.min_gain_fraction + (1.0 - params.min_gain_fraction) * smoothstep01(normalized)
}

pub fn thermal_factor(params: &ModelParameters, temperature_k: f64) -> f64 {
    let normalized = ((params.thermal_limit_k - temperature_k) / params.thermal_gain_soft_zone_k.max(1.0))
        .clamp(0.0, 1.0);
    params.min_gain_fraction + (1.0 - params.min_gain_fraction) * smoothstep01(normalized)
}

fn normalized_temperature_fraction(params: &ModelParameters, temperature_k: f64) -> f64 {
    let numerator = (temperature_k - params.ambient_temperature_k).max(0.0);
    let denominator = (params.thermal_limit_k - params.ambient_temperature_k).max(1.0);
    (numerator / denominator).clamp(0.0, 1.5)
}

fn smoothstep01(x: f64) -> f64 {
    let x = x.clamp(0.0, 1.0);
    x * x * (3.0 - 2.0 * x)
}

fn allocation_weights(strategy: AllocationStrategy) -> [f64; LIMB_COUNT] {
    strategy.normalized_weights()
}
