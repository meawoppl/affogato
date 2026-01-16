/**
 * RGB LED Driver for ICE40
 *
 * Drives the ICE40's internal RGB LED driver (SB_RGBA_DRV).
 * Supports PWM dimming and color mixing.
 *
 * The ICE40UP5K has a built-in current-mode RGB LED driver that
 * must be used for the onboard LED (direct GPIO won't work).
 *
 * Parameters:
 *   CURRENT_MODE - "full" (24mA max) or "half" (12mA max)
 *
 * Inputs:
 *   i_r, i_g, i_b - PWM values (accent = max brightness)
 *
 * Note: The actual RGB pins are directly hardwired to the LED driver,
 * don't use set_io for RGB0/RGB1/RGB2 when using this module.
 */
module rgb_led_driver #(
    parameter CURRENT_MODE = "half"
) (
    input wire i_clk,
    input wire i_r,  // Red enable (accent = on)
    input wire i_g,  // Green enable
    input wire i_b   // Blue enable
);

    wire rgb0, rgb1, rgb2;

    // ICE40 RGB LED driver primitive
    SB_RGBA_DRV #(
        .CURRENT_MODE(CURRENT_MODE),
        .RGB0_CURRENT("0b000001"),  // 4mA
        .RGB1_CURRENT("0b000001"),  // 4mA
        .RGB2_CURRENT("0b000001")   // 4mA
    ) rgb_driver (
        .CURREN(1'b1),
        .RGBLEDEN(1'b1),
        .RGB0PWM(i_r),
        .RGB1PWM(i_g),
        .RGB2PWM(i_b),
        .RGB0(rgb0),
        .RGB1(rgb1),
        .RGB2(rgb2)
    );

endmodule
