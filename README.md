# Affogato

Standardized development tool for ESP32-S2 + ICE40 FPGA projects.

Affogato provides:
- **Unified Docker container** with Yosys, nextpnr-ice40, icestorm, iverilog, and ESP-IDF
- **Reusable ESP-IDF component** (`ice40`) for FPGA loading and SPI communication
- **Parameterized FPGA build system** with common Verilog modules
- **Project templates** to bootstrap new hardware projects

## Quick Start

```bash
# Pull the development container
docker pull ghcr.io/meawoppl/affogato:latest

# Create a new project
make new-project NAME=myproject

# Build and flash
cd myproject
make build      # Build FPGA + firmware
make flash      # Flash to device
make monitor    # Serial console
```

## Project Structure

```
affogato/
├── docker/              # Unified development container
│   └── Dockerfile
├── components/          # ESP-IDF components
│   └── ice40/          # ICE40 FPGA loader + SPI driver
├── fpga/               # FPGA build system
│   ├── Makefile        # Parameterized build rules
│   ├── iced-espresso.pcf  # Base pin constraints
│   └── rtl/            # Reusable Verilog modules
└── templates/          # Project scaffolding
```

## Using in Your Project

### 1. Add ice40 Component

In your ESP-IDF project's `CMakeLists.txt`:

```cmake
set(EXTRA_COMPONENT_DIRS $ENV{AFFOGATO_PATH}/components)
include($ENV{IDF_PATH}/tools/cmake/project.cmake)
project(myproject)

# Embed FPGA bitstream
target_add_binary_data(${PROJECT_NAME}.elf "fpga/top.bin" BINARY)
```

### 2. Use in Code

```c
#include "ice40.h"

extern const uint8_t _binary_top_bin_start[];
extern const uint8_t _binary_top_bin_end[];

static const fpga_bin_t fpga = {
    .start = _binary_top_bin_start,
    .end = _binary_top_bin_end,
};

void app_main(void) {
    master_spi_init();
    fpga_loader_init();
    fpga_loader_load_from_rom(&fpga);
    // FPGA is now running!
}
```

### 3. Include FPGA Build Rules

In your FPGA `Makefile`:

```makefile
TARGET = top
PCF_FILE = project.pcf
VERILOG_FILES = rtl/top.v

include $(AFFOGATO_PATH)/fpga/Makefile
```

## Reusable Verilog Modules

| Module | Description |
|--------|-------------|
| `spi_slave_bulk.v` | Simple bulk status read (SPI Mode 0) |
| `spi_slave_reg.v` | Register protocol with commands (SPI Mode 3) |
| `sync_ff.v` | Two flip-flop synchronizer |
| `toggle_to_strobe.v` | Toggle-to-strobe converter |
| `edge_detect.v` | Rising/falling edge detector |
| `rgb_led_driver.v` | ICE40 RGB LED driver wrapper |

## Configuration (Kconfig)

The `ice40` component exposes these configuration options:

| Option | Default | Description |
|--------|---------|-------------|
| `FPGA_CS_GPIO` | 10 | SPI chip select |
| `FPGA_SCLK_GPIO` | 12 | SPI clock |
| `FPGA_MOSI_GPIO` | 11 | SPI data out |
| `FPGA_MISO_GPIO` | 13 | SPI data in |
| `FPGA_CRESET_GPIO` | 36 | FPGA reset (active low) |
| `FPGA_CDONE_GPIO` | 37 | FPGA config done |
| `FPGA_SPI_FREQ_PROGRAMMING` | 20 | Programming speed (MHz) |
| `FPGA_SPI_FREQ_COMMS` | 40 | Runtime speed (MHz) |

## Docker Container

The container includes:
- **Yosys 0.47** - Verilog synthesis
- **nextpnr-ice40** - Place and route
- **icestorm** - Bitstream tools (icepack, iceprog)
- **iverilog + gtkwave** - Simulation
- **verilator** - Linting and simulation
- **ESP-IDF 5.3.2** - ESP32 firmware development

### Building Locally

```bash
make docker-build
```

### Using the Container

```bash
# Interactive shell
make docker-shell

# With USB access (for flashing)
make docker-shell-usb
```

## Hardware Support

Affogato is designed for the IcedEspresso board series (ESP32-S2 + ICE40UP5K).

Common pin assignments are defined in `fpga/iced-espresso.pcf`:
- SPI: CLK=15, MOSI=17, MISO=14, CS=16
- RGB LED: R=39, G=40, B=41

## License

MIT
