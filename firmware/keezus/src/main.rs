#![no_main]
#![no_std]

mod layout;
mod delay;

const NUM_COLS: usize = 10;
const NUM_ROWS: usize = 4;
const NUM_LAYERS: usize = 1;

pub struct Graphics{
    x: i32,
    y: i32,
}  

/// The linker will place this boot block at the start of our program image. We
/// need this to help the ROM bootloader get our code up and running.
#[link_section = ".boot2"]
#[used]
pub static BOOT2: [u8; 256] = rp2040_boot2::BOOT_LOADER_W25Q080;

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
        timer::{Alarm3, Timer, Alarm, Alarm2},
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
    use embedded_graphics::geometry::Point;
    use st7735_lcd;
    use st7735_lcd::Orientation;
    use embedded_time::rate::Hertz;

    use core::iter::once;

    use crate::delay::RP2040TimerDelay;
    use crate::{NUM_COLS, NUM_ROWS, NUM_LAYERS};
    use crate::Graphics;


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
    use embedded_hal::{prelude::*, watchdog};
    use asm_delay::AsmDelay;
    use asm_delay::bitrate::U32BitrateExt;

    const SCAN_TIME_US: u32 = 1000;
    // const SCAN_TIME_US: u32 = 2000;

    const DISPLAY_UPDATE_TIME_US: u32 = 1700;
    // const DISPLAY_UPDATE_TIME_US: u32 = 3400;

    const EXTERNAL_XTAL_FREQ_HZ: u32 = 12_000_000u32;
    static mut USB_BUS: Option<UsbBusAllocator<UsbBus>> = None;

    const SCREEN_WIDTH: u32 = 128;
    const SCREEN_HEIGHT: u32 = 160;
  
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
        #[lock_free]
        display: st7735_lcd::ST7735<rp2040_hal::Spi<rp2040_hal::spi::Enabled,SPI0,8> , rp2040_hal::gpio::Pin<Gpio16,rp2040_hal::gpio::Output<rp2040_hal::gpio::PushPull>> , rp2040_hal::gpio::Pin<Gpio14,rp2040_hal::gpio::Output<rp2040_hal::gpio::PushPull>>>,
        displayAlarm: Alarm2,
        #[lock_free]
        graphics: Graphics
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
        let mut displayAlarm = timer.alarm_2().unwrap();
        let _ = displayAlarm.schedule(DISPLAY_UPDATE_TIME_US.microseconds());
        displayAlarm.enable_interrupt();


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
        // let matrix = keyberon::matrix::Matrix::new(
        //     [
        //         pins.gpio27.into_push_pull_output().into(),
        //         pins.gpio26.into_push_pull_output().into(),
        //         pins.gpio22.into_push_pull_output().into(),
        //         pins.gpio21.into_push_pull_output().into(),
        //         pins.gpio20.into_push_pull_output().into(),
        //         pins.gpio4.into_push_pull_output().into(),
        //         pins.gpio3.into_push_pull_output().into(),
        //         pins.gpio2.into_push_pull_output().into(),
        //         pins.gpio1.into_push_pull_output().into(),
        //         pins.gpio0.into_push_pull_output().into(),
        //     ],
        //     [
        //         pins.gpio5.into_pull_down_input().into(),
        //         pins.gpio6.into_pull_down_input().into(),
        //         pins.gpio7.into_pull_down_input().into(),
        //         pins.gpio8.into_pull_down_input().into(),
        //     ],
        // );


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

        let mut display = st7735_lcd::ST7735::new(spi, dc, rst, true, false, SCREEN_WIDTH, SCREEN_HEIGHT);
        

        // Cannot use SYST as RTIC has already taken this
        // https://github.com/rtic-rs/cortex-m-rtic/issues/523

        let mut delay = RP2040TimerDelay::new(&timer);
        // let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().0);
        

        // This WORKS
        // use cortex_m::asm::delay;
        // cortex_m::asm::delay(10000000_u32);
        // lcd_led.set_high().unwrap();


        // delay.delay_ms(1000_u32);
        // lcd_led.set_high().unwrap();

        display.init(&mut delay).unwrap();
        display.set_orientation(&Orientation::PortraitSwapped).unwrap();
        display.clear(Rgb565::BLACK).unwrap();
        display.set_offset(0, 0);

        // let image_raw: ImageRawLE<Rgb565> =
        //     ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);

        // let image: Image<_> = Image::new(&image_raw, Point::new(24, 28));

        // image.draw(&mut display).unwrap();
        
        // Wait until the background and image have been rendered otherwise
        // the screen will show random pixels for a brief moment

        lcd_led.set_high().unwrap();

        // start watchdog after initialization
        // It needs to be fairly high though to account for screen drawing etc
        // watchdog.start(10_000.microseconds());
        watchdog.start(1_000_000.microseconds());


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
                display,
                displayAlarm,
                graphics: crate::Graphics{x:0,y:0}
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

    #[task(binds = TIMER_IRQ_2, priority = 1, shared = [ display, displayAlarm, graphics ])]
    fn screen_update_irq(c: screen_update_irq::Context) {
        // please ignore some of this sloppy code
        // i am a good coder irl i pinky promise

        let mut alarm = c.shared.displayAlarm;

        let display = c.shared.display;
        let graphics = c.shared.graphics;

        let image_raw: ImageRawLE<Rgb565> =
        ImageRaw::new(include_bytes!("../assets/ferris.raw"), 86);

        let image: Image<_> = Image::new(&image_raw, Point::new(0, graphics.y));
        
        //display.clear(Rgb565::BLACK).unwrap();

        //let style = embedded_graphics::mono_font::MonoTextStyle::new(, Rgb565::WHITE);
        let styleBlack = embedded_graphics::mono_font::MonoTextStyle::new(&embedded_graphics::mono_font::ascii::FONT_8X13_BOLD, Rgb565::BLACK);

        let textStyleWhite = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::ascii::FONT_8X13_BOLD)
            .text_color(Rgb565::WHITE)
            .background_color(Rgb565::BLACK)
            .build();
        let textStyleRed = embedded_graphics::mono_font::MonoTextStyleBuilder::new()
            .font(&embedded_graphics::mono_font::jis_x0201::FONT_10X20)
            .text_color(Rgb565::RED)
            .background_color(Rgb565::BLACK)
            .build();

        let FONT_BUFFER = 13;
        // let NUM_LINES = 9;
        let NUM_LINES = 8;
        let TOTAL_HEIGHT = SCREEN_HEIGHT as i32+FONT_BUFFER;

        //embedded_graphics::text::Text::new("Hello Rust!", Point::new(20, graphics.y-1), styleBlack).draw(display);
        for i in 0..NUM_LINES {
            //graphics.y += (( i/NUM_LINES) * 200);
            let newY = (graphics.y+(((SCREEN_HEIGHT as i32+FONT_BUFFER)/(NUM_LINES))*i))%(SCREEN_HEIGHT as i32+FONT_BUFFER);// %(SCREEN_HEIGHT as i32+FONT_BUFFER);
            let style = if i == 0 {textStyleRed} else {textStyleWhite};
            //let text1 = if i == 0 {"! 「システム"} else {"! SYSTEM"}; 
            let text1 = if i == 0 {"! ｼｽﾃﾑ"} else {"! SYSTEM"}; 
            //let text2 = if i == 0 {"パニック」!"} else {"PANIC !"};
            let text2 = if i == 0 {"ﾊﾟﾆｯｸ!"} else {"PANIC !"};
            // let text = if i%2==0 {"hocus"} else {"pocus"};
            embedded_graphics::text::Text::new(text1, Point::new(0, TOTAL_HEIGHT-newY), style).draw(display);
            embedded_graphics::text::Text::new(text2, Point::new(70, newY), style).draw(display);
        }

        
        graphics.y = (graphics.y + 1)%(SCREEN_HEIGHT as i32+FONT_BUFFER);

        //image.draw(display).unwrap();

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(DISPLAY_UPDATE_TIME_US.microseconds());
        });

    }


    #[task(binds = TIMER_IRQ_3, priority = 2, shared = [ matrix, debouncer, timer, alarm, watchdog, usb_dev, usb_class])]
    fn scan_timer_irq(mut c: scan_timer_irq::Context) {


        c.shared.watchdog.feed();

        for event in c.shared.debouncer.events(c.shared.matrix.get().unwrap()) {
            handle_event::spawn(Some(event)).unwrap();
        }

        handle_event::spawn(None).unwrap();

        let mut alarm = c.shared.alarm;

        alarm.lock(|a| {
            a.clear_interrupt();
            let _ = a.schedule(SCAN_TIME_US.microseconds());
        });
    }
}