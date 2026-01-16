/**
 * SPI RGB Slave
 *
 * Simple SPI slave that receives RGB values and outputs PWM.
 *
 * Protocol (directly as sent from the ESP32):
 *   CS low -> [8-bit R][8-bit G][8-bit B] -> CS high
 *
 * The RGB values are latched on CS rising edge and used for PWM.
 */
module spi_rgb_slave (
    input wire i_clk,         // System clock (48MHz)
    input wire i_rst,

    // SPI interface
    input wire i_cs,
    input wire i_sck,
    input wire i_mosi,

    // RGB output (accent = set that color on)
    output reg [7:0] o_red,
    output reg [7:0] o_green,
    output reg [7:0] o_blue,
    output reg o_valid        // Pulses when new color received
);

    // Shift register for incoming data
    reg [23:0] shift_reg;
    reg [4:0] bit_count;

    // Synchronize CS to system clock for edge detection
    reg cs_sync1, cs_sync2, cs_prev;
    wire cs_rising = cs_sync2 && !cs_prev;

    always @(posedge i_clk) begin
        cs_sync1 <= i_cs;
        cs_sync2 <= cs_sync1;
        cs_prev <= cs_sync2;
    end

    // Shift in data on SPI clock rising edge
    always @(posedge i_sck or posedge i_cs) begin
        if (i_cs) begin
            bit_count <= 0;
        end else begin
            shift_reg <= {shift_reg[22:0], i_mosi};
            bit_count <= bit_count + 1;
        end
    end

    // Latch RGB values on CS rising edge (transaction complete)
    always @(posedge i_clk or posedge i_rst) begin
        if (i_rst) begin
            o_red <= 8'd0;
            o_green <= 8'd0;
            o_blue <= 8'd0;
            o_valid <= 1'b0;
        end else begin
            o_valid <= 1'b0;

            if (cs_rising && bit_count >= 24) begin
                o_red <= shift_reg[23:16];
                o_green <= shift_reg[15:8];
                o_blue <= shift_reg[7:0];
                o_valid <= 1'b1;
            end
        end
    end

endmodule
