use anyhow::{bail, Result};
use colored::Colorize;
use serde::Deserialize;
use std::fs;
use std::path::{Path, PathBuf};

/// Project configuration from affogato.toml
#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectConfig {
    #[serde(default)]
    pub project: ProjectSection,
    #[serde(default)]
    pub fpga: FpgaConfig,
    #[allow(dead_code)]
    #[serde(default)]
    pub firmware: FirmwareConfig,
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct ProjectSection {
    #[serde(default)]
    pub name: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct FpgaConfig {
    #[serde(default = "default_device")]
    pub device: String,
    #[serde(default = "default_package")]
    pub package: String,
    #[serde(default = "default_top")]
    pub top: String,
    #[serde(default)]
    pub pcf: Option<String>,
    /// Additional Verilog files/directories to include
    #[serde(default)]
    pub include: Vec<String>,
}

fn default_device() -> String {
    "up5k".to_string()
}

fn default_package() -> String {
    "sg48".to_string()
}

fn default_top() -> String {
    "top".to_string()
}

impl Default for FpgaConfig {
    fn default() -> Self {
        Self {
            device: default_device(),
            package: default_package(),
            top: default_top(),
            pcf: None,
            include: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Deserialize, Default)]
pub struct FirmwareConfig {
    #[allow(dead_code)]
    #[serde(default)]
    pub project_name: Option<String>,
}

impl ProjectConfig {
    /// Load project config from affogato.toml
    pub fn load(project_root: &Path) -> Result<Self> {
        let config_path = project_root.join("affogato.toml");
        if config_path.exists() {
            let content = fs::read_to_string(&config_path)?;
            Ok(toml::from_str(&content)?)
        } else {
            Ok(Self::default())
        }
    }
}

pub struct Project {
    pub root: Option<PathBuf>,
    #[allow(dead_code)]
    pub name: Option<String>,
    pub config: Option<ProjectConfig>,
}

impl Project {
    /// Detect if we're in an Affogato project
    pub fn detect() -> Result<Self> {
        let cwd = std::env::current_dir()?;

        let mut dir = cwd.clone();
        loop {
            // Check for affogato.toml (new style)
            if dir.join("affogato.toml").exists() {
                let config = ProjectConfig::load(&dir)?;
                let name = config
                    .project
                    .name
                    .clone()
                    .or_else(|| dir.file_name().map(|n| n.to_string_lossy().to_string()));
                return Ok(Self {
                    root: Some(dir),
                    name,
                    config: Some(config),
                });
            }

            // Check for legacy markers (firmware/CMakeLists.txt + fpga/ directory)
            // No longer requires fpga/Makefile
            let has_firmware = dir.join("firmware/CMakeLists.txt").exists();
            let has_fpga = dir.join("fpga").is_dir();
            if has_firmware && has_fpga {
                let name = dir.file_name().map(|n| n.to_string_lossy().to_string());
                // Try to load config if it exists
                let config = if dir.join("affogato.toml").exists() {
                    Some(ProjectConfig::load(&dir)?)
                } else {
                    None
                };
                return Ok(Self {
                    root: Some(dir),
                    name,
                    config,
                });
            }

            if !dir.pop() {
                break;
            }
        }

        Ok(Self {
            root: None,
            name: None,
            config: None,
        })
    }

    pub fn require_project(&self) -> Result<()> {
        if self.root.is_none() {
            bail!(
                "Not in an Affogato project. Run 'affogato new <name>' to create one, or 'affogato init' to initialize the current directory."
            );
        }
        Ok(())
    }
}

/// Create a new project
pub fn create_new(name: &str, _template: &str) -> Result<()> {
    let project_dir = PathBuf::from(name);

    if project_dir.exists() {
        bail!("Directory '{}' already exists", name);
    }

    println!(
        "{}",
        format!("==> Creating new project: {}", name).blue().bold()
    );

    // Create directory structure
    fs::create_dir_all(project_dir.join("firmware/main"))?;
    fs::create_dir_all(project_dir.join("fpga/rtl"))?;

    // Write affogato.toml
    write_affogato_toml(&project_dir, name)?;

    // Write firmware files
    write_firmware_files(&project_dir, name)?;

    // Write FPGA files
    write_fpga_files(&project_dir, name)?;

    println!("{}", "Project created successfully!".green());
    println!();
    println!("Next steps:");
    println!("  cd {}", name);
    println!("  affogato build    # Build FPGA + firmware");
    println!("  affogato flash    # Flash to device");
    println!("  affogato monitor  # Serial console");

    Ok(())
}

/// Initialize current directory as a project
pub fn init_current(_template: &str) -> Result<()> {
    let cwd = std::env::current_dir()?;
    let name = cwd
        .file_name()
        .map(|n| n.to_string_lossy().to_string())
        .unwrap_or_else(|| "project".to_string());

    if cwd.join("firmware").exists() || cwd.join("fpga").exists() {
        bail!("Directory already contains firmware/ or fpga/ - already initialized?");
    }

    println!(
        "{}",
        format!("==> Initializing project: {}", name).blue().bold()
    );

    fs::create_dir_all(cwd.join("firmware/main"))?;
    fs::create_dir_all(cwd.join("fpga/rtl"))?;

    write_affogato_toml(&cwd, &name)?;
    write_firmware_files(&cwd, &name)?;
    write_fpga_files(&cwd, &name)?;

    println!("{}", "Project initialized!".green());

    Ok(())
}

fn write_firmware_files(project_dir: &Path, name: &str) -> Result<()> {
    // CMakeLists.txt
    let cmake = format!(
        r#"cmake_minimum_required(VERSION 3.16)

include($ENV{{IDF_PATH}}/tools/cmake/project.cmake)
project({name})

target_add_binary_data(${{CMAKE_PROJECT_NAME}}.elf "../fpga/top.bin" BINARY)
"#
    );
    fs::write(project_dir.join("firmware/CMakeLists.txt"), cmake)?;

    // main/CMakeLists.txt
    let main_cmake = r#"idf_component_register(
    SRCS "main.c"
    INCLUDE_DIRS "."
    REQUIRES driver
)
"#;
    fs::write(project_dir.join("firmware/main/CMakeLists.txt"), main_cmake)?;

    // main/main.c
    let main_c = format!(
        r#"#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "driver/spi_master.h"
#include "driver/gpio.h"

static const char *TAG = "{name}";

// FPGA bitstream symbols (from target_add_binary_data)
extern const uint8_t _binary_top_bin_start[];
extern const uint8_t _binary_top_bin_end[];

void app_main(void)
{{
    ESP_LOGI(TAG, "{name} starting");

    size_t fpga_size = _binary_top_bin_end - _binary_top_bin_start;
    ESP_LOGI(TAG, "FPGA bitstream size: %d bytes", fpga_size);

    // TODO: Initialize SPI and load FPGA
    // See affogato/components/ice40 for reusable loader

    while (1) {{
        ESP_LOGI(TAG, "Heartbeat");
        vTaskDelay(pdMS_TO_TICKS(1000));
    }}
}}
"#
    );
    fs::write(project_dir.join("firmware/main/main.c"), main_c)?;

    // sdkconfig.defaults
    let sdkconfig = r#"CONFIG_IDF_TARGET="esp32s2"
CONFIG_ESP_CONSOLE_USB_CDC=y
CONFIG_ESP_MAIN_TASK_STACK_SIZE=4096
CONFIG_LOG_COLORS=y
"#;
    fs::write(project_dir.join("firmware/sdkconfig.defaults"), sdkconfig)?;

    Ok(())
}

fn write_affogato_toml(project_dir: &Path, name: &str) -> Result<()> {
    let toml_content = format!(
        r#"[project]
name = "{name}"

[fpga]
device = "up5k"
package = "sg48"
top = "top"
pcf = "fpga/project.pcf"
"#
    );
    fs::write(project_dir.join("affogato.toml"), toml_content)?;
    Ok(())
}

fn write_fpga_files(project_dir: &Path, name: &str) -> Result<()> {
    // project.pcf
    let pcf = r#"# SPI Interface to ESP32-S2
set_io FSPI_CLK     15
set_io FSPI_MOSI    17
set_io FSPI_MISO    14
set_io FSPI_CS      16

# Note: RGB LED pins (39, 40, 41) are directly driven by the SB_RGBA_DRV
# primitive and do not require PCF assignments.
"#;
    fs::write(project_dir.join("fpga/project.pcf"), pcf)?;

    // top.v
    let top_v = format!(
        r#"// {name} - FPGA Top Module
module top (
    input wire FSPI_CLK,
    input wire FSPI_MOSI,
    output wire FSPI_MISO,
    input wire FSPI_CS
);
    // 48MHz internal oscillator
    wire clk;
    SB_HFOSC #(.CLKHF_DIV("0b00")) osc (.CLKHFPU(1'b1), .CLKHFEN(1'b1), .CLKHF(clk));

    // Heartbeat counter
    reg [25:0] counter;
    always @(posedge clk) counter <= counter + 1;

    // RGB LED (directly driven by SB_RGBA_DRV primitive, no external pins needed)
    wire rgb0, rgb1, rgb2;
    SB_RGBA_DRV #(
        .CURRENT_MODE("0b0"),
        .RGB0_CURRENT("0b000001"),
        .RGB1_CURRENT("0b000001"),
        .RGB2_CURRENT("0b000001")
    ) rgb (
        .CURREN(1'b1),
        .RGBLEDEN(1'b1),
        .RGB0PWM(counter[24]),
        .RGB1PWM(counter[25]),
        .RGB2PWM(counter[23]),
        .RGB0(rgb0),
        .RGB1(rgb1),
        .RGB2(rgb2)
    );

    // SPI stub (directly drives MISO low)
    assign FSPI_MISO = 1'b0;
endmodule
"#
    );
    fs::write(project_dir.join("fpga/rtl/top.v"), top_v)?;

    Ok(())
}
