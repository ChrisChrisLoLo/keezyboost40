#![no_main]
#![no_std]

const NUM_COLS: usize = 10;
const NUM_ROWS: usize = 4;
const NUM_LAYERS: usize = 1;

mod layout;
mod delay;

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

const EXTERNAL_CRYSTAL_FREQUENCY_HZ: u32 = 12_000_000;

#[defmt::panic_handler]
fn panic() -> ! {
    cortex_m::asm::udf()
}

#[rtic::app(device = rp2040_hal::pac, peripherals = true, dispatchers = [PIO0_IRQ_0, PIO0_IRQ_1, PIO1_IRQ_0])]
mod app {
    use cortex_m::prelude::{
        _embedded_hal_watchdog_Watchdog, _embedded_hal_watchdog_WatchdogEnable,
    };
    use defmt_rtt as _;
    use embedded_time::duration::Extensions;
    use embedded_time::rate::Extensions as RateExtensions;
    use panic_probe as _;
    use rp2040_hal;
    use rp2040_hal::{
        clocks::{init_clocks_and_plls, Clock},
        gpio::{bank0::*, dynpin::DynPin},
        pac::{I2C0, PIO0, RESETS, SPI0, CorePeripherals},
        pio::{PIOExt, SM0, SM1},
        sio::Sio,
        timer::{Alarm3, Timer, Alarm},
        usb::UsbBus,
        watchdog::Watchdog,
    };
    use embedded_hal::{
        digital::v2::{InputPin, OutputPin},
        timer::CountDown,
    };

    // lcd traits
    use embedded_graphics::image::{Image, ImageRaw, ImageRawLE};
    use embedded_graphics::prelude::*;
    use embedded_graphics::pixelcolor::Rgb565;
    use st7735_lcd;
    use st7735_lcd::Orientation;
    use embedded_time::rate::Hertz;

    use core::iter::once;

    use crate::delay::RP2040TimerDelay;
    use crate::{NUM_COLS, NUM_ROWS, NUM_LAYERS};


    use crate::layout as kb_layout;
    use keyberon::debounce::Debouncer;
    use keyberon::key_code;
    use keyberon::layout::{ Event, Layout};

    use usb_device::class::UsbClass;
    use usb_device::class_prelude::UsbBusAllocator;
    use usb_device::device::UsbDeviceState;

    // hardware delay
    // we explicitly do NOT use any delays using SYST as
    // RTIC has already taken it
    use embedded_hal::prelude::*;
    use asm_delay::AsmDelay;
    use asm_delay::bitrate::U32BitrateExt;

    const SCAN_TIME_US: u32 = 1000;
    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    #[shared]
    struct Shared {
        usb_dev: usb_device::device::UsbDevice<'static, UsbBus>,
        usb_class: keyberon::Class<'static, UsbBus, ()>,
        timer: Timer,
        alarm: Alarm3,
        #[lock_free]
        matrix: keyberon::matrix::Matrix<DynPin,DynPin,NUM_COLS,NUM_ROWS> ,
        layout: Layout<NUM_COLS, NUM_ROWS, NUM_LAYERS, kb_layout::CustomActions>,
        #[lock_free]
        debouncer: Debouncer<[[bool; NUM_COLS]; NUM_ROWS]>,
        #[lock_free]
        watchdog: Watchdog,
    }

    #[local]
    struct Local {}


    #[init]
    fn init(c: init::Context) -> (Shared, Local, init::Monotonics) {
        let mut resets = c.device.RESETS;
        let mut watchdog = Watchdog::new(c.device.WATCHDOG);
        watchdog.pause_on_debug(false);

        let clocks = init_clocks_and_plls(
            EXTERNAL_XTAL_FREQ_HZ,
            c.device.XOSC,
            c.device.CLOCKS,
            c.device.PLL_SYS,
            c.device.PLL_USB,
            &mut resets,
            &mut watchdog,
        )
        .ok()
        .unwrap();

        let sio = Sio::new(c.device.SIO);
        let pins = rp2040_hal::gpio::Pins::new(
            c.device.IO_BANK0,
            c.device.PADS_BANK0,
            sio.gpio_bank0,
            &mut resets,
        );

        let mut timer = Timer::new(c.device.TIMER, &mut resets);
        let mut alarm = timer.alarm_3().unwrap();
        let _ = alarm.schedule(SCAN_TIME_US.microseconds());
        alarm.enable_interrupt();

        let (mut pio, sm0, sm1, _, _) = c.device.PIO0.split(&mut resets);

        let usb_bus = UsbBusAllocator::new(UsbBus::new(
            c.device.USBCTRL_REGS,
            c.device.USBCTRL_DPRAM,
            clocks.usb_clock,
            true,
            &mut resets,
        ));

        unsafe {
            USB_BUS = Some(usb_bus);
        }

        let usb_class = keyberon::new_class(unsafe { USB_BUS.as_ref().unwrap() }, ());
        let usb_dev = keyberon::new_device(unsafe { USB_BUS.as_ref().unwrap() });

        watchdog.start(10_000.microseconds());

        let matrix = keyberon::matrix::Matrix::new(
            [
                pins.gpio27.into_pull_up_input().into(),
                pins.gpio26.into_pull_up_input().into(),
                pins.gpio22.into_pull_up_input().into(),
                pins.gpio21.into_pull_up_input().into(),
                pins.gpio20.into_pull_up_input().into(),
                pins.gpio4.into_pull_up_input().into(),
                pins.gpio3.into_pull_up_input().into(),
                pins.gpio2.into_pull_up_input().into(),
                pins.gpio1.into_pull_up_input().into(),
                pins.gpio0.into_pull_up_input().into(),
            ],
            [
                pins.gpio5.into_push_pull_output().into(),
                pins.gpio6.into_push_pull_output().into(),
                pins.gpio7.into_push_pull_output().into(),
                pins.gpio8.into_push_pull_output().into(),
            ],
        );


        // These are implicitly used by the spi driver if they are in the correct mode
        let _spi_sclk = pins.gpio18.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let _spi_mosi = pins.gpio19.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        //let _spi_miso = pins.gpio4.into_mode::<rp2040_hal::gpio::FunctionSpi>();
        let spi = rp2040_hal::Spi::<_, _, 8>::new(c.device.SPI0);

        let mut lcd_led = pins.gpio15.into_push_pull_output();
        let dc = pins.gpio16.into_push_pull_output();
        let rst = pins.gpio14.into_push_pull_output();

        // Exchange the uninitialised SPI driver for an initialised one
        let spi = spi.init(
            &mut resets,
            clocks.peripheral_clock.freq(),
            Hertz::new(16_000_000u32),
            &embedded_hal::spi::MODE_0,
        );

        let mut disp = st7735_lcd::ST7735::new(spi, dc, rst, true, false, 128, 160);
        

        // Cannot use SYST as RTIC has already taken this
        // https://github.com/rtic-rs/cortex-m-rtic/issues/523
        // let mut delay = RP2040TimerDelay::new(&timer);
        // let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().0);
        use cortex_m::asm::delay;
        //_delay(2000000_u32);
        lcd_led.set_high().unwrap();



        // delay.delay_ms(2000000_u32);
        // disp.init(&mut delay).unwrap();
        // disp.set_orientation(&Orientation::PortraitSwapped).unwrap();
        // disp.clear(Rgb565::GREEN).unwrap();
        // disp.set_offset(0, 25);

        // let image_raw: ImageRawLE<Rgb565> =
        //     ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);

        // let image: Image<_> = Image::new(&image_raw, Point::new(24, 28));

        // image.draw(&mut disp).unwrap();
        
        // // Wait until the background and image have been rendered otherwise
        // // the screen will show random pixels for a brief moment

        // lcd_led.set_high().unwrap();

        (
            Shared {
                usb_dev,
                usb_class,
                timer,
                alarm,
                matrix: matrix.unwrap(),
                debouncer: Debouncer::new([[false; NUM_COLS]; NUM_ROWS], [[false; NUM_COLS]; NUM_ROWS], 10),
                layout: Layout::new(&kb_layout::LAYERS),
                watchdog,
            },
            Local {},
            init::Monotonics(),
        )
    }

    #[task(binds = USBCTRL_IRQ, priority = 4, shared = [usb_dev, usb_class])]
    fn usb_rx(c: usb_rx::Context) {
        let usb = c.shared.usb_dev;
        let kb = c.shared.usb_class;
        (usb, kb).lock(|usb, kb| {
            if usb.poll(&mut [kb]) {
                kb.poll();
            }
        });
    }

    #[task(priority = 2, capacity = 8, shared = [usb_dev, usb_class, layout])]
    fn handle_event(mut c: handle_event::Context, event: Option<Event>) {
        let mut layout = c.shared.layout;
        match event {
            None => {
                if let keyberon::layout::CustomEvent::Press(event) = layout.lock(|l| l.tick()) {
                    match event {
                        kb_layout::CustomActions::Bootloader => {
                            rp2040_hal::rom_data::reset_to_usb_boot(0, 0);
                        }
                    };
                }
            }
            Some(e) => {
                layout.lock(|l| l.event(e));
                return;
            }
        }       

        let report: key_code::KbHidReport = layout.lock(|l| l.keycodes().collect());
        if !c
            .shared
            .usb_class
            .lock(|k| k.device_mut().set_keyboard_report(report.clone()))
        {
            return;
        }
        if c.shared.usb_dev.lock(|d| d.state()) != UsbDeviceState::Configured {
            return;
        }
        while let Ok(0) = c.shared.usb_class.lock(|k| k.write(report.as_bytes())) {}
    }

    #[task(binds = TIMER_IRQ_3, priority = 1, shared = [ matrix, debouncer, timer, alarm, watchdog, usb_dev, usb_class])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {
        let mut alarm = c.shared.alarm;

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });

        c.shared.watchdog.feed();

        for event in c.shared.debouncer.events(c.shared.matrix.get().unwrap()) {
            handle_event::spawn(Some(event)).unwrap();
        }

        handle_event::spawn(None).unwrap();
    }
}