#pragma once

#include <stdint.h>

/**
 * @brief FPGA binary image descriptor
 *
 * Used to reference an FPGA bitstream embedded in the firmware binary
 * via ESP-IDF's target_add_binary_data() CMake function.
 *
 * Example usage:
 * @code
 * // In CMakeLists.txt:
 * // target_add_binary_data(${PROJECT_NAME}.elf "fpga/top.bin" BINARY)
 *
 * // In C code:
 * extern const uint8_t _binary_top_bin_start[];
 * extern const uint8_t _binary_top_bin_end[];
 *
 * static const fpga_bin_t fpga_image = {
 *     .start = _binary_top_bin_start,
 *     .end = _binary_top_bin_end,
 * };
 *
 * fpga_loader_load_from_rom(&fpga_image);
 * @endcode
 */
typedef struct {
    const uint8_t *start;  ///< Pointer to the start of the bitstream in ROM
    const uint8_t *end;    ///< Pointer to the end of the bitstream in ROM
} fpga_bin_t;
