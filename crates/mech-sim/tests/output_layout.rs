use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;

use mech_sim::outputs::prepare_run_root;

#[test]
fn prepare_run_root_creates_unique_timestamped_directories() -> Result<()> {
    let mut root = std::env::temp_dir();
    root.push(unique_name("mech-sim-output-test"));
    fs::create_dir_all(&root)?;

    let first = prepare_run_root(&root)?;
    let second = prepare_run_root(&root)?;

    assert!(first.exists());
    assert!(second.exists());
    assert_ne!(first, second);
    assert_eq!(first.parent(), Some(root.as_path()));
    assert_eq!(second.parent(), Some(root.as_path()));

    fs::remove_dir_all(&root)?;
    Ok(())
}

fn unique_name(prefix: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time went backwards")
        .as_nanos();
    PathBuf::from(format!("{prefix}-{nanos}"))
}
