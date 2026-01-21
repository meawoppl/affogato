use anyhow::{bail, Context, Result};
use colored::Colorize;
use std::process::{Command, Stdio};

use crate::project::Project;

const DEFAULT_IMAGE: &str = "ghcr.io/meawoppl/affogato:latest";

pub struct Docker {
    image: String,
    verbose: bool,
}

impl Docker {
    pub fn new(image: Option<String>, verbose: bool) -> Result<Self> {
        // Check Docker is available
        which::which("docker").context(
            "Docker not found. Please install Docker: https://docs.docker.com/get-docker/",
        )?;

        Ok(Self {
            image: image.unwrap_or_else(|| DEFAULT_IMAGE.to_string()),
            verbose,
        })
    }

    /// Check if image exists locally
    fn image_exists(&self) -> Result<bool> {
        let output = Command::new("docker")
            .args(["image", "inspect", &self.image])
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()?;

        Ok(output.success())
    }

    /// Ensure image is available, pulling if needed
    pub fn ensure_image(&self) -> Result<()> {
        if !self.image_exists()? {
            println!(
                "{}",
                format!("Image {} not found, pulling...", self.image).yellow()
            );
            self.pull()?;
        }
        Ok(())
    }

    /// Pull the container image
    pub fn pull(&self) -> Result<()> {
        println!("{}", format!("==> Pulling {}", self.image).blue().bold());

        let status = Command::new("docker")
            .args(["pull", &self.image])
            .status()
            .context("Failed to run docker pull")?;

        if !status.success() {
            bail!("Failed to pull image: {}", self.image);
        }

        println!("{}", "Pull complete".green());
        Ok(())
    }

    /// Build container locally from Dockerfile
    pub fn build_local(&self) -> Result<()> {
        // Find affogato root (where docker/Dockerfile lives)
        let affogato_root = self.find_affogato_root()?;
        let dockerfile_dir = affogato_root.join("docker");

        if !dockerfile_dir.join("Dockerfile").exists() {
            bail!(
                "Dockerfile not found at {:?}. Are you in the affogato repository?",
                dockerfile_dir
            );
        }

        println!(
            "{}",
            format!("==> Building {} from {:?}", self.image, dockerfile_dir)
                .blue()
                .bold()
        );

        let status = Command::new("docker")
            .args(["build", "-t", &self.image, "."])
            .current_dir(&dockerfile_dir)
            .status()
            .context("Failed to run docker build")?;

        if !status.success() {
            bail!("Docker build failed");
        }

        println!("{}", "Build complete".green());
        Ok(())
    }

    /// Show container info
    pub fn info(&self) -> Result<()> {
        println!("{}", "Affogato Container Info".blue().bold());
        println!("  Image: {}", self.image);

        if self.image_exists()? {
            println!("  Status: {}", "Available locally".green());

            // Get image details
            let output = Command::new("docker")
                .args([
                    "image",
                    "inspect",
                    &self.image,
                    "--format",
                    "{{.Id}} {{.Size}} {{.Created}}",
                ])
                .output()?;

            if output.status.success() {
                let info = String::from_utf8_lossy(&output.stdout);
                let parts: Vec<&str> = info.split_whitespace().collect();
                if parts.len() >= 3 {
                    println!("  ID: {}", &parts[0][7..19]); // Short ID
                    let size: u64 = parts[1].parse().unwrap_or(0);
                    println!("  Size: {:.1} MB", size as f64 / 1_000_000.0);
                }
            }
        } else {
            println!("  Status: {}", "Not pulled yet".yellow());
            println!("  Run: affogato docker pull");
        }

        Ok(())
    }

    /// Run command in container with project mounted
    pub fn run_in_project(
        &self,
        project: &Project,
        cmd: &[&str],
        extra_args: &[String],
        usb: bool,
    ) -> Result<()> {
        let project_root = project
            .root
            .as_ref()
            .context("Not in an Affogato project")?;

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", project_root.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        // Add USB device if requested
        if usb {
            args.push("--device=/dev/ttyACM0".to_string());
            args.push("--privileged".to_string());
        }

        // Add image
        args.push(self.image.clone());

        // Add command
        args.extend(cmd.iter().map(|s| s.to_string()));
        args.extend(extra_args.iter().cloned());

        if self.verbose {
            println!("{}", format!("docker {}", args.join(" ")).dimmed());
        }

        let status = Command::new("docker")
            .args(&args)
            .status()
            .context("Failed to run docker")?;

        if !status.success() {
            bail!("Command failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run command in container and capture output
    pub fn run_in_project_capture(&self, project: &Project, cmd: &[&str]) -> Result<String> {
        let project_root = project
            .root
            .as_ref()
            .context("Not in an Affogato project")?;

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", project_root.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        args.push(self.image.clone());
        args.extend(cmd.iter().map(|s| s.to_string()));

        let output = Command::new("docker")
            .args(&args)
            .output()
            .context("Failed to run docker")?;

        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);

        Ok(format!("{}{}", stdout, stderr))
    }

    /// Run command in container with project and extra mount options
    pub fn run_in_project_with_extra_mounts(
        &self,
        project: &Project,
        cmd: &[&str],
        extra_mounts: &[&str],
        usb: bool,
    ) -> Result<()> {
        let project_root = project
            .root
            .as_ref()
            .context("Not in an Affogato project")?;

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", project_root.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        // Add extra mounts
        for mount in extra_mounts {
            args.push(mount.to_string());
        }

        if usb {
            args.push("--device=/dev/ttyACM0".to_string());
            args.push("--privileged".to_string());
        }

        args.push(self.image.clone());
        args.extend(cmd.iter().map(|s| s.to_string()));

        if self.verbose {
            println!("{}", format!("docker {}", args.join(" ")).dimmed());
        }

        let status = Command::new("docker")
            .args(&args)
            .status()
            .context("Failed to run docker")?;

        if !status.success() {
            bail!("Command failed with exit code: {:?}", status.code());
        }

        Ok(())
    }

    /// Run command in container without project
    pub fn run_standalone(&self, cmd: &[&str], usb: bool) -> Result<()> {
        let cwd = std::env::current_dir()?;

        let mut args = vec![
            "run".to_string(),
            "--rm".to_string(),
            "-it".to_string(),
            "-v".to_string(),
            format!("{}:/workspace", cwd.display()),
            "-w".to_string(),
            "/workspace".to_string(),
        ];

        if usb {
            args.push("--device=/dev/ttyACM0".to_string());
            args.push("--privileged".to_string());
        }

        args.push(self.image.clone());
        args.extend(cmd.iter().map(|s| s.to_string()));

        let status = Command::new("docker")
            .args(&args)
            .status()
            .context("Failed to run docker")?;

        if !status.success() {
            bail!("Command failed");
        }

        Ok(())
    }

    fn find_affogato_root(&self) -> Result<std::path::PathBuf> {
        // Try to find affogato root by looking for docker/Dockerfile
        let mut dir = std::env::current_dir()?;
        loop {
            if dir.join("docker/Dockerfile").exists() && dir.join("components/ice40").exists() {
                return Ok(dir);
            }
            if !dir.pop() {
                break;
            }
        }

        // Try common locations
        if let Some(home) = dirs::home_dir() {
            let candidates = [
                home.join("repos/affogato"),
                home.join("src/affogato"),
                home.join(".affogato"),
            ];
            for candidate in candidates {
                if candidate.join("docker/Dockerfile").exists() {
                    return Ok(candidate);
                }
            }
        }

        bail!("Could not find Affogato installation. Set AFFOGATO_PATH or run from the affogato directory.");
    }
}
