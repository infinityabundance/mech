use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Duration, Local};
use serde::Serialize;

use crate::integrator::SimulationResult;
use crate::plots::{render_run_plots, render_sweep_plots};
use crate::sweep::{SweepAggregate, SweepCaseSummary};

pub fn prepare_run_root(output_root: &Path) -> Result<PathBuf> {
    fs::create_dir_all(output_root)?;
    let base_timestamp = Local::now();

    for offset_s in 0..120 {
        let timestamp = (base_timestamp + Duration::seconds(offset_s))
            .format("%Y-%m-%d_%H-%M-%S")
            .to_string();
        let candidate = output_root.join(timestamp);
        match fs::create_dir(&candidate) {
            Ok(()) => return Ok(candidate),
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error).context("failed to create timestamped run directory"),
        }
    }

    anyhow::bail!("unable to create a unique timestamped output directory within 120 seconds")
}

pub fn write_run_outputs(run_root: &Path, result: &SimulationResult) -> Result<()> {
    fs::create_dir_all(run_root)?;

    write_csv(run_root.join("time_series.csv"), &result.time_series)?;
    write_csv(run_root.join("limb_buffers.csv"), &result.limb_buffers)?;
    write_csv(run_root.join("events.csv"), &result.events)?;
    write_csv(
        run_root.join("derived_metrics.csv"),
        &result.derived_metrics,
    )?;
    write_json(run_root.join("summary.json"), &result.summary)?;
    write_json(
        run_root.join("stability_summary.json"),
        &result.stability_summary,
    )?;
    write_json(
        run_root.join("figure_metadata.json"),
        &result.figure_metadata,
    )?;
    write_json(run_root.join("params.json"), &result.config)?;

    render_run_plots(run_root, result)?;
    Ok(())
}

pub fn write_sweep_outputs(run_root: &Path, aggregate: &SweepAggregate) -> Result<()> {
    write_csv(
        run_root.join("sweep_summary.csv"),
        &aggregate.case_summaries,
    )?;
    write_json(run_root.join("sweep_summary.json"), aggregate)?;
    write_specialized_sweep_outputs(run_root, aggregate)?;
    render_sweep_plots(run_root, aggregate)?;
    Ok(())
}

fn write_csv<T: Serialize>(path: PathBuf, rows: &[T]) -> Result<()> {
    let mut writer = csv::Writer::from_path(&path)?;
    for row in rows {
        writer.serialize(row)?;
    }
    writer.flush()?;
    Ok(())
}

fn write_json<T: Serialize>(path: PathBuf, value: &T) -> Result<()> {
    let mut file = File::create(path)?;
    file.write_all(serde_json::to_string_pretty(value)?.as_bytes())?;
    file.write_all(b"\n")?;
    Ok(())
}

fn write_specialized_sweep_outputs(run_root: &Path, aggregate: &SweepAggregate) -> Result<()> {
    match aggregate.preset.as_str() {
        "thermal-duty-matrix" => {
            let rows: Vec<_> = aggregate
                .case_summaries
                .iter()
                .map(ThermalDutyRow::from_case)
                .collect();
            write_csv(run_root.join("thermal_duty_matrix.csv"), &rows)?;
            write_json(run_root.join("thermal_duty_matrix.json"), &rows)?;
            write_csv(run_root.join("thermal_duty_heatmap.csv"), &rows)?;
        }
        "limb-allocation-comparison" => {
            let rows: Vec<_> = aggregate
                .case_summaries
                .iter()
                .map(LimbAllocationRow::from_case)
                .collect();
            write_csv(run_root.join("limb_allocation_comparison.csv"), &rows)?;
            write_json(run_root.join("limb_allocation_comparison.json"), &rows)?;
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize)]
struct ThermalDutyRow {
    case_id: String,
    thermal_rejection_mw_per_k: f64,
    burst_cadence_s: f64,
    peak_temperature_k: f64,
    time_above_tmax_s: f64,
    recharge_readiness_fraction: f64,
    successful_burst_fraction: f64,
    mean_authority_utilization: f64,
    degraded_state_fraction: f64,
    success: bool,
    output_dir: String,
}

impl ThermalDutyRow {
    fn from_case(case: &SweepCaseSummary) -> Self {
        Self {
            case_id: case.case_id.clone(),
            thermal_rejection_mw_per_k: case.thermal_rejection_mw_per_k,
            burst_cadence_s: case.burst_cadence_s.unwrap_or(0.0),
            peak_temperature_k: case.peak_temperature_k,
            time_above_tmax_s: case.time_above_thermal_threshold_s,
            recharge_readiness_fraction: case.recharge_readiness_fraction,
            successful_burst_fraction: case.successful_burst_fraction,
            mean_authority_utilization: case.mean_authority_utilization,
            degraded_state_fraction: case.degraded_state_fraction,
            success: case.success,
            output_dir: case.output_dir.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
struct LimbAllocationRow {
    case_id: String,
    allocation_strategy: String,
    success: bool,
    first_local_breach_s: Option<f64>,
    first_admissible_breach_s: Option<f64>,
    local_imbalance_max_mj: f64,
    saturation_count: usize,
    mean_delivered_ratio: f64,
    min_local_buffer_mj: f64,
    output_dir: String,
}

impl LimbAllocationRow {
    fn from_case(case: &SweepCaseSummary) -> Self {
        Self {
            case_id: case.case_id.clone(),
            allocation_strategy: case
                .allocation_strategy
                .clone()
                .unwrap_or_else(|| "Unknown".to_string()),
            success: case.success,
            first_local_breach_s: case.first_local_buffer_breach_s,
            first_admissible_breach_s: case.first_admissible_breach_s,
            local_imbalance_max_mj: case.local_imbalance_max_mj,
            saturation_count: case.saturation_count,
            mean_delivered_ratio: case.mean_delivered_ratio,
            min_local_buffer_mj: case.min_local_buffer_mj,
            output_dir: case.output_dir.clone(),
        }
    }
}
