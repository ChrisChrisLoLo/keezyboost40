//! no_std implementation of DelayMs and DelayUs for cortex-m
#![no_std]

extern crate embedded_hal as hal;
use core::ops::Mul;

use hal::prelude::*;

use embedded_hal::blocking::delay::{DelayMs,DelayUs};
use embedded_time::duration::{Microseconds, Extensions};
use embedded_time::TimeInt;


use rp2040_hal::pac::watchdog::tick;
use rp2040_hal::{Timer};

// Todo: duplicate constant for testing: simplify this
const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;


/// asm::delay based Timer
pub struct RP2040TimerDelay<'a> {
    timer: &'a Timer,
    sum: i32
}
 
impl RP2040TimerDelay<'_> {
    pub fn new<'a>(timer: &'a Timer) -> RP2040TimerDelay
    {
        RP2040TimerDelay{
            timer,
            sum: 0,
        }
    }
}

impl<U> DelayMs<U> for RP2040TimerDelay<'_>
where
    U: Into<u32> + Mul<u8 , Output = u8>,
{
    fn delay_ms(&mut self, ms: U) {

        //self.timer.count_down().start(2000000_u32.microseconds());
        //self.timer.count_down().wait();
        let ticksPerSecond = EXTERNAL_XTAL_FREQ_HZ;

        let ticksPerMillisecond = ticksPerSecond/1000;

        let iterations = ms * 1;

        // Iterate rather than multiply to prevent buffer overflow
        for _ in 0..iterations{
            cortex_m::asm::delay(ticksPerMillisecond);
        }
    }
}
// impl<U> DelayUs<U> for RP2040TimerDelay
// where
//     U: Into<u32>,
// {
//     fn delay_us(&mut self, us: U) {
//         self.timer.count_down().start(us.us());
//         self.timer.count_down().wait();
//     }
// }