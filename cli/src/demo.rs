use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

use crate::docker::Docker;
use crate::project::Project;

/// Available demos
const DEMOS: &[(&str, &str)] = &[
    ("colorwheel", "RGB LED cycles through colors autonomously"),
    ("web-led", "WiFi color picker controls RGB LED via SPI"),
];

/// List available demos
pub fn list_demos() {
    println!("{}", "Available demos:".blue().bold());
    println!();
    for (name, description) in DEMOS {
        println!("  {:<12} - {}", name.green(), description);
    }
    println!();
    println!("Run a demo with: affogato demo <name>");
}

/// Copy a demo to the current directory and optionally build/run it
pub fn run_demo(
    docker: &Docker,
    name: &str,
    port: &str,
    build_only: bool,
    list: bool,
) -> Result<()> {
    if list {
        list_demos();
        return Ok(());
    }

    // Find the affogato installation to locate examples
    let affogato_path = find_affogato_path()?;
    let demo_src = affogato_path.join("examples").join(name);

    if !demo_src.exists() {
        println!("{}", format!("Demo '{}' not found.", name).red());
        println!();
        list_demos();
        bail!("Unknown demo: {}", name);
    }

    let dest = PathBuf::from(name);

    if dest.exists() {
        println!(
            "{}",
            format!("Directory '{}' already exists. Using existing copy.", name).yellow()
        );
    } else {
        println!(
            "{}",
            format!("==> Copying demo '{}' to ./{}", name, name)
                .blue()
                .bold()
        );
        copy_dir_recursive(&demo_src, &dest)?;
    }

    // Create a project context for the demo directory
    let project = Project {
        root: Some(dest.canonicalize()?),
        name: Some(name.to_string()),
    };

    docker.ensure_image()?;

    // Build the demo
    println!("{}", "==> Building FPGA bitstream".blue().bold());
    docker.run_in_project(&project, &["make", "-C", "fpga"], &[], false)?;

    println!("{}", "==> Building ESP32 firmware".blue().bold());
    // Mount components from the affogato repo
    let components_mount = format!(
        "-v {}:/workspace/components",
        affogato_path.join("components").display()
    );
    docker.run_in_project_with_extra_mounts(
        &project,
        &["bash", "-c", "cd firmware && idf.py build"],
        &[&components_mount],
        false,
    )?;

    if build_only {
        println!("{}", "Build complete!".green());
        println!();
        println!("To flash and run:");
        println!("  cd {}", name);
        println!("  affogato run");
        return Ok(());
    }

    // Flash and monitor
    println!(
        "{}",
        format!("==> Flashing and monitoring on {}", port)
            .blue()
            .bold()
    );
    println!("{}", "Ctrl+] to exit".yellow());

    let flash_cmd = format!("cd firmware && idf.py -p {} flash monitor", port);
    docker.run_in_project_with_extra_mounts(
        &project,
        &["bash", "-c", &flash_cmd],
        &[&components_mount],
        true,
    )?;

    Ok(())
}

/// Find the affogato installation directory
fn find_affogato_path() -> Result<PathBuf> {
    // Check environment variable first
    if let Ok(path) = std::env::var("AFFOGATO_PATH") {
        let p = PathBuf::from(path);
        if p.join("examples").exists() {
            return Ok(p);
        }
    }

    // Check if we're running from within the affogato repo
    let exe_path = std::env::current_exe()?;
    if let Some(parent) = exe_path.parent() {
        // cargo run puts binary in target/debug or target/release
        for ancestor in parent.ancestors() {
            if ancestor.join("examples").exists() && ancestor.join("components").exists() {
                return Ok(ancestor.to_path_buf());
            }
        }
    }

    // Check common installation paths
    let home = dirs::home_dir().unwrap_or_default();
    let candidates = [
        home.join(".affogato"),
        home.join("affogato"),
        PathBuf::from("/usr/share/affogato"),
        PathBuf::from("/usr/local/share/affogato"),
    ];

    for candidate in candidates {
        if candidate.join("examples").exists() {
            return Ok(candidate);
        }
    }

    bail!(
        "Could not find Affogato installation with examples. Set AFFOGATO_PATH environment variable."
    );
}

/// Recursively copy a directory
fn copy_dir_recursive(src: &PathBuf, dest: &PathBuf) -> Result<()> {
    fs::create_dir_all(dest)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}
