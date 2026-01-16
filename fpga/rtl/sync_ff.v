/**
 * Synchronizer - Two Flip-Flop
 *
 * Standard two-stage synchronizer for crossing clock domains.
 * Use this when bringing an asynchronous signal into a clock domain.
 *
 * Parameters:
 *   WIDTH     - Signal width (default: 1)
 *   RESET_VAL - Reset value (default: 0)
 *
 * Timing:
 *   Output is delayed by 2 clock cycles from input transition.
 */
module sync_ff #(
    parameter WIDTH = 1,
    parameter RESET_VAL = 0
) (
    input wire i_clk,
    input wire i_rst,
    input wire [WIDTH-1:0] i_async,
    output reg [WIDTH-1:0] o_sync
);

    reg [WIDTH-1:0] sync_stage1;

    always @(posedge i_clk or posedge i_rst) begin
        if (i_rst) begin
            sync_stage1 <= RESET_VAL;
            o_sync <= RESET_VAL;
        end else begin
            sync_stage1 <= i_async;
            o_sync <= sync_stage1;
        end
    end

endmodule
