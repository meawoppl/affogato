/**
 * Web LED - FPGA Top Module
 *
 * Receives RGB color via SPI and drives the internal RGB LED with PWM.
 *
 * SPI Protocol:
 *   Send 3 bytes (R, G, B) with CS low, values 0-255.
 *   Color is applied when CS goes high.
 */
module top (
    input wire FSPI_CLK,
    input wire FSPI_MOSI,
    output wire FSPI_MISO,
    input wire FSPI_CS
);

    // 48MHz internal oscillator
    wire clk;
    SB_HFOSC #(.CLKHF_DIV("0b00")) osc (
        .CLKHFPU(1'b1),
        .CLKHFEN(1'b1),
        .CLKHF(clk)
    );

    // RGB values from SPI
    wire [7:0] red, green, blue;
    wire color_valid;

    // SPI slave to receive RGB values
    spi_rgb_slave spi (
        .i_clk(clk),
        .i_rst(1'b0),
        .i_cs(FSPI_CS),
        .i_sck(FSPI_CLK),
        .i_mosi(FSPI_MOSI),
        .o_red(red),
        .o_green(green),
        .o_blue(blue),
        .o_valid(color_valid)
    );

    // PWM generators for each color channel
    wire r_pwm, g_pwm, b_pwm;

    pwm pwm_r (.i_clk(clk), .i_duty(red),   .o_pwm(r_pwm));
    pwm pwm_g (.i_clk(clk), .i_duty(green), .o_pwm(g_pwm));
    pwm pwm_b (.i_clk(clk), .i_duty(blue),  .o_pwm(b_pwm));

    // RGB LED driver (directly drives internal LED)
    wire rgb0, rgb1, rgb2;
    SB_RGBA_DRV #(
        .CURRENT_MODE("0b0"),       // Half current mode
        .RGB0_CURRENT("0b000011"),  // 4mA
        .RGB1_CURRENT("0b000011"),  // 4mA
        .RGB2_CURRENT("0b000011")   // 4mA
    ) rgb_driver (
        .CURREN(1'b1),
        .RGBLEDEN(1'b1),
        .RGB0PWM(r_pwm),
        .RGB1PWM(g_pwm),
        .RGB2PWM(b_pwm),
        .RGB0(rgb0),
        .RGB1(rgb1),
        .RGB2(rgb2)
    );

    // MISO directly grounded (not used)
    assign FSPI_MISO = 1'b0;

endmodule
