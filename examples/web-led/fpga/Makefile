TARGET = top
PCF_FILE = project.pcf
VERILOG_FILES = rtl/top.v rtl/spi_rgb_slave.v rtl/pwm.v

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
