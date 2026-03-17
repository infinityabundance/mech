use std::fs;
use std::path::Path;

use anyhow::{Context, Result};
use plotters::prelude::*;

use crate::integrator::SimulationResult;
use crate::sweep::{SweepAggregate, SweepCaseSummary};

const SIZE: (u32, u32) = (1400, 900);

pub fn render_run_plots(run_root: &Path, result: &SimulationResult) -> Result<()> {
    let plots_dir = run_root.join("plots");
    fs::create_dir_all(&plots_dir)?;

    let times: Vec<f64> = result.time_series.iter().map(|record| record.time_s).collect();
    let ep_gj: Vec<f64> = result.time_series.iter().map(|record| record.ep_gj).collect();
    let temperature_c: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| record.temperature_c)
        .collect();
    let requested_power_mw: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| record.requested_actuator_power_w / 1.0e6)
        .collect();
    let delivered_power_mw: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| record.delivered_actuator_power_w / 1.0e6)
        .collect();
    let y_m: Vec<f64> = result.time_series.iter().map(|record| record.y_m).collect();
    let ep_fraction: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| record.ep_j / result.config.model.pulse_energy_max_j.max(1.0))
        .collect();
    let temperature_fraction: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| {
            (record.temperature_k - result.config.model.ambient_temperature_k)
                / (result.config.model.thermal_limit_k - result.config.model.ambient_temperature_k).max(1.0)
        })
        .collect();
    let power_fraction: Vec<f64> = result
        .time_series
        .iter()
        .map(|record| record.requested_actuator_power_w / result.config.model.actuator_peak_power_w.max(1.0))
        .collect();

    line_plot(
        &plots_dir.join("ep_vs_time.png"),
        "Pulse-Layer Energy vs Time",
        "Time [s]",
        "Ep [GJ]",
        &times,
        &[("Ep", &ep_gj, color(0x1F, 0x77, 0xB4))],
    )?;
    line_plot(
        &plots_dir.join("thermal_vs_time.png"),
        "Thermal State vs Time",
        "Time [s]",
        "Temperature [C]",
        &times,
        &[("T", &temperature_c, color(0xD6, 0x27, 0x28))],
    )?;
    line_plot(
        &plots_dir.join("actuator_power_vs_time.png"),
        "Actuator Power Draw vs Time",
        "Time [s]",
        "Power [MW]",
        &times,
        &[
            ("Requested", &requested_power_mw, color(0x2C, 0x7B, 0xB6)),
            ("Delivered", &delivered_power_mw, color(0x22, 0x7A, 0x59)),
        ],
    )?;
    line_plot(
        &plots_dir.join("y_vs_time.png"),
        "Reduced Mechanical Output y(t)",
        "Time [s]",
        "y [m]",
        &times,
        &[("y", &y_m, color(0x94, 0x63, 0xA6))],
    )?;
    line_plot(
        &plots_dir.join("recharge_curve.png"),
        "Recharge Curve",
        "Time [s]",
        "Ep [GJ]",
        &times,
        &[("Ep", &ep_gj, color(0x00, 0x8C, 0x95))],
    )?;
    line_plot(
        &plots_dir.join("burst_overlay.png"),
        "Burst Overlay (Normalized)",
        "Time [s]",
        "Normalized State",
        &times,
        &[
            ("Ep / Ep_max", &ep_fraction, color(0x1F, 0x77, 0xB4)),
            ("(T - Tamb) / (Tlim - Tamb)", &temperature_fraction, color(0xD6, 0x27, 0x28)),
            ("P / P_peak", &power_fraction, color(0xFF, 0x7F, 0x0E)),
        ],
    )?;

    let limb_series = limb_series(result);
    line_plot(
        &plots_dir.join("local_limb_buffers.png"),
        "Local Limb Buffer Energies",
        "Time [s]",
        "Buffer Energy [MJ]",
        &times,
        &[
            ("front_left", &limb_series[0], color(0x2C, 0x7B, 0xB6)),
            ("front_right", &limb_series[1], color(0x63, 0x99, 0x40)),
            ("rear_left", &limb_series[2], color(0xD6, 0x27, 0x28)),
            ("rear_right", &limb_series[3], color(0x94, 0x63, 0xA6)),
        ],
    )?;

    phase_plot(
        &plots_dir.join("ep_vs_t_phase.png"),
        "Ep-T Phase Portrait",
        "Ep [GJ]",
        "Temperature [C]",
        &ep_gj,
        &temperature_c,
        color(0x22, 0x7A, 0x59),
    )?;
    phase_plot(
        &plots_dir.join("actuator_draw_vs_ep.png"),
        "Actuator Draw vs Pulse-Layer State",
        "Ep [GJ]",
        "Requested Actuator Power [MW]",
        &ep_gj,
        &requested_power_mw,
        color(0xFF, 0x7F, 0x0E),
    )?;

    Ok(())
}

pub fn render_sweep_plots(run_root: &Path, aggregate: &SweepAggregate) -> Result<()> {
    let plots_dir = run_root.join("plots");
    fs::create_dir_all(&plots_dir)?;

    line_from_cases(
        &plots_dir.join("pc_vs_recharge_time.png"),
        "Pc vs Recharge Time",
        "Continuous Power Pc [MW]",
        "Recharge Time [s]",
        &group_cases(&aggregate.case_summaries, "recharge_pc"),
        |case| case.continuous_power_mw,
        |case| case.recharge_time_s.unwrap_or(0.0),
    )?;
    line_from_cases(
        &plots_dir.join("thermal_rejection_vs_peak_t.png"),
        "Thermal Rejection vs Peak T",
        "Thermal Rejection [MW/K]",
        "Peak Temperature [C]",
        &group_cases(&aggregate.case_summaries, "thermal_rejection"),
        |case| case.thermal_rejection_mw_per_k,
        |case| case.peak_temperature_c,
    )?;
    line_from_cases(
        &plots_dir.join("burst_power_vs_time_to_threshold.png"),
        "Burst Power vs Time to Threshold",
        "Burst Power [MW]",
        "Time to First Threshold [s]",
        &group_cases(&aggregate.case_summaries, "burst_power"),
        |case| case.burst_power_mw,
        |case| case.time_to_any_threshold_s.unwrap_or(90.0),
    )?;
    line_from_cases(
        &plots_dir.join("pulse_energy_vs_duty_cycle.png"),
        "Pulse Storage vs Effective Duty Cycle",
        "Pulse Storage [GJ]",
        "Effective Duty Cycle [-]",
        &group_cases(&aggregate.case_summaries, "pulse_storage"),
        |case| case.pulse_energy_gj,
        |case| case.effective_duty_cycle,
    )?;
    line_from_cases(
        &plots_dir.join("actuator_demand_vs_saturation.png"),
        "Actuator Demand Scale vs Saturation Count",
        "Demand Scale [-]",
        "Saturation Count [steps]",
        &group_cases(&aggregate.case_summaries, "actuator_demand"),
        |case| case.actuator_demand_scale,
        |case| case.saturation_count as f64,
    )?;

    Ok(())
}

fn limb_series(result: &SimulationResult) -> [Vec<f64>; 4] {
    let sample_count = result.time_series.len();
    let mut series = std::array::from_fn(|_| Vec::with_capacity(sample_count));
    for chunk in result.limb_buffers.chunks_exact(4) {
        for (index, record) in chunk.iter().enumerate() {
            series[index].push(record.buffer_energy_mj);
        }
    }
    while series[0].len() < sample_count {
        for limb in &mut series {
            limb.push(0.0);
        }
    }
    series
}

fn line_plot(
    path: &Path,
    title: &str,
    x_label: &str,
    y_label: &str,
    x_values: &[f64],
    series: &[(&str, &[f64], RGBColor)],
) -> Result<()> {
    let drawing_area = BitMapBackend::new(path, SIZE).into_drawing_area();
    drawing_area.fill(&WHITE)?;

    let x_range = axis_range(x_values);
    let y_data: Vec<f64> = series.iter().flat_map(|(_, values, _)| values.iter().copied()).collect();
    let y_range = axis_range(&y_data);

    let mut chart = ChartBuilder::on(&drawing_area)
        .caption(title, ("sans-serif", 32))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(x_range.0..x_range.1, y_range.0..y_range.1)
        .context("failed to build chart")?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .light_line_style(WHITE.mix(0.15))
        .draw()?;

    for (label, values, series_color) in series {
        let points = x_values.iter().copied().zip(values.iter().copied());
        chart
            .draw_series(LineSeries::new(points, *series_color))?
            .label(*label)
            .legend({
                let legend_color = *series_color;
                move |(x, y)| PathElement::new(vec![(x, y), (x + 20, y)], legend_color)
            });
    }

    chart
        .configure_series_labels()
        .background_style(WHITE.mix(0.85))
        .border_style(BLACK)
        .draw()?;

    drawing_area.present()?;
    Ok(())
}

fn phase_plot(
    path: &Path,
    title: &str,
    x_label: &str,
    y_label: &str,
    x_values: &[f64],
    y_values: &[f64],
    series_color: RGBColor,
) -> Result<()> {
    let drawing_area = BitMapBackend::new(path, SIZE).into_drawing_area();
    drawing_area.fill(&WHITE)?;
    let x_range = axis_range(x_values);
    let y_range = axis_range(y_values);

    let mut chart = ChartBuilder::on(&drawing_area)
        .caption(title, ("sans-serif", 32))
        .margin(20)
        .x_label_area_size(50)
        .y_label_area_size(70)
        .build_cartesian_2d(x_range.0..x_range.1, y_range.0..y_range.1)?;

    chart
        .configure_mesh()
        .x_desc(x_label)
        .y_desc(y_label)
        .light_line_style(WHITE.mix(0.15))
        .draw()?;

    chart.draw_series(LineSeries::new(
        x_values.iter().copied().zip(y_values.iter().copied()),
        series_color,
    ))?;

    drawing_area.present()?;
    Ok(())
}

fn line_from_cases(
    path: &Path,
    title: &str,
    x_label: &str,
    y_label: &str,
    cases: &[SweepCaseSummary],
    x_fn: impl Fn(&SweepCaseSummary) -> f64,
    y_fn: impl Fn(&SweepCaseSummary) -> f64,
) -> Result<()> {
    if cases.is_empty() {
        return Ok(());
    }
    let mut sorted = cases.to_vec();
    sorted.sort_by(|left, right| x_fn(left).partial_cmp(&x_fn(right)).unwrap_or(std::cmp::Ordering::Equal));
    let x_values: Vec<f64> = sorted.iter().map(&x_fn).collect();
    let y_values: Vec<f64> = sorted.iter().map(&y_fn).collect();
    line_plot(
        path,
        title,
        x_label,
        y_label,
        &x_values,
        &[("summary", &y_values, color(0x2C, 0x7B, 0xB6))],
    )
}

fn group_cases<'a>(cases: &'a [SweepCaseSummary], group: &str) -> Vec<SweepCaseSummary> {
    cases
        .iter()
        .filter(|case| case.group == group)
        .cloned()
        .collect()
}

fn axis_range(values: &[f64]) -> (f64, f64) {
    if values.is_empty() {
        return (0.0, 1.0);
    }
    let min = values.iter().copied().fold(f64::INFINITY, f64::min);
    let max = values.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    if (max - min).abs() < 1.0e-9 {
        let padding = if max.abs() < 1.0 { 1.0 } else { max.abs() * 0.1 };
        return (min - padding, max + padding);
    }
    let padding = (max - min) * 0.08;
    (min - padding, max + padding)
}

fn color(r: u8, g: u8, b: u8) -> RGBColor {
    RGBColor(r, g, b)
}
