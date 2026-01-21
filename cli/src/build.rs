use anyhow::{Context, Result};
use std::path::Path;

use crate::docker::Docker;
use crate::project::{Project, ProjectConfig};

/// Build FPGA bitstream using config or Makefile
pub fn build_fpga(docker: &Docker, project: &Project, extra_args: &[String]) -> Result<()> {
    let project_root = project
        .root
        .as_ref()
        .context("Not in an Affogato project")?;

    // Check if there's a Makefile (legacy path) and no config
    if project_root.join("fpga/Makefile").exists() && project.config.is_none() {
        return docker.run_in_project(project, &["make", "-C", "fpga"], extra_args, false);
    }

    // Use affogato.toml config for building
    let config = project
        .config
        .as_ref()
        .context("No affogato.toml found and no fpga/Makefile present")?;

    build_fpga_with_config(docker, project, config)
}

/// Build FPGA using explicit config (used by demos)
pub fn build_fpga_with_config(
    docker: &Docker,
    project: &Project,
    config: &ProjectConfig,
) -> Result<()> {
    let project_root = project
        .root
        .as_ref()
        .context("Not in an Affogato project")?;

    let fpga_config = &config.fpga;

    // Find all Verilog files in fpga/rtl/
    let rtl_dir = project_root.join("fpga/rtl");
    let mut verilog_files = Vec::new();

    if rtl_dir.exists() {
        for entry in std::fs::read_dir(&rtl_dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "v") {
                // Use relative path from project root
                let rel_path = path.strip_prefix(project_root)?;
                verilog_files.push(rel_path.display().to_string());
            }
        }
    }

    // Add third_party verilog files
    let third_party_dir = project_root.join("fpga/third_party");
    if third_party_dir.exists() {
        collect_verilog_files(&third_party_dir, project_root, &mut verilog_files)?;
    }

    // Add any explicitly included paths from config
    for include in &fpga_config.include {
        let include_path = project_root.join(include);
        if include_path.is_dir() {
            collect_verilog_files(&include_path, project_root, &mut verilog_files)?;
        } else if include_path.exists() {
            let rel_path = include_path.strip_prefix(project_root)?;
            verilog_files.push(rel_path.display().to_string());
        }
    }

    if verilog_files.is_empty() {
        anyhow::bail!("No Verilog files found in fpga/rtl/");
    }

    // Determine PCF file
    let pcf_file = fpga_config
        .pcf
        .clone()
        .unwrap_or_else(|| "fpga/project.pcf".to_string());

    // Build the synthesis command
    let verilog_list = verilog_files.join(" ");
    let top = &fpga_config.top;
    let device = &fpga_config.device;
    let package = &fpga_config.package;

    // Full build pipeline: yosys -> nextpnr -> icepack
    let build_cmd = format!(
        r#"set -e
cd /workspace
echo "Synthesizing with Yosys..."
yosys -q -p "synth_ice40 -abc2 -relut -top {top} -json fpga/top.json" {verilog_list}
echo "Place and route with nextpnr..."
nextpnr-ice40 --{device} --package {package} --json fpga/top.json --pcf {pcf_file} --asc fpga/top.asc
echo "Generating bitstream..."
icepack fpga/top.asc fpga/top.bin
echo "FPGA build complete: fpga/top.bin"
"#
    );

    docker.run_in_project(project, &["bash", "-c", &build_cmd], &[], false)
}

/// Recursively collect Verilog files from a directory
fn collect_verilog_files(dir: &Path, project_root: &Path, files: &mut Vec<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_verilog_files(&path, project_root, files)?;
        } else if path.extension().is_some_and(|ext| ext == "v") {
            let rel_path = path.strip_prefix(project_root)?;
            files.push(rel_path.display().to_string());
        }
    }
    Ok(())
}
