#pragma once

#include <esp_err.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>

/**
 * @defgroup master_spi SPI Master Bus
 * @brief Shared SPI bus driver for ICE40 communication
 *
 * This module manages the SPI bus used for both FPGA programming
 * and runtime communication. It provides a mutex semaphore for
 * coordinating access between multiple SPI devices (programming
 * device vs. communication device).
 *
 * @{
 */

/**
 * @brief Semaphore for SPI bus arbitration
 *
 * Take this semaphore before any SPI transaction to ensure
 * exclusive bus access. Release it immediately after the
 * transaction completes.
 *
 * @code
 * xSemaphoreTake(master_spi_semaphore, portMAX_DELAY);
 * spi_device_transmit(device, &transaction);
 * xSemaphoreGive(master_spi_semaphore);
 * @endcode
 */
extern SemaphoreHandle_t master_spi_semaphore;

/**
 * @brief Initialize the SPI master bus
 *
 * Configures the SPI peripheral with pins from Kconfig:
 * - CONFIG_FPGA_SCLK_GPIO
 * - CONFIG_FPGA_MOSI_GPIO
 * - CONFIG_FPGA_MISO_GPIO
 * - CONFIG_FPGA_WP_GPIO (optional QSPI)
 * - CONFIG_FPGA_HD_GPIO (optional QSPI)
 *
 * @return ESP_OK on success, error code otherwise
 */
esp_err_t master_spi_init(void);

/** @} */
