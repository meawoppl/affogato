# Colorwheel

RGB LED color cycling demo for ESP32-S2 + ICE40UP5K.

The ESP32 boots the FPGA with a bitstream that continuously cycles through hues on the internal RGB LED using PWM.

## What it does

1. ESP32 boots and initializes SPI
2. ESP32 loads FPGA bitstream over SPI
3. FPGA starts running and cycles RGB LED through the color spectrum
4. ESP32 prints heartbeat messages while FPGA runs autonomously

## Build & Run

```bash
cd examples/colorwheel

# Using affogato CLI (recommended)
affogato build
affogato run

# Or using make directly
make build
make run
```

## Hardware

Designed for IcedEspresso (ESP32-S2 + ICE40UP5K).

The RGB LED is internal to the ICE40UP5K and driven by the `SB_RGBA_DRV` primitive - no external wiring needed.

## Files

```
colorwheel/
├── firmware/           # ESP32 code
│   ├── main/
│   │   └── main.c     # Boots FPGA, prints heartbeat
│   └── CMakeLists.txt
├── fpga/              # ICE40 code
│   ├── rtl/
│   │   └── top.v      # HSV color wheel with PWM
│   ├── project.pcf    # Pin constraints
│   └── Makefile
└── Makefile           # Top-level build
```

## How the color wheel works

The FPGA implements a simplified HSV-to-RGB conversion:

1. A 26-bit counter increments at 48MHz
2. The top 8 bits (bits 25:18) represent the hue (0-255)
3. Hue is divided into 6 sectors, each transitioning between two primary colors
4. An 8-bit PWM counter compares against the RGB levels to generate PWM signals
5. `SB_RGBA_DRV` drives the internal LED with the PWM outputs

The result is a smooth color transition completing roughly every 1.4 seconds.
