#include <stdio.h>
#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "esp_log.h"
#include "ice40.h"

static const char *TAG = "colorwheel";

// FPGA bitstream (embedded at compile time)
extern const uint8_t _binary_top_bin_start[];
extern const uint8_t _binary_top_bin_end[];

static const fpga_bin_t fpga_image = {
    .start = _binary_top_bin_start,
    .end = _binary_top_bin_end,
};

void app_main(void)
{
    ESP_LOGI(TAG, "Colorwheel example starting");

    // Initialize the SPI bus
    esp_err_t ret = master_spi_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "SPI init failed: %s", esp_err_to_name(ret));
        return;
    }

    // Initialize FPGA control pins (CRESET, CDONE)
    ret = fpga_loader_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "FPGA loader init failed: %s", esp_err_to_name(ret));
        return;
    }

    // Load the bitstream
    size_t fpga_size = _binary_top_bin_end - _binary_top_bin_start;
    ESP_LOGI(TAG, "Loading FPGA bitstream (%d bytes)", fpga_size);

    ret = fpga_loader_load_from_rom(&fpga_image);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "FPGA load failed: %s", esp_err_to_name(ret));
        return;
    }

    ESP_LOGI(TAG, "FPGA running! Watch the RGB LED cycle through colors.");

    // Main loop - just heartbeat
    while (1) {
        ESP_LOGI(TAG, "Heartbeat (FPGA is cycling colors autonomously)");
        vTaskDelay(pdMS_TO_TICKS(5000));
    }
}
