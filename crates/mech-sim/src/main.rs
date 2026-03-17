use std::path::PathBuf;

use anyhow::Result;
use clap::{Args, Parser, Subcommand};

use mech_sim::config::{AllocationStrategy, ScenarioOverrides, ScenarioPreset, SweepPreset};
use mech_sim::{run_config_file_with_overrides, run_scenario_preset, run_sweep_preset};

#[derive(Debug, Parser)]
#[command(
    name = "mech-sim",
    about = "Deterministic reduced-order simulation crate for pulse-energy, thermal, and actuator architecture validation.",
    long_about = "mech-sim is a deterministic reduced-order architecture-validation simulator for the paper 'Gigawatt-Class Terrestrial Legged Vehicles: A Nuclear-Thermal, Pulse-Power, and Electrohydraulic Systems Architecture'. It models pulse discharge/recharge, aggregate thermal state, reduced mechanical response, and limb-local buffers, then emits reproducible CSV/JSON/PNG artifacts under output-mech-sim/<timestamp>/.",
    arg_required_else_help = true
)]
struct Cli {
    #[arg(long, global = true, default_value = "output-mech-sim")]
    output_root: PathBuf,
    #[arg(long, global = true, default_value_t = 1)]
    seed: u64,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand)]
enum Commands {
    /// Run a built-in deterministic scenario preset.
    Scenario(ScenarioArgs),
    /// Run the built-in baseline sweep suite.
    Sweep(SweepArgs),
    /// Load a JSON config file for scenario or sweep execution.
    Config(ConfigArgs),
}

#[derive(Debug, Args)]
struct ScenarioArgs {
    #[arg(value_enum)]
    preset: ScenarioPreset,
    #[command(flatten)]
    overrides: OverrideArgs,
}

#[derive(Debug, Args)]
struct SweepArgs {
    #[arg(value_enum)]
    preset: SweepPreset,
    #[command(flatten)]
    overrides: OverrideArgs,
}

#[derive(Debug, Args)]
struct ConfigArgs {
    path: PathBuf,
}

#[derive(Debug, Args, Default)]
struct OverrideArgs {
    #[arg(long, help = "Override continuous recharge power Pc in MW.")]
    pc_mw: Option<f64>,
    #[arg(long, help = "Override pulse storage capacity Ep,max in GJ.")]
    ep_gj: Option<f64>,
    #[arg(long, help = "Override initial pulse-layer energy Ep(0) in GJ.")]
    initial_ep_gj: Option<f64>,
    #[arg(long, help = "Override scenario duration in seconds.")]
    duration_s: Option<f64>,
    #[arg(long, help = "Override fixed solver step size in seconds.")]
    dt_s: Option<f64>,
    #[arg(long, help = "Override thermal rejection coefficient in MW/K.")]
    thermal_rejection_mw_per_k: Option<f64>,
    #[arg(long, help = "Override actuator peak power in MW.")]
    burst_power_mw: Option<f64>,
    #[arg(long, help = "Override the primary burst duration in seconds when a burst segment exists.")]
    burst_duration_s: Option<f64>,
    #[arg(long, help = "Scale scenario command demand before it reaches the actuator model.")]
    actuator_demand_scale: Option<f64>,
    #[arg(long, value_enum, help = "Override limb-local power allocation strategy.")]
    allocation_strategy: Option<AllocationStrategy>,
    #[arg(long, help = "Override per-limb local buffer energy capacity in MJ.")]
    local_buffer_mj: Option<f64>,
    #[arg(long, help = "Scale the nominal mechanical damping coefficient.")]
    damping_scale: Option<f64>,
    #[arg(long, help = "Scale the nominal mechanical stiffness coefficient.")]
    stiffness_scale: Option<f64>,
    #[arg(long, help = "Apply deterministic seed-driven command wobble amplitude.")]
    seeded_command_wobble: Option<f64>,
    #[arg(long, help = "Apply deterministic seed-driven disturbance amplitude in N.")]
    seeded_disturbance_n: Option<f64>,
}

impl OverrideArgs {
    fn into_overrides(self) -> ScenarioOverrides {
        ScenarioOverrides {
            continuous_power_mw: self.pc_mw,
            pulse_energy_gj: self.ep_gj,
            initial_ep_gj: self.initial_ep_gj,
            duration_s: self.duration_s,
            dt_s: self.dt_s,
            thermal_rejection_mw_per_k: self.thermal_rejection_mw_per_k,
            burst_power_mw: self.burst_power_mw,
            burst_duration_s: self.burst_duration_s,
            actuator_demand_scale: self.actuator_demand_scale,
            allocation_strategy: self.allocation_strategy,
            local_buffer_energy_mj: self.local_buffer_mj,
            damping_scale: self.damping_scale,
            stiffness_scale: self.stiffness_scale,
            seeded_command_wobble: self.seeded_command_wobble,
            seeded_disturbance_n: self.seeded_disturbance_n,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let run_root = match cli.command {
        Commands::Scenario(args) => run_scenario_preset(
            args.preset,
            args.overrides.into_overrides(),
            &cli.output_root,
            cli.seed,
        )?,
        Commands::Sweep(args) => run_sweep_preset(
            args.preset,
            args.overrides.into_overrides(),
            &cli.output_root,
            cli.seed,
        )?,
        Commands::Config(args) => run_config_file_with_overrides(
            args.path,
            Some(&cli.output_root),
            Some(cli.seed),
        )?,
    };

    println!("{}", run_root.display());
    Ok(())
}
