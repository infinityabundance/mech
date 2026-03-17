use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use chrono::{Duration, Local};
use serde::Serialize;

use crate::integrator::SimulationResult;
use crate::plots::{render_run_plots, render_sweep_plots};
use crate::sweep::SweepAggregate;

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
    write_csv(run_root.join("derived_metrics.csv"), &result.derived_metrics)?;
    write_json(run_root.join("summary.json"), &result.summary)?;
    write_json(run_root.join("params.json"), &result.config)?;

    render_run_plots(run_root, result)?;
    Ok(())
}

pub fn write_sweep_outputs(run_root: &Path, aggregate: &SweepAggregate) -> Result<()> {
    write_csv(run_root.join("sweep_summary.csv"), &aggregate.case_summaries)?;
    write_json(run_root.join("sweep_summary.json"), aggregate)?;
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
