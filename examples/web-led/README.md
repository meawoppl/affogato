# Web LED

WiFi-controlled RGB LED demo for ESP32-S2 + ICE40UP5K.

The ESP32 hosts a WiFi access point with a color picker webpage. When you select a color, it sends the RGB values to the FPGA via SPI, which drives the internal RGB LED with PWM.

## Architecture

```
┌─────────────────┐     WiFi      ┌─────────────────┐
│  Your Phone/PC  │◄────────────►│    ESP32-S2     │
│  (Web Browser)  │               │                 │
└─────────────────┘               │  HTTP Server    │
                                  │  Color Picker   │
                                  └────────┬────────┘
                                           │ SPI
                                           │ [R,G,B]
                                  ┌────────▼────────┐
                                  │   ICE40UP5K     │
                                  │                 │
                                  │  SPI Slave      │
                                  │  PWM Generator  │
                                  │  RGB LED Driver │
                                  └─────────────────┘
```

## How It Works

1. **ESP32 boots** and loads FPGA bitstream over SPI
2. **ESP32 starts WiFi AP** (SSID: `FPGA-LED`, Password: `colorwheel`)
3. **Connect your device** to the WiFi network
4. **Open http://192.168.4.1** in a browser
5. **Pick a color** - the FPGA LED changes immediately

## SPI Protocol

The ESP32 sends 3 bytes (R, G, B) to the FPGA:

```
CS ─────┐                         ┌─────
        └─────────────────────────┘
CLK     ──┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬─┬──
MOSI    ──│R7│R6│R5│R4│R3│R2│R1│R0│G7│G6│...│B1│B0│──
           └─ Red (8 bits) ─┘└─ Green ─┘    └ Blue ┘
```

The FPGA latches the color on CS rising edge and applies PWM.

## Build & Run

```bash
cd examples/web-led

# Using affogato CLI (recommended)
affogato build
affogato run

# Or using make
make build
make run
```

## Files

```
web-led/
├── firmware/
│   └── main/
│       └── main.c          # WiFi AP, HTTP server, SPI driver
├── fpga/
│   └── rtl/
│       ├── top.v           # Top module
│       ├── spi_rgb_slave.v # SPI slave receives RGB
│       └── pwm.v           # 8-bit PWM generator
└── Makefile
```

## Customization

- **WiFi credentials**: Edit `WIFI_SSID` and `WIFI_PASS` in `main.c`
- **SPI speed**: Adjust `clock_speed_hz` in `fpga_spi_device_add()`
- **PWM frequency**: Currently ~187kHz (48MHz / 256)
