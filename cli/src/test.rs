use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

use crate::docker::Docker;
use crate::project::Project;

/// Run Verilog testbenches using iverilog
pub fn run_tests(
    docker: &Docker,
    project: &Project,
    test_name: Option<&str>,
    view: bool,
    fpga_dir: &str,
) -> Result<()> {
    let project_root = project.root.as_ref().unwrap();

    // Find test directory - check common patterns
    let test_dirs = [
        format!("{}/rtl_test", fpga_dir),
        format!("{}/test", fpga_dir),
        format!("{}/testbench", fpga_dir),
        format!("{}_test", fpga_dir),
    ];

    let test_dir = test_dirs
        .iter()
        .find(|d| project_root.join(d).exists())
        .map(|d| d.to_string());

    let test_dir = match test_dir {
        Some(d) => d,
        None => {
            println!("{}", "No test directory found. Expected one of:".yellow());
            for d in &test_dirs {
                println!("  - {}", d);
            }
            return Ok(());
        }
    };

    // Find RTL source directory
    let rtl_dir = format!("{}/rtl", fpga_dir);
    if !project_root.join(&rtl_dir).exists() {
        bail!("RTL directory not found: {}", rtl_dir);
    }

    // Discover tests
    let tests = discover_tests(project_root, &test_dir, test_name)?;

    if tests.is_empty() {
        println!("{}", "No tests found".yellow());
        return Ok(());
    }

    println!(
        "{}",
        format!("==> Running {} test(s)", tests.len()).blue().bold()
    );

    let mut results = Vec::new();

    for test in &tests {
        let result = run_single_test(docker, project, test, &rtl_dir, &test_dir, view)?;
        results.push((test.clone(), result));
    }

    // Print summary
    println!();
    println!("{}", "Test Results:".bold());
    let mut all_passed = true;
    for (name, passed) in &results {
        let status = if *passed {
            "PASS".green()
        } else {
            all_passed = false;
            "FAIL".red()
        };
        println!("  {:40} {}", name, status);
    }

    if !all_passed {
        bail!("Some tests failed");
    }

    Ok(())
}

fn discover_tests(
    project_root: &Path,
    test_dir: &str,
    specific: Option<&str>,
) -> Result<Vec<String>> {
    let test_path = project_root.join(test_dir);

    if let Some(name) = specific {
        // Run specific test
        let tb_file = test_path.join(format!("{}_tb.v", name));
        if !tb_file.exists() {
            bail!("Test not found: {}_tb.v", name);
        }
        return Ok(vec![name.to_string()]);
    }

    // Discover all tests
    let mut tests = Vec::new();

    if test_path.exists() {
        for entry in fs::read_dir(&test_path)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().to_string();

            if name.ends_with("_tb.v") {
                let test_name = name.strip_suffix("_tb.v").unwrap().to_string();
                tests.push(test_name);
            }
        }
    }

    tests.sort();
    Ok(tests)
}

fn run_single_test(
    docker: &Docker,
    project: &Project,
    test_name: &str,
    rtl_dir: &str,
    test_dir: &str,
    view: bool,
) -> Result<bool> {
    print!("  Testing {:40} ", test_name);

    // Build the iverilog command that:
    // 1. Compiles all RTL sources + the testbench
    // 2. Runs the simulation
    // 3. Checks for errors in output
    let script = format!(
        r#"
set -e
cd /workspace

# Create temp directory for test
TMPDIR=$(mktemp -d)
trap "rm -rf $TMPDIR" EXIT

# Find all RTL sources
RTL_FILES=$(find {rtl_dir} -name '*.v' | tr '\n' ' ')

# Compile with iverilog
iverilog -g2012 -Wall \
    -DNO_ICE40_DEFAULT_ASSIGNMENTS \
    -s {test_name}_tb \
    -o $TMPDIR/test \
    $RTL_FILES \
    {test_dir}/{test_name}_tb.v \
    2>&1

# Run simulation
cd $TMPDIR
./test 2>&1

# Check for VCD output and optionally view
if [ "{view}" = "true" ]; then
    VCD=$(ls *.vcd 2>/dev/null | head -1 || true)
    if [ -n "$VCD" ]; then
        cp $VCD /workspace/{test_dir}/
        echo "VCD saved to {test_dir}/$VCD"
    fi
fi
"#,
        rtl_dir = rtl_dir,
        test_dir = test_dir,
        test_name = test_name,
        view = view,
    );

    // Run in docker and capture output
    let result = docker.run_in_project_capture(project, &["bash", "-c", &script])?;

    let passed = !result.to_lowercase().contains("error")
        && !result.to_lowercase().contains("fail")
        && result.to_lowercase().contains("pass");

    if passed {
        println!("{}", "PASS".green());
    } else {
        println!("{}", "FAIL".red());
        // Print output on failure
        println!("{}", "--- Output ---".dimmed());
        for line in result.lines() {
            println!("    {}", highlight_output(line));
        }
        println!("{}", "--------------".dimmed());
    }

    Ok(passed)
}

fn highlight_output(line: &str) -> String {
    let lower = line.to_lowercase();

    if lower.contains("error") || lower.contains("fail") {
        line.red().to_string()
    } else if lower.contains("warn") {
        line.yellow().to_string()
    } else if lower.contains("pass") {
        line.green().to_string()
    } else {
        line.to_string()
    }
}
