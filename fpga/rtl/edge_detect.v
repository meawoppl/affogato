/**
 * Edge Detector
 *
 * Detects rising and/or falling edges of an input signal.
 * Outputs a single-cycle pulse on edge detection.
 *
 * Parameters:
 *   EDGE_TYPE - "rising", "falling", or "both" (default: "rising")
 */
module edge_detect #(
    parameter EDGE_TYPE = "rising"  // "rising", "falling", or "both"
) (
    input wire i_clk,
    input wire i_rst,
    input wire i_signal,
    output wire o_edge
);

    reg signal_prev;

    always @(posedge i_clk or posedge i_rst) begin
        if (i_rst) begin
            signal_prev <= 1'b0;
        end else begin
            signal_prev <= i_signal;
        end
    end

    generate
        if (EDGE_TYPE == "rising") begin
            assign o_edge = i_signal & ~signal_prev;
        end else if (EDGE_TYPE == "falling") begin
            assign o_edge = ~i_signal & signal_prev;
        end else begin  // "both"
            assign o_edge = i_signal ^ signal_prev;
        end
    endgenerate

endmodule
