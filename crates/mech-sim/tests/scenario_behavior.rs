use anyhow::Result;

use mech_sim::config::{ScenarioOverrides, ScenarioPreset};
use mech_sim::integrator::simulate;
use mech_sim::model::{energy_factor, thermal_factor, ModelParameters};
use mech_sim::scenarios::build_scenario_config;

#[test]
fn recharge_scenario_recharges_in_paper_scale_window() -> Result<()> {
    let config = build_scenario_config(
        ScenarioPreset::Recharge,
        ScenarioOverrides::default(),
        1,
    )?;
    let result = simulate(config)?;

    let recharge_time_s = result.summary.recharge_time_s.expect("recharge time");
    assert!(recharge_time_s > 55.0, "expected recharge time above 55 s, got {recharge_time_s}");
    assert!(recharge_time_s < 68.0, "expected recharge time below 68 s, got {recharge_time_s}");
    assert!(result.summary.final_ep_j >= 2.95e9);
    Ok(())
}

#[test]
fn hover_scenario_hits_thermal_limit_before_energy_limit() -> Result<()> {
    let config = build_scenario_config(ScenarioPreset::Hover, ScenarioOverrides::default(), 1)?;
    let result = simulate(config)?;

    assert!(result.summary.thermal_breach, "hover scenario should breach thermal limit");
    assert!(!result.summary.energy_breach, "hover scenario should remain energy-feasible");
    assert!(result.summary.first_thermal_breach_s.expect("thermal breach time") > 30.0);
    Ok(())
}

#[test]
fn constraint_violation_case_reports_multiple_breach_types() -> Result<()> {
    let config = build_scenario_config(
        ScenarioPreset::ConstraintViolation,
        ScenarioOverrides::default(),
        1,
    )?;
    let result = simulate(config)?;

    assert!(result.summary.energy_breach);
    assert!(result.summary.local_buffer_breach);
    assert!(result.summary.saturation_breach);
    assert!(!result.summary.success);
    Ok(())
}

#[test]
fn gain_terms_degrade_monotonically() {
    let params = ModelParameters::default();
    let high_energy = energy_factor(&params, params.pulse_energy_max_j);
    let low_energy = energy_factor(&params, 0.0);
    let cool = thermal_factor(&params, params.ambient_temperature_k);
    let hot = thermal_factor(&params, params.thermal_limit_k + 10.0);

    assert!(high_energy > low_energy);
    assert!(cool > hot);
}
