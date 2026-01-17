# Affogato

A development tool for ESP32 + ICE40 FPGA projects. One command to rule them all.

```
┌─────────────┐      SPI       ┌─────────────┐
│   ESP32-S2  │◄──────────────►│  ICE40UP5K  │
│  (firmware) │                │   (FPGA)    │
└─────────────┘                └─────────────┘
       │                              │
       └──────────── affogato ────────┘
```

## What is this?

Affogato wraps the entire ESP32+ICE40 toolchain (Yosys, nextpnr, icestorm, ESP-IDF) in a single CLI. No manual Docker commands, no environment setup, no version conflicts.

```bash
affogato new myproject    # Create project
affogato build            # Build FPGA + firmware
affogato flash            # Flash to device
affogato monitor          # Serial console
```

The Docker container is pulled automatically on first use.

## Install

**From source (requires Rust):**
```bash
git clone https://github.com/meawoppl/affogato
cd affogato/cli
cargo install --path .
```

**Requirements:** Docker

## Quick Start

```bash
# Create a new project
affogato new blinky
cd blinky

# Build everything (FPGA bitstream + ESP32 firmware)
affogato build

# Flash and monitor
affogato run
```

## Commands

```
affogato new <name>     Create new project with templates
affogato init           Initialize current directory as project
affogato build          Build FPGA bitstream + ESP32 firmware
affogato fpga           Build FPGA bitstream only
affogato flash          Flash firmware to device
affogato monitor        Serial console (Ctrl+] to exit)
affogato run            Flash then monitor
affogato test [name]    Run Verilog testbenches
affogato lint           Lint Verilog with Verilator
affogato menuconfig     ESP-IDF configuration menu
affogato clean          Clean build artifacts
affogato shell          Interactive shell in container
affogato docker pull    Pull/update container image
affogato docker info    Show container status
```

## Project Layout

When you run `affogato new myproject`, you get:

```
myproject/
├── firmware/           # ESP32 code
│   ├── main/
│   │   └── main.c     # Application entry point
│   └── CMakeLists.txt
├── fpga/              # ICE40 code
│   ├── rtl/
│   │   └── top.v      # FPGA top module
│   ├── project.pcf    # Pin constraints
│   └── Makefile
└── Makefile           # Top-level build
```

The FPGA bitstream gets embedded into the ESP32 firmware binary and loaded at boot.

## How It Works

1. **FPGA Build:** Verilog → Yosys → nextpnr-ice40 → icepack → `top.bin`
2. **Embed:** `top.bin` linked into ESP32 firmware via `target_add_binary_data()`
3. **Load:** ESP32 soft-loads ICE40 over SPI at boot using the `ice40` component
4. **Run:** ESP32 and FPGA communicate via SPI

## Reusable Components

### ESP-IDF Component: `ice40`

The `components/ice40` directory contains a reusable ESP-IDF component:

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
    // FPGA is running
}
```

### Verilog Modules

Reusable modules in `fpga/rtl/`:

| Module | Description |
|--------|-------------|
| `spi_slave_bulk.v` | Bulk status read (streams N bytes on CS) |
| `spi_slave_reg.v` | Command/address/data register protocol |
| `sync_ff.v` | Two flip-flop clock domain crossing |
| `toggle_to_strobe.v` | Convert toggle signal to pulse |
| `edge_detect.v` | Rising/falling/both edge detection |
| `rgb_led_driver.v` | ICE40 SB_RGBA_DRV wrapper |

## Configuration

GPIO pins are configurable via ESP-IDF menuconfig (`affogato menuconfig`):

| Option | Default | Description |
|--------|---------|-------------|
| `FPGA_CS_GPIO` | 10 | SPI chip select |
| `FPGA_SCLK_GPIO` | 12 | SPI clock |
| `FPGA_MOSI_GPIO` | 11 | SPI MOSI |
| `FPGA_MISO_GPIO` | 13 | SPI MISO |
| `FPGA_CRESET_GPIO` | 36 | FPGA reset (active low) |
| `FPGA_CDONE_GPIO` | 37 | Configuration done |
| `FPGA_SPI_FREQ_PROGRAMMING` | 20 | Programming clock (MHz) |
| `FPGA_SPI_FREQ_COMMS` | 40 | Runtime clock (MHz) |

## Testing

Verilog testbenches are auto-discovered and run with iverilog:

```bash
# Run all tests in fpga/rtl_test/
affogato test

# Run specific test
affogato test pps_counter

# View waveforms (requires X11)
affogato test pps_counter --view
```

Tests should be named `*_tb.v` and print "PASS" or "FAIL".

## Docker Container

The container (`ghcr.io/meawoppl/affogato:latest`) includes:

- Yosys 0.47
- nextpnr-ice40
- icestorm (icepack, iceprog, icetime)
- iverilog + gtkwave
- verilator
- ESP-IDF 5.3.2

Build locally if needed:
```bash
cd docker
docker build -t ghcr.io/meawoppl/affogato:latest .
```

## Hardware

Designed for the [IcedEspresso board](https://www.hackster.io/news/the-iced-espresso-is-a-cool-refreshing-approach-to-working-with-two-of-our-favorite-chips-6ca50670b175) (ESP32-S2 + ICE40UP5K).

Default QSPI pin assignments for FPGA loading:
```
SCLK=12, MOSI=11, MISO=13, CS=10, WP=14, HD=9
CRESET=36, CDONE=37
```

The RGB LED is driven by the ICE40's internal `SB_RGBA_DRV` primitive (no external pin assignments needed).

## Acknowledgments

Thanks to [BlinkinLabs](https://blinkinlabs.com/) and [@cibomahto](https://x.com/cibomahto/status/1423609225503297537) for the IcedEspresso board design that inspired this project.

## License

MIT
