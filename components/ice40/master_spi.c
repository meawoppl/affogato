#include "ice40/master_spi.h"
#include <driver/spi_master.h>
#include <esp_log.h>

static const char *TAG = "ice40_spi";

SemaphoreHandle_t master_spi_semaphore = NULL;

esp_err_t master_spi_init(void)
{
    if (master_spi_semaphore == NULL) {
        master_spi_semaphore = xSemaphoreCreateMutex();
        if (master_spi_semaphore == NULL) {
            ESP_LOGE(TAG, "Failed to create SPI semaphore");
            return ESP_ERR_NO_MEM;
        }
    }

    ESP_LOGI(TAG, "Configuring SPI bus: SCLK=%d MOSI=%d MISO=%d",
             CONFIG_FPGA_SCLK_GPIO,
             CONFIG_FPGA_MOSI_GPIO,
             CONFIG_FPGA_MISO_GPIO);

    spi_bus_config_t buscfg = {
        .mosi_io_num = CONFIG_FPGA_MOSI_GPIO,
        .miso_io_num = CONFIG_FPGA_MISO_GPIO,
        .sclk_io_num = CONFIG_FPGA_SCLK_GPIO,
        .quadwp_io_num = CONFIG_FPGA_WP_GPIO,
        .quadhd_io_num = CONFIG_FPGA_HD_GPIO,
        .max_transfer_sz = CONFIG_FPGA_SPI_BUFFER_SIZE * 4,
        .flags = SPICOMMON_BUSFLAG_MASTER | SPICOMMON_BUSFLAG_GPIO_PINS,
    };

    esp_err_t ret = spi_bus_initialize(FSPI_HOST, &buscfg, SPI_DMA_CH_AUTO);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "SPI bus init failed: %s", esp_err_to_name(ret));
        return ret;
    }

    ESP_LOGI(TAG, "SPI bus initialized");
    return ESP_OK;
}
