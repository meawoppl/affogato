# Future Ideas for Affogato

Ideas and potential directions for the project.

## FPGA Development

### Simulation & Testing
- **Cocotb integration** - Python-based testbenches for Verilog modules
- **Formal verification** - SymbiYosys integration for proving correctness
- **Waveform viewer** - Built-in VCD viewer or surfer integration
- **Coverage reporting** - Track which RTL paths are exercised by tests

### Synthesis & Analysis
- **Resource utilization reports** - Show LUT/FF/BRAM usage after synthesis
- **Timing analysis** - icetime integration to report critical paths
- **Schematic viewer** - Visualize synthesized netlist
- **Incremental builds** - Only rebuild changed modules

### IP Cores
- **UART** - Serial communication module
- **I2C master/slave** - For sensor interfacing
- **PWM controller** - Multi-channel with configurable resolution
- **FIFO** - Async FIFO for clock domain crossing
- **Memory controller** - Interface to external SRAM/PSRAM

## ESP32 Development

### Communication
- **BLE support** - Bluetooth Low Energy for mobile app control
- **MQTT client** - IoT cloud connectivity
- **WebSocket server** - Real-time bidirectional web communication
- **OTA updates** - Over-the-air firmware updates

### SPI Enhancements
- **DMA transfers** - High-speed bulk data to/from FPGA
- **Protocol library** - Standardized command/response framework
- **Streaming mode** - Continuous data acquisition

## Tooling

### CLI Improvements
- **Watch mode** - Auto-rebuild on file changes (`affogato watch`)
- **Project templates** - More starting points (SPI peripheral, UART bridge, etc.)
- **Dependency management** - Pull in Verilog modules from git repos
- **Multi-target** - Support other ICE40 variants (HX, LP)

### IDE Integration
- **VSCode extension** - Syntax highlighting, error squiggles, build tasks
- **Language server** - Verible LSP integration for Verilog
- **Schematic capture** - Visual block diagram to Verilog

### CI/CD
- **GitHub Actions templates** - For user projects
- **Hardware-in-the-loop testing** - Run tests on real hardware in CI
- **Bitstream diffing** - Compare builds to catch unintended changes

## Examples & Documentation

### More Demos
- **Logic analyzer** - Capture digital signals, stream to ESP32
- **Signal generator** - DDS waveform output
- **LED matrix** - WS2812B / APA102 driver
- **Audio** - I2S DAC/ADC interface
- **Camera** - OV2640 capture and streaming
- **USB device** - FPGA as USB peripheral via ESP32

### Learning Resources
- **Tutorial series** - Step-by-step guide from blinking LED to complex project
- **Architecture deep-dive** - How the boot process and SPI communication work
- **Verilog crash course** - FPGA basics for embedded developers
- **Video walkthroughs** - Building and flashing demos

## Hardware

### Board Support
- **Other ESP32 variants** - ESP32-S3, ESP32-C3
- **Other FPGAs** - ECP5, Gowin
- **Custom PCB template** - KiCad project for minimal ESP32+ICE40 board

### Peripherals
- **PMOD compatibility** - Standard pinout for add-on boards
- **Grove connectors** - Seeed ecosystem compatibility

## Community

- **Package registry** - Share and discover Verilog modules
- **Project showcase** - Gallery of things people have built
- **Discord/Matrix** - Real-time community chat

---

*Add your ideas via PR or issues!*
