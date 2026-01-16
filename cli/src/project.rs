use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub struct Project {
    pub root: Option<PathBuf>,
    #[allow(dead_code)]
    pub name: Option<String>,
}

impl Project {
    /// Detect if we're in an Affogato project
    pub fn detect() -> Result<Self> {
        let cwd = std::env::current_dir()?;

        // Look for project markers
        let markers = ["firmware/CMakeLists.txt", "fpga/Makefile"];

        let mut dir = cwd.clone();
        loop {
            let is_project = markers.iter().all(|m| dir.join(m).exists());
            if is_project {
                let name = dir.file_name().map(|n| n.to_string_lossy().to_string());
                return Ok(Self {
                    root: Some(dir),
                    name,
                });
            }

            if !dir.pop() {
                break;
            }
        }

        Ok(Self {
            root: None,
            name: None,
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

    // Write firmware files
    write_firmware_files(&project_dir, name)?;

    // Write FPGA files
    write_fpga_files(&project_dir, name)?;

    // Write project Makefile (for legacy compatibility)
    write_project_makefile(&project_dir, name)?;

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

    write_firmware_files(&cwd, &name)?;
    write_fpga_files(&cwd, &name)?;
    write_project_makefile(&cwd, &name)?;

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

fn write_fpga_files(project_dir: &Path, name: &str) -> Result<()> {
    // Makefile
    let makefile = r#"TARGET = top
PCF_FILE = project.pcf
VERILOG_FILES = rtl/top.v

# Docker image
DOCKER_IMAGE ?= ghcr.io/meawoppl/affogato:latest
DOCKER_RUN = docker run --rm -v $(CURDIR):/work -w /work $(DOCKER_IMAGE)

.PHONY: all clean

all: $(TARGET).bin

$(TARGET).json: $(VERILOG_FILES)
	$(DOCKER_RUN) yosys -q \
		-p "synth_ice40 -abc2 -relut -top $(TARGET) -json $@" \
		$(VERILOG_FILES)

$(TARGET).asc: $(TARGET).json $(PCF_FILE)
	$(DOCKER_RUN) nextpnr-ice40 \
		--up5k --package sg48 \
		--json $< --pcf $(PCF_FILE) --asc $@

$(TARGET).bin: $(TARGET).asc
	$(DOCKER_RUN) icepack $< $@

clean:
	rm -f $(TARGET).json $(TARGET).asc $(TARGET).bin
"#;
    fs::write(project_dir.join("fpga/Makefile"), makefile)?;

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

fn write_project_makefile(project_dir: &Path, name: &str) -> Result<()> {
    let makefile = format!(
        r#"# {name} - Project Makefile
# Use 'affogato' CLI for better experience

DOCKER_IMAGE ?= ghcr.io/meawoppl/affogato:latest
DOCKER_RUN = docker run --rm -v $(CURDIR):/workspace -w /workspace $(DOCKER_IMAGE)
DOCKER_RUN_USB = docker run --rm -v $(CURDIR):/workspace -w /workspace --device /dev/ttyACM0 --privileged $(DOCKER_IMAGE)
PORT ?= /dev/ttyACM0

.PHONY: build-fpga build flash monitor clean

build-fpga:
	$(MAKE) -C fpga

build: build-fpga
	$(DOCKER_RUN) bash -c "cd firmware && idf.py build"

flash:
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) flash"

monitor:
	$(DOCKER_RUN_USB) bash -c "cd firmware && idf.py -p $(PORT) monitor"

clean:
	$(MAKE) -C fpga clean
	$(DOCKER_RUN) bash -c "cd firmware && idf.py clean"
"#
    );
    fs::write(project_dir.join("Makefile"), makefile)?;

    Ok(())
}
