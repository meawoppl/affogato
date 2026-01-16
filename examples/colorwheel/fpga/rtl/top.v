// Colorwheel - RGB LED color cycling demo
// Cycles through hues using PWM on the ICE40UP5K internal RGB LED

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

    // Slow counter for color cycling (~1.4 sec per full rotation at 48MHz)
    reg [25:0] phase_counter;
    always @(posedge clk)
        phase_counter <= phase_counter + 1;

    // Use top 8 bits as hue (0-255)
    wire [7:0] hue = phase_counter[25:18];

    // PWM counter for brightness levels
    reg [7:0] pwm_counter;
    always @(posedge clk)
        pwm_counter <= pwm_counter + 1;

    // HSV to RGB conversion (simplified, saturation=1, value=1)
    // Divides hue into 6 sectors of ~42 values each
    reg [7:0] r_level, g_level, b_level;

    always @(posedge clk) begin
        case (hue[7:5])  // 3 bits = 8 sectors, but we use 6
            3'd0: begin  // Red to Yellow (hue 0-42)
                r_level <= 8'd255;
                g_level <= {hue[4:0], 3'b0};  // Rising
                b_level <= 8'd0;
            end
            3'd1: begin  // Yellow to Green (hue 43-85)
                r_level <= 8'd255 - {hue[4:0], 3'b0};  // Falling
                g_level <= 8'd255;
                b_level <= 8'd0;
            end
            3'd2: begin  // Green to Cyan (hue 86-128)
                r_level <= 8'd0;
                g_level <= 8'd255;
                b_level <= {hue[4:0], 3'b0};  // Rising
            end
            3'd3: begin  // Cyan to Blue (hue 129-170)
                r_level <= 8'd0;
                g_level <= 8'd255 - {hue[4:0], 3'b0};  // Falling
                b_level <= 8'd255;
            end
            3'd4: begin  // Blue to Magenta (hue 171-213)
                r_level <= {hue[4:0], 3'b0};  // Rising
                g_level <= 8'd0;
                b_level <= 8'd255;
            end
            default: begin  // Magenta to Red (hue 214-255)
                r_level <= 8'd255;
                g_level <= 8'd0;
                b_level <= 8'd255 - {hue[4:0], 3'b0};  // Falling
            end
        endcase
    end

    // PWM comparison for each channel
    wire r_pwm = (pwm_counter < r_level);
    wire g_pwm = (pwm_counter < g_level);
    wire b_pwm = (pwm_counter < b_level);

    // RGB LED driver (directly drives internal LED, no external pins)
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

    // SPI stub - just acknowledge we're alive
    assign FSPI_MISO = 1'b0;

endmodule
