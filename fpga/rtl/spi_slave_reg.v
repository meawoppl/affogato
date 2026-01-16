/**
 * SPI Slave - Register Protocol
 *
 * Full-featured SPI slave with command/address/data protocol.
 * Supports memory-mapped register read/write operations.
 *
 * Protocol (SPI Mode 3, CPOL=1, CPHA=1):
 *   Transaction format:
 *   [8-bit command][16-bit address][8-bit dummy][16-bit data words...]
 *
 *   Commands:
 *     0x00 - Read memory (returns data at address, auto-increment)
 *     0x01 - Write memory (writes data to address, auto-increment)
 *     0x02 - Read register (single 16-bit read)
 *     0x03 - Write register (single 16-bit write)
 *
 * Timing:
 *   - 8 dummy bits between address and data for clock domain crossing
 *   - Transaction strobe pulses for each word transferred
 *
 * Ports:
 *   System side (directly in system clock domain or sync'd):
 *     o_command         - Transaction command byte
 *     o_address         - Current transaction address
 *     o_write_data      - Data to write (for write commands)
 *     i_read_data       - Data to read (for read commands)
 *     o_transaction_strobe - Pulses high for one system clock per word
 */
module spi_slave_reg (
    // System clock domain
    input wire i_clk,
    input wire i_rst,

    // SPI interface (SPI clock domain)
    input wire i_cs,
    input wire i_sck,
    input wire i_mosi,
    output reg o_miso,

    // System bus interface
    output reg [7:0] o_command,
    output reg [15:0] o_address,
    output reg [15:0] o_write_data,
    input wire [15:0] i_read_data,
    output wire o_transaction_strobe
);

    // State machine states
    localparam STATE_RX_COMMAND   = 3'd0;
    localparam STATE_RX_ADDRESS   = 3'd1;
    localparam STATE_RX_PRE_DELAY = 3'd2;
    localparam STATE_RX_FIRST     = 3'd3;
    localparam STATE_RX_NEXT      = 3'd4;
    localparam STATE_TX_PRE_DELAY = 3'd5;
    localparam STATE_TX           = 3'd6;

    reg [2:0] state;
    reg [3:0] bit_index;
    reg [14:0] rx_buffer;
    reg [15:0] read_data_sync;
    reg [15:0] read_data_current;
    reg transaction_toggle;

    wire [15:0] rx_data = {rx_buffer, i_mosi};
    wire command_is_write = o_command[0];

    // Synchronize read data to SPI clock domain
    always @(posedge i_sck) begin
        read_data_sync <= i_read_data;
    end

    // Toggle-to-strobe conversion for clock domain crossing
    reg transaction_toggle_sync1, transaction_toggle_sync2;
    reg transaction_toggle_prev;

    always @(posedge i_clk or posedge i_rst) begin
        if (i_rst) begin
            transaction_toggle_sync1 <= 1'b0;
            transaction_toggle_sync2 <= 1'b0;
            transaction_toggle_prev <= 1'b0;
        end else begin
            transaction_toggle_sync1 <= transaction_toggle;
            transaction_toggle_sync2 <= transaction_toggle_sync1;
            transaction_toggle_prev <= transaction_toggle_sync2;
        end
    end

    assign o_transaction_strobe = transaction_toggle_sync2 ^ transaction_toggle_prev;

    // MISO output (falling edge of SCK)
    always @(negedge i_sck or posedge i_cs) begin
        if (i_cs) begin
            o_miso <= 1'b0;
        end else begin
            if (state == STATE_TX) begin
                o_miso <= read_data_current[bit_index];
            end
        end
    end

    // Main SPI state machine (rising edge of SCK)
    always @(posedge i_sck or posedge i_cs) begin
        if (i_cs) begin
            // Reset state on CS deassert
            bit_index <= 4'd7;
            state <= STATE_RX_COMMAND;
        end else begin
            // Shift in MOSI bits
            rx_buffer <= rx_data[14:0];
            bit_index <= bit_index - 1;

            // State transitions on byte/word boundaries
            if (bit_index == 0) begin
                bit_index <= 4'd15;

                case (state)
                    STATE_RX_COMMAND: begin
                        o_command <= rx_data[7:0];
                        state <= STATE_RX_ADDRESS;
                    end

                    STATE_RX_ADDRESS: begin
                        o_address <= rx_data;
                        if (command_is_write) begin
                            state <= STATE_RX_PRE_DELAY;
                            bit_index <= 4'd7;
                        end else begin
                            state <= STATE_TX_PRE_DELAY;
                            bit_index <= 4'd7;
                            transaction_toggle <= ~transaction_toggle;
                        end
                    end

                    STATE_RX_PRE_DELAY: begin
                        state <= STATE_RX_FIRST;
                    end

                    STATE_RX_FIRST: begin
                        o_write_data <= rx_data;
                        transaction_toggle <= ~transaction_toggle;
                        state <= STATE_RX_NEXT;
                    end

                    STATE_RX_NEXT: begin
                        o_write_data <= rx_data;
                        o_address <= o_address + 1;
                        transaction_toggle <= ~transaction_toggle;
                    end

                    STATE_TX_PRE_DELAY: begin
                        read_data_current <= read_data_sync;
                        o_address <= o_address + 1;
                        transaction_toggle <= ~transaction_toggle;
                        state <= STATE_TX;
                    end

                    STATE_TX: begin
                        read_data_current <= read_data_sync;
                        o_address <= o_address + 1;
                        transaction_toggle <= ~transaction_toggle;
                    end
                endcase
            end
        end
    end

    // Initialization for simulation
    initial begin
        o_miso = 0;
        o_write_data = 0;
        o_command = 0;
        o_address = 0;
        rx_buffer = 0;
        transaction_toggle = 0;
        bit_index = 0;
        state = 0;
    end

endmodule
