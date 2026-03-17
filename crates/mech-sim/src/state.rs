use serde::{Deserialize, Serialize};

use crate::config::{AllocationStrategy, LIMB_COUNT};

pub const LIMB_NAMES: [&str; LIMB_COUNT] = ["front_left", "front_right", "rear_left", "rear_right"];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemState {
    pub time_s: f64,
    pub ep_j: f64,
    pub temperature_k: f64,
    pub y_m: f64,
    pub v_mps: f64,
    pub local_buffers_j: [f64; LIMB_COUNT],
}

impl SystemState {
    pub fn new(ep_j: f64, temperature_k: f64, y_m: f64, v_mps: f64, local_buffer_j: f64) -> Self {
        Self {
            time_s: 0.0,
            ep_j,
            temperature_k,
            y_m,
            v_mps,
            local_buffers_j: [local_buffer_j; LIMB_COUNT],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ControlInput {
    pub command_fraction: f64,
    pub disturbance_n: f64,
    pub active_segment: String,
    pub allocation_strategy: AllocationStrategy,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbFlow {
    pub limb: String,
    pub requested_power_w: f64,
    pub transfer_power_w: f64,
    pub delivered_power_w: f64,
    pub buffer_energy_before_j: f64,
    pub buffer_energy_after_j: f64,
    pub saturation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StepDiagnostics {
    pub command_fraction: f64,
    pub disturbance_n: f64,
    pub active_segment: String,
    pub allocation_strategy: AllocationStrategy,
    pub requested_actuator_power_w: f64,
    pub delivered_actuator_power_w: f64,
    pub central_transfer_power_w: f64,
    pub commanded_recharge_power_w: f64,
    pub parasitic_loss_w: f64,
    pub heat_generation_w: f64,
    pub heat_rejection_w: f64,
    pub gain: f64,
    pub damping: f64,
    pub stiffness: f64,
    pub mechanical_force_n: f64,
    pub acceleration_mps2: f64,
    pub delivered_ratio: f64,
    pub energy_factor: f64,
    pub thermal_factor: f64,
    pub saturation_fraction: f64,
    pub ep_dot_j_per_s: f64,
    pub temperature_dot_k_per_s: f64,
    pub mechanical_power_w: f64,
    pub raw_next_ep_j: f64,
    pub raw_next_temperature_k: f64,
    pub raw_next_y_m: f64,
    pub raw_next_v_mps: f64,
    pub ep_clamped: bool,
    pub temperature_clamped: bool,
    pub y_clamped: bool,
    pub v_clamped: bool,
    pub limb_flows: [LimbFlow; LIMB_COUNT],
}

impl StepDiagnostics {
    pub fn zero() -> Self {
        Self {
            command_fraction: 0.0,
            disturbance_n: 0.0,
            active_segment: "initial".to_string(),
            allocation_strategy: AllocationStrategy::Equal,
            requested_actuator_power_w: 0.0,
            delivered_actuator_power_w: 0.0,
            central_transfer_power_w: 0.0,
            commanded_recharge_power_w: 0.0,
            parasitic_loss_w: 0.0,
            heat_generation_w: 0.0,
            heat_rejection_w: 0.0,
            gain: 0.0,
            damping: 0.0,
            stiffness: 0.0,
            mechanical_force_n: 0.0,
            acceleration_mps2: 0.0,
            delivered_ratio: 1.0,
            energy_factor: 1.0,
            thermal_factor: 1.0,
            saturation_fraction: 0.0,
            ep_dot_j_per_s: 0.0,
            temperature_dot_k_per_s: 0.0,
            mechanical_power_w: 0.0,
            raw_next_ep_j: 0.0,
            raw_next_temperature_k: 0.0,
            raw_next_y_m: 0.0,
            raw_next_v_mps: 0.0,
            ep_clamped: false,
            temperature_clamped: false,
            y_clamped: false,
            v_clamped: false,
            limb_flows: std::array::from_fn(|index| LimbFlow {
                limb: LIMB_NAMES[index].to_string(),
                requested_power_w: 0.0,
                transfer_power_w: 0.0,
                delivered_power_w: 0.0,
                buffer_energy_before_j: 0.0,
                buffer_energy_after_j: 0.0,
                saturation: false,
            }),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesRecord {
    pub time_s: f64,
    pub ep_j: f64,
    pub ep_gj: f64,
    pub temperature_k: f64,
    pub temperature_c: f64,
    pub y_m: f64,
    pub v_mps: f64,
    pub command_fraction: f64,
    pub active_segment: String,
    pub requested_actuator_power_w: f64,
    pub delivered_actuator_power_w: f64,
    pub central_transfer_power_w: f64,
    pub commanded_recharge_power_w: f64,
    pub parasitic_loss_w: f64,
    pub heat_generation_w: f64,
    pub heat_rejection_w: f64,
    pub gain: f64,
    pub damping: f64,
    pub stiffness: f64,
    pub mechanical_force_n: f64,
    pub acceleration_mps2: f64,
    pub delivered_ratio: f64,
    pub saturation_fraction: f64,
    pub ep_dot_j_per_s: f64,
    pub temperature_dot_k_per_s: f64,
    pub mechanical_power_w: f64,
    pub authority_utilization: f64,
    pub reduced_response_target_y_m: f64,
    pub reduced_response_target_rate_mps: f64,
    pub reduced_response_error_m: f64,
    pub reduced_response_error_rate_mps: f64,
    pub lyapunov_v: f64,
    pub lyapunov_dv_dt: f64,
    pub raw_next_ep_j: f64,
    pub raw_next_temperature_k: f64,
    pub raw_next_y_m: f64,
    pub raw_next_v_mps: f64,
    pub ep_clamped: bool,
    pub temperature_clamped: bool,
    pub y_clamped: bool,
    pub ydot_clamped: bool,
    pub near_admissible_boundary: bool,
    pub outside_admissible_region: bool,
    pub admissible_margin_fraction: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LimbBufferRecord {
    pub time_s: f64,
    pub limb: String,
    pub buffer_energy_j: f64,
    pub buffer_energy_mj: f64,
    pub requested_power_w: f64,
    pub transfer_power_w: f64,
    pub delivered_power_w: f64,
    pub saturation: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EventRecord {
    pub time_s: f64,
    pub event_type: String,
    pub severity: String,
    pub message: String,
    pub value: f64,
    pub threshold: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DerivedMetricRecord {
    pub metric: String,
    pub value: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Default)]
pub struct EventLatch {
    pub low_energy: bool,
    pub high_temperature: bool,
    pub local_buffer_low: bool,
    pub saturated: bool,
    pub admissible_outside: bool,
    pub admissible_margin: bool,
}
