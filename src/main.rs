//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp2040_hal as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::{FunctionI2C, Pin, Pins},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    I2C,
};

use embedded_hal::blocking::i2c::Write;
use fugit::RateExtU32;


#[entry]
fn main() -> ! {
    info!("Program start");
    let mut pac = pac::Peripherals::take().unwrap();
    let core = pac::CorePeripherals::take().unwrap();
    let mut watchdog = Watchdog::new(pac.WATCHDOG);
    let sio = Sio::new(pac.SIO);

    // External high-speed crystal on the pico board is 12Mhz
    let external_xtal_freq_hz = 12_000_000u32;
    let clocks = init_clocks_and_plls(
        external_xtal_freq_hz,
        pac.XOSC,
        pac.CLOCKS,
        pac.PLL_SYS,
        pac.PLL_USB,
        &mut pac.RESETS,
        &mut watchdog,
    )
        .ok()
        .unwrap();

    let mut delay = cortex_m::delay::Delay::new(core.SYST, clocks.system_clock.freq().to_Hz());

    let pins = Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    let mut led_pin = pins.gpio25.into_push_pull_output(); // 25番がpicoだとLED

    let _button_0 = pins.gpio18.into_pull_down_input();
    let _button_1 = pins.gpio19.into_pull_down_input();
    let _button_2 = pins.gpio20.into_pull_down_input();
    let _button_3 = pins.gpio21.into_pull_down_input();

    let sda_pin: Pin<_, FunctionI2C, _> = pins.gpio16.into_function();
    let scl_pin: Pin<_, FunctionI2C, _> = pins.gpio17.into_function();

    let mut display = I2C::i2c0(
        pac.I2C0,
        sda_pin, // sda
        scl_pin, // scl
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );


    info!("on!");
    _button_3.is_high().unwrap();
    led_pin.set_high().unwrap();

    // for b in bytes {
    //     display.write(0x3c, &b).unwrap();
    // }
    delay.delay_ms(40);

    let commands: [u8; 18] = [
        0x80, 0x38, //  FunctionSet : 2行表示
        0x80, 0x39, //  FunctionSet : ISモード = 1
        0x80, 0x14, //  IS=1:OSC周波数 1/4bias
        0x80, 0x70, //  コントラスト
        0x80, 0x56, //  booster-ON , Contrast-2
        0x80, 0x6C, //  Follower control
        0x80, 0x38, //  FunctionSet : ISモード = 0 0x38 // 1行表示 = 0x34
        0x80, 0x0C, //  Display ON , Cursor ON = 0x0D // Cursor OFF = 0x0C
        0x00, 0x01, //  Clear Display
    ];
    display.write(0x3e, &commands).unwrap();
    delay.delay_ms(1);

    // CONSOLE
    display.write(0x3e, &[0x00, 0x38]).unwrap();
    delay.delay_ms(1);

    // CLS
    display.write(0x3e, &[0x00, 0x01]).unwrap();
    delay.delay_ms(1);

    // DISP
    display.write(0x3e, &[0x40, 0b0011_0000, 0b0011_0001, 0b0011_0010]).unwrap();
    delay.delay_ms(1);

    loop {

        // if button_3.is_high().unwrap() {
        //     led_pin.set_high().unwrap();
        //     display.write(0x7c, &[1, 2, 3]).unwrap();
        // } else {
        //     led_pin.set_low().unwrap();
        // }
        cortex_m::asm::wfi();
    }
}

