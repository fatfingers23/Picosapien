use crate::commands::RobotCommand;
use embassy_rp::{
    dma::Channel,
    pio::{self, Common, Config, PioPin, ShiftConfig, ShiftDirection, StateMachine},
    PeripheralRef,
};
use fixed::traits::ToFixed;
use fixed_macro::types::U56F8;
use pio_proc::pio_asm;

use {defmt_rtt as _, panic_probe as _};

pub struct RobotControl<'d, PIO: pio::Instance, const SM: usize> {
    sm: StateMachine<'d, PIO, SM>,
}

impl<'d, PIO: pio::Instance, const SM: usize> RobotControl<'d, PIO, SM> {
    pub fn new(
        common: &mut Common<'d, PIO>,
        mut sm: StateMachine<'d, PIO, SM>,
        pin: impl PioPin,
    ) -> Self {
        let prg = pio_asm!(
            "set pindirs, 1",
            "set pins, 1",
            ".wrap_target",
            "pull block",
            "set pins, 0 [15]",
            "set y, 8",
            "bitloop:"
            "out x, 1"
            "jmp !y, end",
            "jmp !x, zero",
            "set pins, 1 [7]"
            "set pins, 0 [1]"
            "jmp y--, bitloop",
            "zero:"
            "set pins, 1 [1]"
            "set pins, 0 [1]",
            "jmp y--, bitloop",
            "end:"
            "set pins, 1 "
            ".wrap"
        );
        // let prg = pio_proc::pio_asm!(
        //     r#"
        //     set pins, 1
        //     set pindirs, 1
        //     .wrap_target
        //         ; Drive pin low for 8 cycles (start condition)
        //         pull block            ; Get command byte from TX FIFO
        //         set pins, 0     ; Drive pin low
        //         nop [7]         ; Wait for 8 cycles

        //         ; Loop through all 8 bits of the command
        //     bitloop:
        //         out x, 1        ; Extract the next bit into X
        //         jmp !x, zero    ; If bit is 0, jump to zero logic
        //         ; Bit is 1
        //         set pins, 1     ; Drive pin high
        //         nop [3]         ; Wait for 4 cycles
        //         set pins, 0     ; Drive pin low
        //         nop             ; Wait for 1 cycle
        //         jmp endbit

        //     zero:
        //         ; Bit is 0
        //         set pins, 1     ; Drive pin high
        //         nop             ; Wait for 1 cycle
        //         set pins, 0     ; Drive pin low
        //         nop             ; Wait for 1 cycle

        //     endbit:
        //         jmp bitloop     ; Repeat for the next bit

        //         ; Set pin back to high after transmission
        //     .wrap
        //         set pins, 1     ; Drive pin high
        //     "#,
        // );

        let pin = common.make_pio_pin(pin);
        let mut cfg = Config::default();
        cfg.use_program(&common.load_program(&prg.program), &[]);
        cfg.set_out_pins(&[&pin]);
        cfg.set_set_pins(&[&pin]);

        //A cycle is .833ms
        cfg.clock_divider = (U56F8!(125_000_000) / U56F8!(2_400)).to_fixed();

        sm.set_config(&cfg);
        sm.set_enable(true);
        Self { sm }
    }

    pub async fn send_command<'a, C: Channel>(
        &mut self,
        ch: PeripheralRef<'a, C>,
        command: RobotCommand,
    ) {
        let mut command_data = [0u8; 8];
        let tx: &mut pio::StateMachineTx<'_, PIO, SM> = self.sm.tx();
        for i in (0..8).rev() {
            let bit = (0x82 as u8 >> i) & 1;
            command_data[i] = bit;
            // ch.dma_push(&bit_data).await;
        }
        tx.dma_push(ch, &command_data).await;
        // self.sm.restart();
    }
}
