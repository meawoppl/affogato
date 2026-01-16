idf_component_register(
    SRCS "main.c"
    INCLUDE_DIRS "."
    REQUIRES ice40 nvs_flash esp_wifi esp_netif esp_http_server
)
