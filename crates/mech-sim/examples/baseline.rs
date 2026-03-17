use anyhow::Result;

use mech_sim::config::{ScenarioOverrides, ScenarioPreset};
use mech_sim::run_scenario_preset;

fn main() -> Result<()> {
    let output_dir = run_scenario_preset(
        ScenarioPreset::Burst,
        ScenarioOverrides::default(),
        "output-mech-sim",
        1,
    )?;
    println!("{}", output_dir.display());
    Ok(())
}
