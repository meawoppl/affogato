#pragma once

#include <esp_err.h>
#include "fpga_bin.h"

/**
 * @defgroup fpga_loader ICE40 FPGA Loader
 * @brief Interface for loading configuration into Lattice ICE40 FPGAs
 *
 * This module implements the ICE40 SPI slave configuration procedure
 * as documented in Lattice Technical Note TN1248 (iCE40 Programming
 * and Configuration).
 *
 * The loader supports both:
 * - Loading from embedded ROM (bitstream linked into firmware)
 * - Loading from filesystem (bitstream stored in VFS)
 *
 * Hardware requirements:
 * - SPI connection (MOSI, SCLK, CS directly wired)
 * - CRESET_B GPIO output (active low reset)
 * - CDONE GPIO input (configuration done indicator)
 *
 * @{
 */

/**
 * @brief Initialize the FPGA loader hardware
 *
 * Configures the CRESET and CDONE GPIO pins for FPGA programming.
 * Must be called before any load operations.
 *
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t fpga_loader_init(void);

/**
 * @brief Load FPGA configuration from embedded ROM
 *
 * Performs the complete ICE40 configuration sequence:
 * 1. Assert CRESET_B low (put FPGA in reset)
 * 2. Assert CS low, clock dummy bytes
 * 3. Release CRESET_B
 * 4. Stream bitstream over SPI
 * 5. Wait for CDONE high
 * 6. Send additional clocks to activate I/O
 *
 * @param fpga_bin Pointer to fpga_bin_t describing the embedded bitstream
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t fpga_loader_load_from_rom(const fpga_bin_t *fpga_bin);

/**
 * @brief Load FPGA configuration from a file
 *
 * Same as fpga_loader_load_from_rom() but reads bitstream from
 * a file in the ESP-IDF VFS (e.g., SPIFFS, SD card).
 *
 * @param filename Path to the bitstream file (e.g., "/spiffs/top.bin")
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t fpga_loader_load_from_file(const char *filename);

/** @} */
