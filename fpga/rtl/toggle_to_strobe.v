/**
 * Toggle to Strobe Converter
 *
 * Converts a toggle signal (flips on each event) to a strobe
 * (pulses high for one clock cycle per event).
 *
 * Useful for crossing clock domains when combined with sync_ff:
 *   1. Source domain toggles a register on each event
 *   2. sync_ff synchronizes the toggle to destination domain
 *   3. toggle_to_strobe converts back to single-cycle pulse
 *
 * Example:
 *   sync_ff sync_inst (
 *       .i_clk(dest_clk), .i_rst(rst),
 *       .i_async(src_toggle), .o_sync(toggle_synced)
 *   );
 *   toggle_to_strobe conv_inst (
 *       .i_clk(dest_clk), .i_rst(rst),
 *       .i_toggle(toggle_synced), .o_strobe(event_strobe)
 *   );
 */
module toggle_to_strobe (
    input wire i_clk,
    input wire i_rst,
    input wire i_toggle,
    output wire o_strobe
);

    reg toggle_prev;

    always @(posedge i_clk or posedge i_rst) begin
        if (i_rst) begin
            toggle_prev <= 1'b0;
        end else begin
            toggle_prev <= i_toggle;
        end
    end

    assign o_strobe = i_toggle ^ toggle_prev;

endmodule
