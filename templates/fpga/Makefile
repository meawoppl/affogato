# Project FPGA Makefile
# Uses Affogato build system

# Project configuration
TARGET = top
PCF_FILE = project.pcf

# Verilog source files
VERILOG_FILES = \
	rtl/top.v

# Include Affogato FPGA build rules
# Set AFFOGATO_PATH or adjust relative path
AFFOGATO_PATH ?= $(shell realpath ../../..)
include $(AFFOGATO_PATH)/fpga/Makefile
