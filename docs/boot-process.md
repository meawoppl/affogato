# ESP32 + ICE40 Boot Process

This document describes how an Affogato project boots, from power-on to running application code on both the ESP32 and ICE40.

## Overview

```
Power On
    │
    ▼
┌─────────────────────────────────────────────────────────────────┐
│                         ESP32-S2                                 │
│                                                                  │
│  1. ROM Bootloader                                               │
│  2. Second-stage Bootloader                                      │
│  3. FreeRTOS + app_main()                                        │
│       │                                                          │
│       ▼                                                          │
│  4. master_spi_init()     ─── Configure SPI peripheral           │
│  5. fpga_loader_init()    ─── Configure CRESET/CDONE GPIOs       │
│  6. fpga_loader_load()    ─── Load bitstream via SPI ──────────┐ │
│       │                                                        │ │
│       ▼                                                        │ │
│  7. Application runs      ─── SPI communication with FPGA      │ │
│                                                                │ │
└────────────────────────────────────────────────────────────────│─┘
                                                                 │
                              SPI Bus                            │
                                                                 │
┌────────────────────────────────────────────────────────────────│─┐
│                         ICE40UP5K                              │ │
│                                                                │ │
│  A. Power-on: FPGA in reset (CRESET_B held low)               ◄┘ │
│  B. Configuration: Receives bitstream via SPI slave             │
│  C. CDONE goes high: Configuration complete                     │
│  D. User I/O activated: FPGA design running                     │
│                                                                  │
└──────────────────────────────────────────────────────────────────┘
```

## Detailed Sequence

### Phase 1: ESP32 Boot (0-500ms)

The ESP32-S2 follows its standard boot sequence:

1. **ROM Bootloader** - Runs from mask ROM, initializes basic hardware
2. **Second-stage Bootloader** - Loaded from flash, validates app partition
3. **Application** - FreeRTOS scheduler starts, calls `app_main()`

At this point, the ICE40 is held in reset (CRESET_B is low by default or floating).

### Phase 2: SPI Bus Initialization

```c
esp_err_t master_spi_init(void)
```

Configures the ESP32's FSPI peripheral:

| Signal | GPIO | Description |
|--------|------|-------------|
| SCLK | 12 | SPI clock |
| MOSI | 11 | Master Out, Slave In |
| MISO | 13 | Master In, Slave Out |
| CS | 10 | Chip select (directly controlled) |

The SPI bus is initialized with DMA support and a mutex semaphore for thread-safe access.

### Phase 3: FPGA Loader Initialization

```c
esp_err_t fpga_loader_init(void)
```

Configures the ICE40 control pins:

| Signal | GPIO | Direction | Description |
|--------|------|-----------|-------------|
| CRESET_B | 36 | Output | Active-low reset |
| CDONE | 37 | Input | Configuration done indicator |

CRESET_B is driven low to keep the FPGA in reset until we're ready to configure it.

### Phase 4: ICE40 Configuration (TN1248)

The `fpga_loader_load_from_rom()` function implements the ICE40 SPI Slave Configuration sequence from Lattice Technical Note TN1248:

```
Time ──────────────────────────────────────────────────────────────►

CRESET_B  ▔▔▔▔▔▔▔▔╲___________╱▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔▔
                   ◄── 200ns ──►

SPI_CS    ▔▔▔▔▔▔▔▔▔▔▔▔╲_______╱▔╲________________________________╱▔▔
                      dummy     ◄────── bitstream transfer ──────►

SPI_CLK   ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁┊╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲╱╲▁▁
                                   1-25 MHz during configuration

CDONE     ▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁▁╱▔▔▔▔▔▔▔▔▔
                                                        ▲
                                                        │
                                               config complete
```

#### Step-by-Step:

1. **Assert CRESET_B low** - Put FPGA in reset
2. **Assert SPI_CS low** - Prepare for configuration
3. **Wait 200ns minimum** - Reset setup time
4. **Release CRESET_B high** - Exit reset
5. **Wait 1200µs minimum** - Internal initialization
6. **Send 8 dummy clocks** - With CS high
7. **Assert CS low, send bitstream** - MSB first, 1-25 MHz
8. **Wait for CDONE high** - Send 100+ clocks after bitstream
9. **Send 49+ additional clocks** - Activates user I/O pins
10. **Release CS** - Configuration complete

### Phase 5: FPGA Running

After CDONE goes high:

- The ICE40's internal oscillator starts (48 MHz SB_HFOSC)
- User logic begins executing
- SPI pins transition from configuration to user I/O mode
- The same SPI bus can now be used for ESP32 ↔ FPGA communication

## Bitstream Embedding

The FPGA bitstream is embedded in the ESP32 firmware at compile time:

```cmake
# In CMakeLists.txt
target_add_binary_data(${PROJECT_NAME}.elf "fpga/top.bin" BINARY)
```

This creates linker symbols:
- `_binary_top_bin_start` - Pointer to first byte
- `_binary_top_bin_end` - Pointer past last byte

The bitstream is stored in flash and read directly during configuration (no RAM copy needed).

## Timing Budget

| Phase | Duration | Notes |
|-------|----------|-------|
| ESP32 boot | ~300ms | ROM + bootloader + FreeRTOS |
| SPI init | <1ms | GPIO + peripheral config |
| FPGA reset | ~2ms | Including margins |
| Bitstream transfer | ~7ms | 104KB @ 20 MHz |
| Post-config clocks | <1ms | 49+ clocks |
| **Total** | **~310ms** | Power-on to FPGA running |

## Error Handling

The loader checks for failures at each step:

| Error | Cause | Recovery |
|-------|-------|----------|
| `ESP_ERR_TIMEOUT` | CDONE didn't go high | Check bitstream, check wiring |
| `ESP_ERR_NO_MEM` | DMA buffer allocation failed | Reduce buffer size in Kconfig |
| `ESP_ERR_INVALID_ARG` | Invalid fpga_bin_t pointer | Check linker symbols |

## Code Example

```c
#include "ice40.h"

extern const uint8_t _binary_top_bin_start[];
extern const uint8_t _binary_top_bin_end[];

static const fpga_bin_t fpga_image = {
    .start = _binary_top_bin_start,
    .end = _binary_top_bin_end,
};

void app_main(void)
{
    // Initialize SPI bus
    ESP_ERROR_CHECK(master_spi_init());

    // Initialize FPGA control pins
    ESP_ERROR_CHECK(fpga_loader_init());

    // Load FPGA bitstream (blocks until complete)
    esp_err_t err = fpga_loader_load_from_rom(&fpga_image);
    if (err != ESP_OK) {
        ESP_LOGE(TAG, "FPGA load failed: %s", esp_err_to_name(err));
        return;
    }

    ESP_LOGI(TAG, "FPGA configured successfully");

    // Now safe to communicate with FPGA over SPI
    // ...
}
```

## SPI Modes

Different SPI modes are used for different phases:

| Phase | SPI Mode | Clock | Notes |
|-------|----------|-------|-------|
| Configuration | Mode 3 (CPOL=1, CPHA=1) | 1-25 MHz | Per TN1248 |
| Runtime communication | Mode 0 or 3 | Up to 40 MHz | Application-specific |

The loader temporarily adds a Mode 3 SPI device for configuration, then removes it. Your application can then add its own SPI device with the appropriate mode for runtime communication.

## References

- [Lattice TN1248: iCE40 Programming and Configuration](https://www.latticesemi.com/view_document?document_id=46502)
- [ESP-IDF SPI Master Driver](https://docs.espressif.com/projects/esp-idf/en/latest/esp32s2/api-reference/peripherals/spi_master.html)
- [ICE40 UltraPlus Family Data Sheet](https://www.latticesemi.com/view_document?document_id=51968)
