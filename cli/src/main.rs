use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

mod config;
mod demo;
mod docker;
mod project;
mod test;

use docker::Docker;
use project::Project;

/// Affogato - ESP32-S2 + ICE40 FPGA Development Tool
#[derive(Parser)]
#[command(name = "affogato")]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
struct Cli {
    #[command(subcommand)]
    command: Commands,

    /// Docker image to use
    #[arg(long, global = true, env = "AFFOGATO_IMAGE")]
    image: Option<String>,

    /// Verbose output
    #[arg(short, long, global = true)]
    verbose: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Create a new Affogato project
    New {
        /// Project name
        name: String,

        /// Template to use (default: basic)
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// Initialize Affogato in an existing directory
    Init {
        /// Template to use
        #[arg(short, long, default_value = "basic")]
        template: String,
    },

    /// Build FPGA bitstream
    #[command(alias = "build-fpga")]
    Fpga {
        /// Additional arguments passed to make
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Build ESP32 firmware (includes FPGA)
    Build {
        /// Additional arguments passed to idf.py
        #[arg(trailing_var_arg = true)]
        args: Vec<String>,
    },

    /// Flash firmware to device
    Flash {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyACM0")]
        port: String,
    },

    /// Monitor serial output
    Monitor {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyACM0")]
        port: String,
    },

    /// Flash and immediately monitor
    Run {
        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyACM0")]
        port: String,
    },

    /// Run Verilog testbenches
    Test {
        /// Specific test to run (without _tb.v suffix)
        name: Option<String>,

        /// Launch GTKWave to view waveforms
        #[arg(long)]
        view: bool,

        /// FPGA directory (default: fpga)
        #[arg(long, default_value = "fpga")]
        dir: String,
    },

    /// Lint Verilog files
    Lint {
        /// FPGA directory (default: fpga)
        #[arg(long, default_value = "fpga")]
        dir: String,
    },

    /// Open ESP-IDF menuconfig
    Menuconfig,

    /// Clean build artifacts
    Clean {
        /// Full clean including CMake cache
        #[arg(long)]
        full: bool,
    },

    /// Open interactive shell in container
    Shell {
        /// Enable USB device access
        #[arg(long)]
        usb: bool,
    },

    /// Manage Docker container
    Docker {
        #[command(subcommand)]
        command: DockerCommands,
    },

    /// Run a demo project
    Demo {
        /// Demo name (colorwheel, web-led). Omit to list available demos.
        name: Option<String>,

        /// Serial port
        #[arg(short, long, default_value = "/dev/ttyACM0")]
        port: String,

        /// Build only, don't flash
        #[arg(long)]
        build_only: bool,

        /// List available demos
        #[arg(short, long)]
        list: bool,
    },
}

#[derive(Subcommand)]
enum DockerCommands {
    /// Pull latest container image
    Pull,

    /// Build container locally
    Build,

    /// Show container info
    Info,
}

fn main() -> Result<()> {
    let cli = Cli::parse();

    let docker = Docker::new(cli.image, cli.verbose)?;
    let project = Project::detect()?;

    match cli.command {
        Commands::New { name, template } => {
            project::create_new(&name, &template)?;
        }

        Commands::Init { template } => {
            project::init_current(&template)?;
        }

        Commands::Fpga { args } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!("{}", "==> Building FPGA bitstream".blue().bold());
            docker.run_in_project(&project, &["make", "-C", "fpga"], &args, false)?;
        }

        Commands::Build { args } => {
            project.require_project()?;
            docker.ensure_image()?;

            // Build FPGA first
            println!("{}", "==> Building FPGA bitstream".blue().bold());
            docker.run_in_project(&project, &["make", "-C", "fpga"], &[], false)?;

            // Then build firmware
            println!("{}", "==> Building ESP32 firmware".blue().bold());
            let idf_cmd = if args.is_empty() {
                "cd firmware && idf.py build".to_string()
            } else {
                format!("cd firmware && idf.py build {}", args.join(" "))
            };
            docker.run_in_project(&project, &["bash", "-c", &idf_cmd], &[], false)?;
        }

        Commands::Flash { port } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!("{}", format!("==> Flashing to {}", port).blue().bold());
            let cmd = format!("cd firmware && idf.py -p {} flash", port);
            docker.run_in_project(&project, &["bash", "-c", &cmd], &[], true)?;
        }

        Commands::Monitor { port } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!("{}", "Ctrl+] to exit".yellow());
            let cmd = format!("cd firmware && idf.py -p {} monitor", port);
            docker.run_in_project(&project, &["bash", "-c", &cmd], &[], true)?;
        }

        Commands::Run { port } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!(
                "{}",
                format!("==> Flash and monitor on {}", port).blue().bold()
            );
            println!("{}", "Ctrl+] to exit".yellow());
            let cmd = format!("cd firmware && idf.py -p {} flash monitor", port);
            docker.run_in_project(&project, &["bash", "-c", &cmd], &[], true)?;
        }

        Commands::Test { name, view, dir } => {
            project.require_project()?;
            docker.ensure_image()?;

            test::run_tests(&docker, &project, name.as_deref(), view, &dir)?;
        }

        Commands::Lint { dir } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!("{}", "==> Linting Verilog".blue().bold());
            let cmd = format!(
                "find {}/rtl -name '*.v' | xargs verilator --lint-only -Wall 2>&1 || true",
                dir
            );
            docker.run_in_project(&project, &["bash", "-c", &cmd], &[], false)?;
        }

        Commands::Menuconfig => {
            project.require_project()?;
            docker.ensure_image()?;

            docker.run_in_project(
                &project,
                &["bash", "-c", "cd firmware && idf.py menuconfig"],
                &[],
                false,
            )?;
        }

        Commands::Clean { full } => {
            project.require_project()?;
            docker.ensure_image()?;

            println!("{}", "==> Cleaning build artifacts".blue().bold());
            docker.run_in_project(&project, &["make", "-C", "fpga", "clean"], &[], false)?;

            let idf_cmd = if full { "fullclean" } else { "clean" };
            let cmd = format!("cd firmware && idf.py {}", idf_cmd);
            docker.run_in_project(&project, &["bash", "-c", &cmd], &[], false)?;
        }

        Commands::Shell { usb } => {
            docker.ensure_image()?;

            println!("{}", "==> Opening shell in container".blue().bold());
            if project.root.is_some() {
                docker.run_in_project(&project, &["/bin/bash"], &[], usb)?;
            } else {
                docker.run_standalone(&["/bin/bash"], usb)?;
            }
        }

        Commands::Docker { command } => match command {
            DockerCommands::Pull => {
                docker.pull()?;
            }
            DockerCommands::Build => {
                docker.build_local()?;
            }
            DockerCommands::Info => {
                docker.info()?;
            }
        },

        Commands::Demo {
            name,
            port,
            build_only,
            list,
        } => {
            if list || name.is_none() {
                demo::list_demos();
            } else {
                demo::run_demo(&docker, name.as_deref().unwrap(), &port, build_only, false)?;
            }
        }
    }

    Ok(())
}
