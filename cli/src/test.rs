use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use crate::docker::Docker;
use crate::project::Project;

/// Test result with timing information
struct TestResult {
    name: String,
    passed: bool,
    duration: Duration,
    #[allow(dead_code)]
    output: String,
}

/// Run Verilog testbenches using iverilog
pub fn run_tests(
    docker: &Docker,
    project: &Project,
    test_name: Option<&str>,
    view: bool,
    fpga_dir: &str,
    verbose: bool,
    parallel: bool,
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

    let test_count = tests.len();
    println!(
        "{}",
        format!("==> Running {} test(s)", test_count).blue().bold()
    );

    let start_time = Instant::now();
    let results = if parallel && test_count > 1 && test_name.is_none() {
        run_tests_parallel(docker, project, &tests, &rtl_dir, &test_dir, view, verbose)?
    } else {
        run_tests_sequential(docker, project, &tests, &rtl_dir, &test_dir, view, verbose)?
    };

    let total_duration = start_time.elapsed();

    // Print summary
    println!();
    println!("{}", "Test Results:".bold());
    let mut all_passed = true;
    let mut pass_count = 0;

    for result in &results {
        let status = if result.passed {
            pass_count += 1;
            "PASS".green()
        } else {
            all_passed = false;
            "FAIL".red()
        };
        println!(
            "  {:40} {} ({:.2}s)",
            result.name,
            status,
            result.duration.as_secs_f64()
        );
    }

    // Print timing summary
    println!();
    println!(
        "{} {} passed, {} failed in {:.2}s",
        "Summary:".bold(),
        pass_count.to_string().green(),
        (test_count - pass_count).to_string().red(),
        total_duration.as_secs_f64()
    );

    if !all_passed {
        bail!("Some tests failed");
    }

    Ok(())
}

fn run_tests_sequential(
    docker: &Docker,
    project: &Project,
    tests: &[String],
    rtl_dir: &str,
    test_dir: &str,
    view: bool,
    verbose: bool,
) -> Result<Vec<TestResult>> {
    let mut results = Vec::new();

    for test in tests {
        let result = run_single_test(docker, project, test, rtl_dir, test_dir, view, verbose)?;
        results.push(result);
    }

    Ok(results)
}

fn run_tests_parallel(
    docker: &Docker,
    project: &Project,
    tests: &[String],
    rtl_dir: &str,
    test_dir: &str,
    view: bool,
    verbose: bool,
) -> Result<Vec<TestResult>> {
    // Parallel execution would require Docker struct to impl Clone/Send
    // For now, fall back to sequential execution
    println!(
        "{}",
        "Note: Parallel execution not yet implemented, running sequentially".dimmed()
    );
    run_tests_sequential(docker, project, tests, rtl_dir, test_dir, view, verbose)
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
    verbose: bool,
) -> Result<TestResult> {
    if !verbose {
        print!("  Testing {:40} ", test_name);
    } else {
        println!("  {} {}", "Testing".blue(), test_name.bold());
    }

    let start = Instant::now();

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
    let output = docker.run_in_project_capture(project, &["bash", "-c", &script])?;

    let duration = start.elapsed();

    let passed = !output.to_lowercase().contains("error")
        && !output.to_lowercase().contains("fail")
        && output.to_lowercase().contains("pass");

    if verbose {
        // Always show output in verbose mode
        println!("{}", "--- Output ---".dimmed());
        for line in output.lines() {
            println!("    {}", highlight_output(line));
        }
        println!("{}", "--------------".dimmed());
        let status = if passed { "PASS".green() } else { "FAIL".red() };
        println!("  Result: {} ({:.2}s)", status, duration.as_secs_f64());
        println!();
    } else if passed {
        println!("{}", "PASS".green());
    } else {
        println!("{}", "FAIL".red());
        // Print output on failure
        println!("{}", "--- Output ---".dimmed());
        for line in output.lines() {
            println!("    {}", highlight_output(line));
        }
        println!("{}", "--------------".dimmed());
    }

    Ok(TestResult {
        name: test_name.to_string(),
        passed,
        duration,
        output,
    })
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
