use anyhow::Result;

use crate::config::{
    AllocationStrategy, ScenarioOverrides, ScenarioPreset, ScenarioProfile, ScenarioSegment,
    SimulationConfig, SolverConfig, SweepCase, SweepCaseMetadata, SweepPreset,
};
use crate::model::ModelParameters;
use crate::state::ControlInput;

pub fn build_scenario_config(
    preset: ScenarioPreset,
    overrides: ScenarioOverrides,
    seed: u64,
) -> Result<SimulationConfig> {
    let mut config = match preset {
        ScenarioPreset::Burst => burst_config(seed),
        ScenarioPreset::Recharge => recharge_config(seed),
        ScenarioPreset::DutyCycle => duty_cycle_config(seed),
        ScenarioPreset::Hover => hover_config(seed),
        ScenarioPreset::Stress => stress_config(seed),
        ScenarioPreset::ConstraintViolation => constraint_violation_config(seed),
    };
    config.apply_overrides(&overrides)?;
    config.validate()?;
    Ok(config)
}

pub fn build_sweep_cases(
    preset: SweepPreset,
    overrides: ScenarioOverrides,
    seed: u64,
) -> Result<Vec<SweepCase>> {
    let mut cases = match preset {
        SweepPreset::Baseline => baseline_sweep_cases(seed)?,
    };
    if !is_noop_overrides(&overrides) {
        for case in &mut cases {
            case.config.apply_overrides(&overrides)?;
            refresh_case_metadata(case);
        }
    }
    Ok(cases)
}

pub fn sample_control(profile: &ScenarioProfile, seed: u64, time_s: f64) -> ControlInput {
    let mut command_fraction = profile.idle_command;
    let mut disturbance_n = 0.0;
    let mut active_segment = "idle".to_string();
    let mut allocation_strategy = profile.baseline_allocation;
    let mut best_demand = profile.idle_command;

    for segment in &profile.segments {
        if time_s >= segment.start_s && time_s < segment.end_s && segment.demand_fraction >= best_demand {
            best_demand = segment.demand_fraction;
            command_fraction = segment.demand_fraction;
            disturbance_n = segment.disturbance_n;
            active_segment = segment.label.clone();
            allocation_strategy = segment
                .allocation_strategy
                .unwrap_or(profile.baseline_allocation);
        }
    }

    if profile.seeded_command_wobble > 0.0 {
        let wobble = centered_hash(seed, time_s, 17);
        command_fraction *= 1.0 + profile.seeded_command_wobble * wobble;
    }
    if profile.seeded_disturbance_n.abs() > 0.0 {
        disturbance_n += profile.seeded_disturbance_n * centered_hash(seed, time_s, 97);
    }

    ControlInput {
        command_fraction: command_fraction.max(0.0),
        disturbance_n,
        active_segment,
        allocation_strategy,
    }
}

fn burst_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 4.0e9,
        pulse_energy_initial_j: 4.0e9,
        continuous_power_w: 50.0e6,
        actuator_peak_power_w: 1.0e9,
        ..ModelParameters::default()
    };
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::Burst,
        name: "burst".to_string(),
        description: "One 1 GW-class, 1 second pulse followed by recharge and thermal decay.".to_string(),
        idle_command: 0.0,
        baseline_allocation: AllocationStrategy::Equal,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments: vec![ScenarioSegment {
            label: "burst_1gw".to_string(),
            start_s: 5.0,
            end_s: 6.0,
            demand_fraction: 1.0,
            disturbance_n: 0.0,
            allocation_strategy: Some(AllocationStrategy::Equal),
        }],
    };
    make_config(
        "burst",
        "Reference single-burst proof-of-life case for pulse discharge, thermal rise, and recharge tail.",
        seed,
        0.02,
        90.0,
        model,
        scenario,
    )
}

fn recharge_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 3.0e9,
        pulse_energy_initial_j: 0.0,
        pulse_energy_min_j: 0.0,
        low_energy_threshold_j: 0.0,
        continuous_power_w: 50.0e6,
        recharge_efficiency: 1.0,
        actuator_peak_power_w: 1.0e9,
        loss_idle_w: 0.15e6,
        loss_storage_coeff_w: 0.10e6,
        loss_thermal_coeff_w_per_k: 0.02e6,
        local_buffer_initial_j: 150.0e6,
        local_buffer_energy_max_j: 150.0e6,
        local_buffer_low_threshold_j: 30.0e6,
        ..ModelParameters::default()
    };
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::Recharge,
        name: "recharge".to_string(),
        description: "No maneuver demand; central pulse store refills from a 50 MW continuous power source.".to_string(),
        idle_command: 0.0,
        baseline_allocation: AllocationStrategy::Equal,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments: vec![],
    };
    make_config(
        "recharge",
        "3 GJ recharge interpretation case with approximately 60 second refill behavior.",
        seed,
        0.05,
        70.0,
        model,
        scenario,
    )
}

fn duty_cycle_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 4.5e9,
        pulse_energy_initial_j: 4.2e9,
        continuous_power_w: 50.0e6,
        actuator_peak_power_w: 0.95e9,
        ..ModelParameters::default()
    };
    let segments = vec![
        burst_segment("burst_a", 4.0, 5.0, 0.95, AllocationStrategy::Equal),
        burst_segment("burst_b", 14.0, 15.0, 0.90, AllocationStrategy::DiagonalBias),
        burst_segment("burst_c", 24.0, 25.0, 0.92, AllocationStrategy::Equal),
        burst_segment("burst_d", 38.0, 39.0, 0.88, AllocationStrategy::FrontBiased),
        burst_segment("burst_e", 52.0, 53.0, 0.94, AllocationStrategy::DiagonalBias),
        burst_segment("burst_f", 68.0, 69.0, 0.90, AllocationStrategy::Equal),
        burst_segment("burst_g", 82.0, 83.0, 0.92, AllocationStrategy::RearBiased),
    ];
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::DutyCycle,
        name: "duty-cycle".to_string(),
        description: "Repeated burst / coast / recharge pattern that exposes effective duty-cycle limits.".to_string(),
        idle_command: 0.02,
        baseline_allocation: AllocationStrategy::Equal,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments,
    };
    make_config(
        "duty-cycle",
        "Alternating maneuver bursts with recharge gaps to quantify duty-cycle-limited behavior.",
        seed,
        0.02,
        95.0,
        model,
        scenario,
    )
}

fn hover_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 18.0e9,
        pulse_energy_initial_j: 18.0e9,
        low_energy_threshold_j: 1.5e9,
        continuous_power_w: 50.0e6,
        actuator_peak_power_w: 1.0e9,
        thermal_capacity_j_per_k: 1.0e8,
        thermal_soft_limit_k: 324.15,
        thermal_limit_k: 336.15,
        thermal_rejection_w_per_k: 2.5e6,
        thermal_rejection_quadratic_w_per_k2: 2.0e4,
        actuator_heat_fraction: 0.45,
        transfer_heat_fraction: 0.04,
        local_buffer_energy_max_j: 250.0e6,
        local_buffer_initial_j: 250.0e6,
        local_buffer_transfer_limit_w: 320.0e6,
        local_buffer_low_threshold_j: 50.0e6,
        local_buffer_recovery_tau_s: 5.0,
        ..ModelParameters::default()
    };
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::Hover,
        name: "hover".to_string(),
        description: "Hover-equivalent sustained maneuver demand in the 350-500 MW window.".to_string(),
        idle_command: 0.0,
        baseline_allocation: AllocationStrategy::Equal,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments: vec![ScenarioSegment {
            label: "sustained_hover".to_string(),
            start_s: 5.0,
            end_s: 48.0,
            demand_fraction: 0.42,
            disturbance_n: 0.0,
            allocation_strategy: Some(AllocationStrategy::Equal),
        }],
    };
    make_config(
        "hover",
        "Sustained high-power maneuver that exposes the thermal state as the dominant limiter.",
        seed,
        0.05,
        65.0,
        model,
        scenario,
    )
}

fn stress_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 3.5e9,
        pulse_energy_initial_j: 3.3e9,
        continuous_power_w: 42.0e6,
        actuator_peak_power_w: 0.95e9,
        local_buffer_energy_max_j: 120.0e6,
        local_buffer_initial_j: 120.0e6,
        local_buffer_transfer_limit_w: 170.0e6,
        local_buffer_low_threshold_j: 24.0e6,
        ..ModelParameters::default()
    };
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::Stress,
        name: "stress".to_string(),
        description: "High limb-force request with biased power allocation to stress local buffers and gain degradation.".to_string(),
        idle_command: 0.02,
        baseline_allocation: AllocationStrategy::FrontBiased,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments: vec![
            ScenarioSegment {
                label: "front_loaded_push".to_string(),
                start_s: 3.0,
                end_s: 18.0,
                demand_fraction: 0.82,
                disturbance_n: 0.0,
                allocation_strategy: Some(AllocationStrategy::FrontBiased),
            },
            ScenarioSegment {
                label: "diagonal_recover".to_string(),
                start_s: 22.0,
                end_s: 30.0,
                demand_fraction: 0.95,
                disturbance_n: 0.0,
                allocation_strategy: Some(AllocationStrategy::DiagonalBias),
            },
        ],
    };
    make_config(
        "stress",
        "Actuator stress-test with local-buffer depletion, central support lag, and degraded actuation gain.",
        seed,
        0.02,
        45.0,
        model,
        scenario,
    )
}

fn constraint_violation_config(seed: u64) -> SimulationConfig {
    let model = ModelParameters {
        pulse_energy_max_j: 1.6e9,
        pulse_energy_initial_j: 1.4e9,
        pulse_energy_min_j: 0.15e9,
        low_energy_threshold_j: 0.28e9,
        continuous_power_w: 25.0e6,
        actuator_peak_power_w: 1.0e9,
        thermal_rejection_w_per_k: 2.4e6,
        local_buffer_energy_max_j: 80.0e6,
        local_buffer_initial_j: 80.0e6,
        local_buffer_transfer_limit_w: 135.0e6,
        local_buffer_low_threshold_j: 16.0e6,
        ..ModelParameters::default()
    };
    let scenario = ScenarioProfile {
        preset: ScenarioPreset::ConstraintViolation,
        name: "constraint-violation".to_string(),
        description: "Insufficient continuous power, weak thermal rejection, and excessive actuator demand to trigger explicit breaches.".to_string(),
        idle_command: 0.04,
        baseline_allocation: AllocationStrategy::FrontBiased,
        seeded_command_wobble: 0.0,
        seeded_disturbance_n: 0.0,
        segments: vec![
            ScenarioSegment {
                label: "overdriven_burst".to_string(),
                start_s: 3.0,
                end_s: 15.0,
                demand_fraction: 1.10,
                disturbance_n: 0.0,
                allocation_strategy: Some(AllocationStrategy::FrontBiased),
            },
            ScenarioSegment {
                label: "rear_loaded_recovery_failure".to_string(),
                start_s: 20.0,
                end_s: 32.0,
                demand_fraction: 0.95,
                disturbance_n: 0.0,
                allocation_strategy: Some(AllocationStrategy::RearBiased),
            },
        ],
    };
    make_config(
        "constraint-violation",
        "Explicit failure case that emits low-energy, thermal, local-buffer, and saturation flags.",
        seed,
        0.02,
        40.0,
        model,
        scenario,
    )
}

fn baseline_sweep_cases(seed: u64) -> Result<Vec<SweepCase>> {
    let mut cases = Vec::new();

    for pc_mw in [40.0, 45.0, 50.0, 55.0, 60.0] {
        let config = build_scenario_config(
            ScenarioPreset::Recharge,
            ScenarioOverrides {
                continuous_power_mw: Some(pc_mw),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "recharge_pc",
            format!("recharge_pc_{pc_mw:.0}mw"),
            format!("Recharge case with Pc = {pc_mw:.0} MW"),
            config,
        ));
    }

    for burst_power_mw in [800.0, 900.0, 1000.0] {
        let config = build_scenario_config(
            ScenarioPreset::Burst,
            ScenarioOverrides {
                burst_power_mw: Some(burst_power_mw),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "burst_power",
            format!("burst_power_{burst_power_mw:.0}mw"),
            format!("Single-burst case with actuator peak power = {burst_power_mw:.0} MW"),
            config,
        ));
    }

    for burst_duration_s in [0.5, 1.0, 1.5] {
        let config = build_scenario_config(
            ScenarioPreset::Burst,
            ScenarioOverrides {
                burst_duration_s: Some(burst_duration_s),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "burst_duration",
            format!("burst_duration_{burst_duration_s:.1}s"),
            format!("Burst duration sensitivity case with {burst_duration_s:.1} second pulse"),
            config,
        ));
    }

    for pulse_energy_gj in [1.0, 3.0, 5.0, 8.0, 10.0, 12.0] {
        let config = build_scenario_config(
            ScenarioPreset::DutyCycle,
            ScenarioOverrides {
                pulse_energy_gj: Some(pulse_energy_gj),
                initial_ep_gj: Some(pulse_energy_gj),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "pulse_storage",
            format!("pulse_storage_{pulse_energy_gj:.0}gj"),
            format!("Pulse-storage reserve sensitivity at {pulse_energy_gj:.0} GJ"),
            config,
        ));
    }

    for rejection_mw_per_k in [3.0, 4.0, 5.0, 6.0, 7.0] {
        let config = build_scenario_config(
            ScenarioPreset::Hover,
            ScenarioOverrides {
                thermal_rejection_mw_per_k: Some(rejection_mw_per_k),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "thermal_rejection",
            format!("thermal_rejection_{rejection_mw_per_k:.0}mw_per_k"),
            format!("Thermal rejection sensitivity at {rejection_mw_per_k:.0} MW/K"),
            config,
        ));
    }

    for demand_scale in [0.8, 0.9, 1.0, 1.1, 1.2] {
        let config = build_scenario_config(
            ScenarioPreset::Stress,
            ScenarioOverrides {
                actuator_demand_scale: Some(demand_scale),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "actuator_demand",
            format!("actuator_demand_{demand_scale:.1}x"),
            format!("Actuator demand sensitivity at {demand_scale:.1}x command scale"),
            config,
        ));
    }

    for (damping_scale, stiffness_scale) in [(0.8, 0.8), (0.8, 1.2), (1.0, 1.0), (1.2, 0.8), (1.2, 1.2)] {
        let config = build_scenario_config(
            ScenarioPreset::DutyCycle,
            ScenarioOverrides {
                damping_scale: Some(damping_scale),
                stiffness_scale: Some(stiffness_scale),
                ..ScenarioOverrides::default()
            },
            seed,
        )?;
        cases.push(case(
            "mechanical_tuning",
            format!("mechanical_tuning_d{damping_scale:.1}_k{stiffness_scale:.1}"),
            format!("Mechanical response tuning with damping {damping_scale:.1}x and stiffness {stiffness_scale:.1}x"),
            config,
        ));
    }

    Ok(cases)
}

fn case(group: &str, case_id: String, note: String, config: SimulationConfig) -> SweepCase {
    let metadata = SweepCaseMetadata {
        case_id,
        group: group.to_string(),
        note,
        continuous_power_mw: config.model.continuous_power_w / 1.0e6,
        burst_power_mw: config.model.actuator_peak_power_w / 1.0e6,
        burst_duration_s: primary_burst_duration(&config.scenario),
        pulse_energy_gj: config.model.pulse_energy_max_j / 1.0e9,
        initial_ep_gj: config.model.pulse_energy_initial_j / 1.0e9,
        thermal_rejection_mw_per_k: config.model.thermal_rejection_w_per_k / 1.0e6,
        actuator_demand_scale: config.model.actuator_demand_scale,
        damping_scale: config.model.damping_scale,
        stiffness_scale: config.model.stiffness_scale,
    };
    SweepCase { metadata, config }
}

fn make_config(
    name: &str,
    description: &str,
    seed: u64,
    dt_s: f64,
    duration_s: f64,
    model: ModelParameters,
    scenario: ScenarioProfile,
) -> SimulationConfig {
    SimulationConfig {
        name: name.to_string(),
        description: description.to_string(),
        seed,
        solver: SolverConfig {
            dt_s,
            duration_s,
            integrator: crate::config::IntegratorKind::SemiImplicitEuler,
        },
        model,
        scenario,
    }
}

fn burst_segment(
    label: &str,
    start_s: f64,
    end_s: f64,
    demand_fraction: f64,
    allocation: AllocationStrategy,
) -> ScenarioSegment {
    ScenarioSegment {
        label: label.to_string(),
        start_s,
        end_s,
        demand_fraction,
        disturbance_n: 0.0,
        allocation_strategy: Some(allocation),
    }
}

fn centered_hash(seed: u64, time_s: f64, salt: u64) -> f64 {
    let bucket = (time_s / 0.5).floor() as u64;
    let mut x = seed ^ salt.wrapping_mul(0x9E3779B97F4A7C15);
    x ^= bucket.wrapping_mul(0xBF58476D1CE4E5B9);
    x = x.wrapping_mul(0x94D049BB133111EB).rotate_left(17);
    let fraction = ((x >> 11) as f64) / ((1u64 << 53) as f64);
    2.0 * fraction - 1.0
}

fn is_noop_overrides(overrides: &ScenarioOverrides) -> bool {
    overrides.continuous_power_mw.is_none()
        && overrides.pulse_energy_gj.is_none()
        && overrides.initial_ep_gj.is_none()
        && overrides.duration_s.is_none()
        && overrides.dt_s.is_none()
        && overrides.thermal_rejection_mw_per_k.is_none()
        && overrides.burst_power_mw.is_none()
        && overrides.burst_duration_s.is_none()
        && overrides.actuator_demand_scale.is_none()
        && overrides.allocation_strategy.is_none()
        && overrides.local_buffer_energy_mj.is_none()
        && overrides.damping_scale.is_none()
        && overrides.stiffness_scale.is_none()
        && overrides.seeded_command_wobble.is_none()
        && overrides.seeded_disturbance_n.is_none()
}

fn refresh_case_metadata(case: &mut SweepCase) {
    case.metadata.continuous_power_mw = case.config.model.continuous_power_w / 1.0e6;
    case.metadata.burst_power_mw = case.config.model.actuator_peak_power_w / 1.0e6;
    case.metadata.burst_duration_s = primary_burst_duration(&case.config.scenario);
    case.metadata.pulse_energy_gj = case.config.model.pulse_energy_max_j / 1.0e9;
    case.metadata.initial_ep_gj = case.config.model.pulse_energy_initial_j / 1.0e9;
    case.metadata.thermal_rejection_mw_per_k = case.config.model.thermal_rejection_w_per_k / 1.0e6;
    case.metadata.actuator_demand_scale = case.config.model.actuator_demand_scale;
    case.metadata.damping_scale = case.config.model.damping_scale;
    case.metadata.stiffness_scale = case.config.model.stiffness_scale;
}

fn primary_burst_duration(profile: &ScenarioProfile) -> f64 {
    profile
        .segments
        .iter()
        .filter(|segment| segment.label.contains("burst"))
        .map(|segment| segment.end_s - segment.start_s)
        .fold(0.0, f64::max)
}
