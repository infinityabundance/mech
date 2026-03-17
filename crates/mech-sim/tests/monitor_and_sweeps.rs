use anyhow::Result;

use mech_sim::config::{ScenarioOverrides, ScenarioPreset, SweepPreset};
use mech_sim::integrator::simulate;
use mech_sim::scenarios::{build_scenario_config, build_sweep_cases};

#[test]
fn burst_summary_reports_partial_refill_metadata() -> Result<()> {
    let config = build_scenario_config(ScenarioPreset::Burst, ScenarioOverrides::default(), 1)?;
    let result = simulate(config)?;

    assert!(result.summary.energy_depleted_j > 0.0);
    assert!(result.summary.recharge_fraction_of_full_reserve > 0.0);
    assert!(result.summary.ideal_refill_time_s.unwrap_or(0.0) > 0.0);
    assert_eq!(
        result.figure_metadata.y_label,
        "Reduced Maneuver Response y"
    );
    Ok(())
}

#[test]
fn monitor_fields_are_populated_in_time_series() -> Result<()> {
    let config = build_scenario_config(ScenarioPreset::Stress, ScenarioOverrides::default(), 1)?;
    let result = simulate(config)?;
    let record = result.time_series.last().expect("time-series record");

    assert!(record.lyapunov_v >= 0.0);
    assert!(record.authority_utilization >= 0.0);
    assert!(result.summary.ep_clamped_count <= result.time_series.len());
    assert!(result.summary.percent_time_outside_admissible_region >= 0.0);
    Ok(())
}

#[test]
fn new_sweep_presets_build_expected_case_counts() -> Result<()> {
    let thermal_cases = build_sweep_cases(
        SweepPreset::ThermalDutyMatrix,
        ScenarioOverrides::default(),
        1,
    )?;
    let allocation_cases = build_sweep_cases(
        SweepPreset::LimbAllocationComparison,
        ScenarioOverrides::default(),
        1,
    )?;

    assert_eq!(thermal_cases.len(), 20);
    assert_eq!(allocation_cases.len(), 4);
    assert!(
        thermal_cases
            .iter()
            .all(|case| case.metadata.burst_cadence_s.unwrap_or(0.0) > 0.0)
    );
    assert!(
        allocation_cases
            .iter()
            .all(|case| case.metadata.allocation_strategy.is_some())
    );
    Ok(())
}
