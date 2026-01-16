/**
 * 8-bit PWM generator
 *
 * Generates PWM output based on 8-bit duty cycle value.
 * At 48MHz with 8-bit counter, PWM frequency is ~187.5kHz.
 */
module pwm (
    input wire i_clk,
    input wire [7:0] i_duty,
    output wire o_pwm
);

    reg [7:0] counter;

    always @(posedge i_clk)
        counter <= counter + 1;

    assign o_pwm = (counter < i_duty);

endmodule
