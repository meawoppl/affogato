# Affogato FPGA Build System
# Parameterized Makefile for ICE40UP5K synthesis
#
# Usage:
#   make                    # Build bitstream
#   make TARGET=mydesign    # Build with custom top module name
#   make lint               # Run Verilator linting
#   make stats              # Show utilization statistics
#   make clean              # Remove build artifacts
#
# Required variables (can be set in project Makefile that includes this):
#   VERILOG_FILES  - List of Verilog source files
#   PCF_FILE       - Pin constraint file for the project
#
# Optional variables:
#   TARGET         - Top module name (default: top)
#   DEVICE         - ICE40 device (default: up5k)
#   PACKAGE        - Device package (default: sg48)
#   AFFOGATO_PATH  - Path to Affogato root (auto-detected)

# Defaults
TARGET ?= top
DEVICE ?= up5k
PACKAGE ?= sg48
RTL_DIR ?= rtl

# Auto-detect Affogato path if not set
AFFOGATO_PATH ?= $(shell dirname $(realpath $(lastword $(MAKEFILE_LIST))))

# Docker image - uses GitHub Container Registry
DOCKER_IMAGE ?= ghcr.io/meawoppl/affogato:latest
DOCKER_RUN = docker run --rm -v $(CURDIR):/work -w /work $(DOCKER_IMAGE)

# Build outputs
JSON_FILE = $(TARGET).json
ASC_FILE = $(TARGET).asc
BIN_FILE = $(TARGET).bin
LOG_YOSYS = $(TARGET)-yosys.log
LOG_NEXTPNR = $(TARGET)-nextpnr.log

# Default target
.PHONY: all
all: $(BIN_FILE)

# Synthesis: Verilog -> JSON netlist
$(JSON_FILE): $(VERILOG_FILES)
	@echo "==> Synthesizing $(TARGET)"
	$(DOCKER_RUN) yosys -q \
		-p "synth_ice40 -abc2 -relut -top $(TARGET) -json $@" \
		-l $(LOG_YOSYS) \
		$(VERILOG_FILES)

# Place & Route: JSON -> ASC
$(ASC_FILE): $(JSON_FILE) $(PCF_FILE)
	@echo "==> Place & Route $(TARGET)"
	$(DOCKER_RUN) nextpnr-ice40 \
		--$(DEVICE) \
		--package $(PACKAGE) \
		--json $< \
		--pcf $(PCF_FILE) \
		--asc $@ \
		-l $(LOG_NEXTPNR)

# Bitstream: ASC -> BIN
$(BIN_FILE): $(ASC_FILE)
	@echo "==> Generating bitstream $(TARGET).bin"
	$(DOCKER_RUN) icepack $< $@
	@echo "==> Done: $(BIN_FILE) ($(shell stat -c%s $@ 2>/dev/null || stat -f%z $@) bytes)"

# Lint with Verilator
.PHONY: lint
lint: $(VERILOG_FILES)
	@echo "==> Linting Verilog"
	$(DOCKER_RUN) verilator --lint-only -Wall -Wno-DECLFILENAME $(VERILOG_FILES)

# Show utilization statistics
.PHONY: stats
stats: $(JSON_FILE) $(ASC_FILE)
	@echo "==> Yosys Statistics:"
	@sed -n '/=== $(TARGET) ===/,/^$$/p' $(LOG_YOSYS)
	@echo ""
	@echo "==> Device Utilization:"
	@sed -n '/Info: Device utilisation/,/Info: Placed/p' $(LOG_NEXTPNR)
	@echo ""
	@echo "==> Timing:"
	@grep -E 'Info: Max frequency' $(LOG_NEXTPNR) || true

# View synthesized design (requires X11)
.PHONY: view
view: $(VERILOG_FILES)
	$(DOCKER_RUN) yosys -p "read_verilog $(VERILOG_FILES); proc; opt; show -format dot -prefix $(TARGET)"
	@echo "Generated $(TARGET).dot - open with xdot or convert to SVG"

# Clean build artifacts
.PHONY: clean
clean:
	rm -f $(JSON_FILE) $(ASC_FILE) $(BIN_FILE) $(LOG_YOSYS) $(LOG_NEXTPNR) *.dot

# Help
.PHONY: help
help:
	@echo "Affogato FPGA Build System"
	@echo ""
	@echo "Targets:"
	@echo "  all     - Build bitstream (default)"
	@echo "  lint    - Run Verilator linting"
	@echo "  stats   - Show utilization statistics"
	@echo "  view    - Generate visual representation"
	@echo "  clean   - Remove build artifacts"
	@echo ""
	@echo "Variables:"
	@echo "  TARGET=$(TARGET)"
	@echo "  DEVICE=$(DEVICE)"
	@echo "  PACKAGE=$(PACKAGE)"
	@echo "  VERILOG_FILES=$(VERILOG_FILES)"
	@echo "  PCF_FILE=$(PCF_FILE)"
