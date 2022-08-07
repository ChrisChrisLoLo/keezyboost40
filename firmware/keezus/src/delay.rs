//! no_std implementation of DelayMs and DelayUs for cortex-m
#![no_std]

extern crate embedded_hal as hal;
use hal::prelude::*;

use embedded_hal::blocking::delay::{DelayMs,DelayUs};
use embedded_time::duration::{Microseconds, Extensions};
use embedded_time::TimeInt;


use rp2040_hal::{Timer};

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
    U: Into<u32>,
{
    fn delay_ms(&mut self, ms: U) {

        //self.timer.count_down().start(2000000_u32.microseconds());
        //self.timer.count_down().wait();

        for i in 0..1444 {
            for j in 0..1444 {
                for k in 0..1444 {
                    self.sum += i*j*k
                }
            }
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