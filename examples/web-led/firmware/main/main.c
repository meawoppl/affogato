#include <stdio.h>
#include <string.h>
#include <stdlib.h>

#include "freertos/FreeRTOS.h"
#include "freertos/task.h"
#include "freertos/event_groups.h"

#include "esp_wifi.h"
#include "esp_event.h"
#include "esp_log.h"
#include "esp_netif.h"
#include "esp_http_server.h"
#include "nvs_flash.h"

#include "driver/spi_master.h"
#include "driver/gpio.h"

#include "ice40.h"

static const char *TAG = "web-led";

// FPGA bitstream
extern const uint8_t _binary_top_bin_start[];
extern const uint8_t _binary_top_bin_end[];

static const fpga_bin_t fpga_image = {
    .start = _binary_top_bin_start,
    .end = _binary_top_bin_end,
};

// SPI device for FPGA communication (after boot)
static spi_device_handle_t fpga_spi_device = NULL;

// Current RGB values
static uint8_t current_r = 0, current_g = 0, current_b = 0;

// WiFi AP configuration
#define WIFI_SSID "FPGA-LED"
#define WIFI_PASS "colorwheel"
#define WIFI_CHANNEL 1
#define MAX_STA_CONN 4

// HTML page with color picker
static const char *INDEX_HTML =
"<!DOCTYPE html>\n"
"<html>\n"
"<head>\n"
"  <meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n"
"  <title>FPGA LED Control</title>\n"
"  <style>\n"
"    body { font-family: sans-serif; text-align: center; padding: 20px; background: #1a1a2e; color: #eee; }\n"
"    h1 { color: #fff; }\n"
"    .picker { margin: 30px auto; }\n"
"    input[type=color] { width: 200px; height: 200px; border: none; cursor: pointer; border-radius: 50%; }\n"
"    .color-display { font-size: 24px; margin: 20px; font-family: monospace; }\n"
"    .info { color: #888; font-size: 14px; margin-top: 40px; }\n"
"  </style>\n"
"</head>\n"
"<body>\n"
"  <h1>FPGA RGB LED</h1>\n"
"  <div class=\"picker\">\n"
"    <input type=\"color\" id=\"colorPicker\" value=\"#000000\">\n"
"  </div>\n"
"  <div class=\"color-display\" id=\"colorValue\">#000000</div>\n"
"  <p class=\"info\">Pick a color to control the ICE40 FPGA RGB LED via SPI</p>\n"
"  <script>\n"
"    const picker = document.getElementById('colorPicker');\n"
"    const display = document.getElementById('colorValue');\n"
"    let timeout = null;\n"
"    picker.addEventListener('input', function() {\n"
"      display.textContent = this.value;\n"
"      display.style.color = this.value;\n"
"      clearTimeout(timeout);\n"
"      timeout = setTimeout(() => {\n"
"        const hex = this.value.substring(1);\n"
"        const r = parseInt(hex.substring(0,2), 16);\n"
"        const g = parseInt(hex.substring(2,4), 16);\n"
"        const b = parseInt(hex.substring(4,6), 16);\n"
"        fetch('/set?r=' + r + '&g=' + g + '&b=' + b);\n"
"      }, 50);\n"
"    });\n"
"  </script>\n"
"</body>\n"
"</html>\n";

// Send RGB to FPGA via SPI
static esp_err_t send_rgb_to_fpga(uint8_t r, uint8_t g, uint8_t b)
{
    if (fpga_spi_device == NULL) {
        return ESP_ERR_INVALID_STATE;
    }

    uint8_t data[3] = {r, g, b};

    spi_transaction_t trans = {
        .length = 24,  // 3 bytes = 24 bits
        .tx_buffer = data,
    };

    xSemaphoreTake(master_spi_semaphore, portMAX_DELAY);
    esp_err_t ret = spi_device_transmit(fpga_spi_device, &trans);
    xSemaphoreGive(master_spi_semaphore);

    if (ret == ESP_OK) {
        ESP_LOGI(TAG, "Sent RGB(%d, %d, %d) to FPGA", r, g, b);
    }

    return ret;
}

// HTTP handler for index page
static esp_err_t index_handler(httpd_req_t *req)
{
    httpd_resp_set_type(req, "text/html");
    httpd_resp_send(req, INDEX_HTML, strlen(INDEX_HTML));
    return ESP_OK;
}

// HTTP handler for setting color
static esp_err_t set_handler(httpd_req_t *req)
{
    char buf[64];
    int r = 0, g = 0, b = 0;

    // Parse query string
    if (httpd_req_get_url_query_str(req, buf, sizeof(buf)) == ESP_OK) {
        char param[8];
        if (httpd_query_key_value(buf, "r", param, sizeof(param)) == ESP_OK) {
            r = atoi(param);
        }
        if (httpd_query_key_value(buf, "g", param, sizeof(param)) == ESP_OK) {
            g = atoi(param);
        }
        if (httpd_query_key_value(buf, "b", param, sizeof(param)) == ESP_OK) {
            b = atoi(param);
        }
    }

    // Clamp values
    r = (r < 0) ? 0 : (r > 255) ? 255 : r;
    g = (g < 0) ? 0 : (g > 255) ? 255 : g;
    b = (b < 0) ? 0 : (b > 255) ? 255 : b;

    current_r = r;
    current_g = g;
    current_b = b;

    send_rgb_to_fpga(r, g, b);

    httpd_resp_set_type(req, "text/plain");
    httpd_resp_send(req, "OK", 2);
    return ESP_OK;
}

static httpd_handle_t start_webserver(void)
{
    httpd_config_t config = HTTPD_DEFAULT_CONFIG();
    httpd_handle_t server = NULL;

    if (httpd_start(&server, &config) == ESP_OK) {
        httpd_uri_t index_uri = {
            .uri = "/",
            .method = HTTP_GET,
            .handler = index_handler,
        };
        httpd_register_uri_handler(server, &index_uri);

        httpd_uri_t set_uri = {
            .uri = "/set",
            .method = HTTP_GET,
            .handler = set_handler,
        };
        httpd_register_uri_handler(server, &set_uri);

        ESP_LOGI(TAG, "HTTP server started");
    }

    return server;
}

static void wifi_event_handler(void *arg, esp_event_base_t event_base,
                               int32_t event_id, void *event_data)
{
    if (event_id == WIFI_EVENT_AP_STACONNECTED) {
        wifi_event_ap_staconnected_t *event = (wifi_event_ap_staconnected_t *)event_data;
        ESP_LOGI(TAG, "Station connected, AID=%d", event->aid);
    } else if (event_id == WIFI_EVENT_AP_STADISCONNECTED) {
        wifi_event_ap_stadisconnected_t *event = (wifi_event_ap_stadisconnected_t *)event_data;
        ESP_LOGI(TAG, "Station disconnected, AID=%d", event->aid);
    }
}

static void wifi_init_softap(void)
{
    ESP_ERROR_CHECK(esp_netif_init());
    ESP_ERROR_CHECK(esp_event_loop_create_default());
    esp_netif_create_default_wifi_ap();

    wifi_init_config_t cfg = WIFI_INIT_CONFIG_DEFAULT();
    ESP_ERROR_CHECK(esp_wifi_init(&cfg));

    ESP_ERROR_CHECK(esp_event_handler_instance_register(WIFI_EVENT,
                    ESP_EVENT_ANY_ID, &wifi_event_handler, NULL, NULL));

    wifi_config_t wifi_config = {
        .ap = {
            .ssid = WIFI_SSID,
            .ssid_len = strlen(WIFI_SSID),
            .channel = WIFI_CHANNEL,
            .password = WIFI_PASS,
            .max_connection = MAX_STA_CONN,
            .authmode = WIFI_AUTH_WPA2_PSK,
            .pmf_cfg = {
                .required = false,
            },
        },
    };

    ESP_ERROR_CHECK(esp_wifi_set_mode(WIFI_MODE_AP));
    ESP_ERROR_CHECK(esp_wifi_set_config(WIFI_IF_AP, &wifi_config));
    ESP_ERROR_CHECK(esp_wifi_start());

    ESP_LOGI(TAG, "WiFi AP started. SSID: %s, Password: %s", WIFI_SSID, WIFI_PASS);
    ESP_LOGI(TAG, "Connect and open http://192.168.4.1");
}

static esp_err_t fpga_spi_device_add(void)
{
    spi_device_interface_config_t devcfg = {
        .clock_speed_hz = 1000000,  // 1MHz for reliable communication
        .mode = 0,                   // SPI Mode 0
        .spics_io_num = CONFIG_FPGA_CS_GPIO,
        .queue_size = 1,
        .flags = SPI_DEVICE_HALFDUPLEX,
    };

    return spi_bus_add_device(FSPI_HOST, &devcfg, &fpga_spi_device);
}

void app_main(void)
{
    ESP_LOGI(TAG, "Web LED example starting");

    // Initialize NVS (required for WiFi)
    esp_err_t ret = nvs_flash_init();
    if (ret == ESP_ERR_NVS_NO_FREE_PAGES || ret == ESP_ERR_NVS_NEW_VERSION_FOUND) {
        ESP_ERROR_CHECK(nvs_flash_erase());
        ret = nvs_flash_init();
    }
    ESP_ERROR_CHECK(ret);

    // Initialize SPI bus
    ret = master_spi_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "SPI init failed: %s", esp_err_to_name(ret));
        return;
    }

    // Initialize FPGA control pins
    ret = fpga_loader_init();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "FPGA loader init failed: %s", esp_err_to_name(ret));
        return;
    }

    // Load FPGA bitstream
    size_t fpga_size = _binary_top_bin_end - _binary_top_bin_start;
    ESP_LOGI(TAG, "Loading FPGA bitstream (%d bytes)", fpga_size);

    ret = fpga_loader_load_from_rom(&fpga_image);
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "FPGA load failed: %s", esp_err_to_name(ret));
        return;
    }

    ESP_LOGI(TAG, "FPGA configured successfully");

    // Add SPI device for FPGA communication
    ret = fpga_spi_device_add();
    if (ret != ESP_OK) {
        ESP_LOGE(TAG, "Failed to add FPGA SPI device: %s", esp_err_to_name(ret));
        return;
    }

    // Set initial color (off)
    send_rgb_to_fpga(0, 0, 0);

    // Start WiFi AP
    wifi_init_softap();

    // Start HTTP server
    start_webserver();

    // Main loop - heartbeat
    while (1) {
        ESP_LOGI(TAG, "RGB(%d, %d, %d) - http://192.168.4.1", current_r, current_g, current_b);
        vTaskDelay(pdMS_TO_TICKS(10000));
    }
}
