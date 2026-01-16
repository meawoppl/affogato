use anyhow::{Context, Result};
use colored::Colorize;
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::sync::mpsc::channel;
use std::time::{Duration, Instant};

use crate::docker::Docker;
use crate::project::Project;

/// Run watch mode - rebuild on file changes
pub fn run_watch(docker: &Docker, project: &Project, fpga_only: bool) -> Result<()> {
    let project_root = project
        .root
        .as_ref()
        .context("Not in an Affogato project")?;

    let fpga_dir = project_root.join("fpga");
    let firmware_dir = project_root.join("firmware");

    println!("{}", "==> Starting watch mode".blue().bold());
    println!("Watching for changes in:");
    if fpga_dir.exists() {
        println!("  - fpga/");
    }
    if !fpga_only && firmware_dir.exists() {
        println!("  - firmware/");
    }
    println!();
    println!("{}", "Press Ctrl+C to stop".yellow());
    println!();

    // Initial build
    run_build(docker, project, fpga_only)?;

    // Set up file watcher
    let (tx, rx) = channel();
    let mut watcher = RecommendedWatcher::new(tx, Config::default())?;

    // Watch fpga directory
    if fpga_dir.exists() {
        watcher.watch(&fpga_dir, RecursiveMode::Recursive)?;
    }

    // Watch firmware directory
    if !fpga_only && firmware_dir.exists() {
        watcher.watch(&firmware_dir, RecursiveMode::Recursive)?;
    }

    // Debounce settings
    let debounce_duration = Duration::from_millis(500);
    let mut last_build = Instant::now() - debounce_duration;

    loop {
        match rx.recv() {
            Ok(event) => {
                if let Ok(event) = event {
                    // Skip non-modify events and build artifacts
                    if !should_trigger_rebuild(&event) {
                        continue;
                    }

                    // Debounce rapid changes
                    let now = Instant::now();
                    if now.duration_since(last_build) < debounce_duration {
                        continue;
                    }
                    last_build = now;

                    // Determine what changed
                    let changed_path = event.paths.first();
                    let is_fpga_change = changed_path
                        .map(|p| p.starts_with(&fpga_dir))
                        .unwrap_or(false);

                    println!();
                    if let Some(path) = changed_path {
                        let relative = path.strip_prefix(project_root).unwrap_or(path);
                        println!(
                            "{}",
                            format!("Change detected: {}", relative.display())
                                .yellow()
                                .bold()
                        );
                    }

                    // Run appropriate build
                    if is_fpga_change {
                        if let Err(e) = run_fpga_build(docker, project) {
                            println!("{}", format!("FPGA build failed: {}", e).red());
                        }
                    } else if !fpga_only {
                        if let Err(e) = run_build(docker, project, fpga_only) {
                            println!("{}", format!("Build failed: {}", e).red());
                        }
                    }
                }
            }
            Err(e) => {
                println!("{}", format!("Watch error: {}", e).red());
            }
        }
    }
}

/// Check if this event should trigger a rebuild
fn should_trigger_rebuild(event: &notify::Event) -> bool {
    use notify::EventKind;

    // Only trigger on modifications and creates
    match event.kind {
        EventKind::Modify(_) | EventKind::Create(_) => {}
        _ => return false,
    }

    // Check file extensions
    for path in &event.paths {
        if let Some(ext) = path.extension() {
            let ext = ext.to_string_lossy().to_lowercase();
            // Verilog, C, headers, config files
            if matches!(
                ext.as_str(),
                "v" | "sv" | "vh" | "c" | "h" | "cpp" | "hpp" | "cmake" | "pcf" | "toml" | "txt"
            ) {
                return true;
            }
        }
        // Also check for CMakeLists.txt specifically
        if let Some(name) = path.file_name() {
            if name == "CMakeLists.txt" || name == "Makefile" || name == "Kconfig" {
                return true;
            }
        }
    }

    false
}

/// Run FPGA build only
fn run_fpga_build(docker: &Docker, project: &Project) -> Result<()> {
    println!("{}", "==> Building FPGA bitstream".blue().bold());
    docker.run_in_project(project, &["make", "-C", "fpga"], &[], false)?;
    println!("{}", "FPGA build complete".green());
    Ok(())
}

/// Run full build (FPGA + firmware)
fn run_build(docker: &Docker, project: &Project, fpga_only: bool) -> Result<()> {
    // Build FPGA
    run_fpga_build(docker, project)?;

    if !fpga_only {
        // Build firmware
        println!("{}", "==> Building ESP32 firmware".blue().bold());
        docker.run_in_project(
            project,
            &["bash", "-c", "cd firmware && idf.py build"],
            &[],
            false,
        )?;
        println!("{}", "Firmware build complete".green());
    }

    Ok(())
}
