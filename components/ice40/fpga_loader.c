#include "ice40/fpga_loader.h"
#include "ice40/master_spi.h"

#include <driver/gpio.h>
#include <driver/spi_master.h>
#include <esp_log.h>
#include <freertos/FreeRTOS.h>
#include <freertos/semphr.h>
#include <freertos/task.h>
#include <soc/gpio_sig_map.h>
#include <soc/soc.h>
#include <rom/gpio.h>

#include <string.h>
#include <sys/stat.h>

#define LOADER_BUFFER_SIZE (CONFIG_FPGA_SPI_BUFFER_SIZE * 4)

static const char *TAG = "ice40_loader";

static spi_device_handle_t fpga_update_device = NULL;

typedef struct {
    size_t size;
    void *ctx;
    size_t (*read)(void *buffer, size_t size, void *ctx);
} firmware_source_t;

static esp_err_t update_spi_device_add(void)
{
    spi_device_interface_config_t devcfg = {
        .clock_speed_hz = CONFIG_FPGA_SPI_FREQ_PROGRAMMING * 1000000,
        .mode = 3,  // ICE40 programming uses SPI Mode 3
        .spics_io_num = -1,  // Manual CS control
        .queue_size = 1,
        .command_bits = 0,
        .address_bits = 0,
        .dummy_bits = 0,
        .flags = SPI_DEVICE_HALFDUPLEX,
    };

    return spi_bus_add_device(FSPI_HOST, &devcfg, &fpga_update_device);
}

static esp_err_t update_spi_device_remove(void)
{
    esp_err_t ret = spi_bus_remove_device(fpga_update_device);
    fpga_update_device = NULL;
    return ret;
}

static esp_err_t write_update_block(const uint8_t *buffer, int length)
{
    if (length > LOADER_BUFFER_SIZE) {
        ESP_LOGE(TAG, "Block too large: %d > %d", length, LOADER_BUFFER_SIZE);
        return ESP_FAIL;
    }

    spi_transaction_t trans = {
        .length = length * 8,
        .tx_buffer = buffer,
        .rx_buffer = NULL,
    };

    xSemaphoreTake(master_spi_semaphore, portMAX_DELAY);
    esp_err_t ret = spi_device_transmit(fpga_update_device, &trans);
    xSemaphoreGive(master_spi_semaphore);

    return ret;
}

static void reset_pin_set(bool value)
{
    gpio_set_level(CONFIG_FPGA_CRESET_GPIO, value ? 1 : 0);
}

static esp_err_t cdone_pin_wait(bool value, uint32_t timeout_ms)
{
    TickType_t timeout = xTaskGetTickCount() + pdMS_TO_TICKS(timeout_ms);

    while (gpio_get_level(CONFIG_FPGA_CDONE_GPIO) != (int)value) {
        if (xTaskGetTickCount() > timeout) {
            return ESP_ERR_TIMEOUT;
        }
        vTaskDelay(1);
    }

    return ESP_OK;
}

static esp_err_t fpga_loader_load(firmware_source_t *source)
{
    esp_err_t ret;

    ret = update_spi_device_add();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to add SPI device: %s", esp_err_to_name(ret));
        return ret;
    }

    ret = spi_device_acquire_bus(fpga_update_device, portMAX_DELAY);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to acquire SPI bus: %s", esp_err_to_name(ret));
        goto cleanup_device;
    }

    // ICE40 Programming Sequence (TN1248, Figure 13.3)

    // Step 1: Drive CRESET_B = 0
    reset_pin_set(0);

    // Step 2: Drive SPI_SS_B = 0
    gpio_set_level(CONFIG_FPGA_CS_GPIO, 0);
    gpio_matrix_out(CONFIG_FPGA_CS_GPIO, SIG_GPIO_OUT_IDX, false, false);

    // Step 3: Wait minimum 200ns
    vTaskDelay(1);

    // Step 4: Release CRESET_B
    reset_pin_set(1);

    // Step 5: Wait minimum 1200us
    vTaskDelay(pdMS_TO_TICKS(2));

    // Step 6: Set SPI_SS_B = 1, send 8 dummy clocks
    gpio_set_level(CONFIG_FPGA_CS_GPIO, 1);
    {
        uint8_t dummy[1] = {0};
        ret = write_update_block(dummy, sizeof(dummy));
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "Failed to send dummy clocks");
            goto cleanup_bus;
        }
    }
    gpio_set_level(CONFIG_FPGA_CS_GPIO, 0);

    // Step 7: Send configuration bitstream
    uint8_t *buffer = heap_caps_malloc(LOADER_BUFFER_SIZE, MALLOC_CAP_DMA);
    if (buffer == NULL) {
        ESP_LOGE(TAG, "Failed to allocate DMA buffer");
        ret = ESP_ERR_NO_MEM;
        goto cleanup_bus;
    }

    size_t remaining = source->size;
    ESP_LOGI(TAG, "Loading %d bytes", remaining);

    while (remaining > 0) {
        size_t chunk = (remaining > LOADER_BUFFER_SIZE) ? LOADER_BUFFER_SIZE : remaining;

        size_t read = source->read(buffer, chunk, source->ctx);
        if (read != chunk) {
            ESP_LOGE(TAG, "Read error: expected %d, got %d", chunk, read);
            ret = ESP_FAIL;
            break;
        }

        ret = write_update_block(buffer, chunk);
        if (ret != ESP_OK) {
            ESP_LOGE(TAG, "Write error");
            break;
        }

        remaining -= chunk;
    }

    // Step 8: Wait for CDONE (send 100+ clocks)
    gpio_set_level(CONFIG_FPGA_CS_GPIO, 1);
    memset(buffer, 0, LOADER_BUFFER_SIZE);
    write_update_block(buffer, 13);  // 13 * 8 = 104 clocks

    ret = cdone_pin_wait(true, 100);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "CDONE timeout - configuration failed");
    }

    // Step 9: Send 49+ additional clocks to activate I/O
    write_update_block(buffer, 7);  // 7 * 8 = 56 clocks

    // Step 10: Restore CS to hardware control
    gpio_set_level(CONFIG_FPGA_CS_GPIO, 1);
    gpio_matrix_out(CONFIG_FPGA_CS_GPIO, FSPICS0_OUT_IDX, false, false);

    heap_caps_free(buffer);

    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "FPGA configuration complete");
    }

cleanup_bus:
    spi_device_release_bus(fpga_update_device);

cleanup_device:
    update_spi_device_remove();

    return ret;
}

// ROM source implementation
typedef struct {
    const uint8_t *data;
    size_t size;
    size_t pos;
} rom_ctx_t;

static size_t rom_read(void *buffer, size_t size, void *ctx)
{
    rom_ctx_t *rom = (rom_ctx_t *)ctx;

    if (rom->pos + size > rom->size) {
        return 0;
    }

    memcpy(buffer, rom->data + rom->pos, size);
    rom->pos += size;
    return size;
}

esp_err_t fpga_loader_load_from_rom(const fpga_bin_t *fpga_bin)
{
    if (fpga_bin == NULL || fpga_bin->end <= fpga_bin->start) {
        ESP_LOGE(TAG, "Invalid FPGA binary");
        return ESP_ERR_INVALID_ARG;
    }

    rom_ctx_t ctx = {
        .data = fpga_bin->start,
        .size = fpga_bin->end - fpga_bin->start,
        .pos = 0,
    };

    ESP_LOGI(TAG, "Loading FPGA from ROM, size=%d", ctx.size);

    firmware_source_t source = {
        .size = ctx.size,
        .ctx = &ctx,
        .read = rom_read,
    };

    return fpga_loader_load(&source);
}

// File source implementation
static size_t file_read(void *buffer, size_t size, void *ctx)
{
    return fread(buffer, 1, size, (FILE *)ctx);
}

esp_err_t fpga_loader_load_from_file(const char *filename)
{
    struct stat st;
    if (stat(filename, &st) == -1) {
        ESP_LOGE(TAG, "File not found: %s", filename);
        return ESP_ERR_NOT_FOUND;
    }

    FILE *fp = fopen(filename, "rb");
    if (fp == NULL) {
        ESP_LOGE(TAG, "Failed to open: %s", filename);
        return ESP_FAIL;
    }

    ESP_LOGI(TAG, "Loading FPGA from %s, size=%ld", filename, st.st_size);

    firmware_source_t source = {
        .size = st.st_size,
        .ctx = fp,
        .read = file_read,
    };

    esp_err_t ret = fpga_loader_load(&source);
    fclose(fp);

    return ret;
}

esp_err_t fpga_loader_init(void)
{
    // Configure CRESET as output (active low)
    gpio_config_t creset_cfg = {
        .pin_bit_mask = (1ULL << CONFIG_FPGA_CRESET_GPIO),
        .mode = GPIO_MODE_OUTPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_set_level(CONFIG_FPGA_CRESET_GPIO, 0);
    gpio_config(&creset_cfg);

    // Configure CDONE as input
    gpio_config_t cdone_cfg = {
        .pin_bit_mask = (1ULL << CONFIG_FPGA_CDONE_GPIO),
        .mode = GPIO_MODE_INPUT,
        .pull_up_en = GPIO_PULLUP_DISABLE,
        .pull_down_en = GPIO_PULLDOWN_DISABLE,
        .intr_type = GPIO_INTR_DISABLE,
    };
    gpio_config(&cdone_cfg);

    ESP_LOGI(TAG, "FPGA loader initialized (CRESET=%d, CDONE=%d)",
             CONFIG_FPGA_CRESET_GPIO, CONFIG_FPGA_CDONE_GPIO);

    return ESP_OK;
}
