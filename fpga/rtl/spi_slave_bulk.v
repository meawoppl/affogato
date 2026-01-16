/**
 * SPI Slave - Bulk Status Read
 *
 * Simple SPI slave that streams out a fixed-width status register
 * on every CS assertion. Operates entirely in the SPI clock domain.
 *
 * Protocol:
 *   - SPI Mode 0 (CPOL=0, CPHA=0)
 *   - Data shifts out MSB first on SCK falling edge
 *   - Master samples on SCK rising edge
 *   - Data latched at CS falling edge
 *
 * Parameters:
 *   DATA_WIDTH - Total bits to send (must be multiple of 8)
 *
 * Example instantiation for 10-byte status:
 *   spi_slave_bulk #(.DATA_WIDTH(80)) spi_inst (
 *       .i_cs(FSPI_CS),
 *       .i_sck(FSPI_CLK),
 *       .o_miso(FSPI_MISO),
 *       .i_data({8'h00, status, lock_count, cycles})
 *   );
 */
module spi_slave_bulk #(
    parameter DATA_WIDTH = 80
) (
    // SPI interface
    input wire i_cs,                        // Chip select (active low)
    input wire i_sck,                       // SPI clock
    output reg o_miso,                      // Master In Slave Out

    // Data input (directly from system clock domain - async)
    input wire [DATA_WIDTH-1:0] i_data
);

    reg [DATA_WIDTH-1:0] shift_reg;
    reg [$clog2(DATA_WIDTH)-1:0] bit_counter;

    // Latch data at start of transaction (CS falling edge)
    always @(negedge i_cs) begin
        // Pre-shift by 1 bit for SPI Mode 0 timing compensation
        // (we output on falling edge, master samples on rising edge)
        shift_reg <= i_data << 1;
    end

    // Shift out data on SCK falling edge
    always @(negedge i_sck or posedge i_cs) begin
        if (i_cs) begin
            // CS high (idle) - drive MISO low, reset counter
            o_miso <= 1'b0;
            bit_counter <= DATA_WIDTH - 1;
        end else begin
            // CS low (active) - shift out next bit
            o_miso <= shift_reg[bit_counter];
            bit_counter <= (bit_counter == 0) ? DATA_WIDTH - 1 : bit_counter - 1;
        end
    end

endmodule
