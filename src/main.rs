//! Blinks the LED on a Pico board
//!
//! This will blink an LED attached to GP25, which is the pin the Pico uses for the on-board LED.
#![no_std]
#![no_main]

use bsp::entry;
use defmt::*;
use defmt_rtt as _;
use embedded_hal::adc::OneShot;
use embedded_hal::blocking::i2c::Write;
use embedded_hal::digital::v2::{InputPin, OutputPin};
use panic_probe as _;

// Provide an alias for our BSP so we can switch targets quickly.
// Uncomment the BSP you included in Cargo.toml, the rest of the code does not need to change.
use rp_pico as bsp;
// use sparkfun_pro_micro_rp2040 as bsp;

use bsp::hal::fugit::RateExtU32;
use bsp::hal::{
    clocks::{init_clocks_and_plls, Clock},
    gpio::{FunctionI2C, Pin, PullUp},
    pac,
    sio::Sio,
    watchdog::Watchdog,
    Adc, I2C,
};

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

    let pins = bsp::Pins::new(
        pac.IO_BANK0,
        pac.PADS_BANK0,
        sio.gpio_bank0,
        &mut pac.RESETS,
    );

    // This is the correct pin on the Raspberry Pico board. On other boards, even if they have an
    // on-board LED, it might need to be changed.
    //
    // Notably, on the Pico W, the LED is not connected to any of the RP2040 GPIOs but to the cyw43 module instead.
    // One way to do that is by using [embassy](https://github.com/embassy-rs/embassy/blob/main/examples/rp/src/bin/wifi_blinky.rs)
    //
    // If you have a Pico W and want to toggle a LED with a simple GPIO output pin, you can connect an external
    // LED to one of the GPIO pins, and reference that pin here. Don't forget adding an appropriate resistor
    // in series with the LED.
    let mut led_pin = pins.led.into_push_pull_output();

    let button_0 = pins.gpio19.into_pull_down_input();
    let button_1 = pins.gpio18.into_pull_down_input();
    let button_2 = pins.gpio17.into_pull_down_input();
    let button_3 = pins.gpio16.into_pull_down_input();

    let mut led_0 = pins.gpio13.into_push_pull_output();
    let mut led_1 = pins.gpio12.into_push_pull_output();
    let mut led_2 = pins.gpio11.into_push_pull_output();
    let mut led_3 = pins.gpio10.into_push_pull_output();

    // Enable ADC
    let mut adc = Adc::new(pac.ADC, &mut pac.RESETS);
    // // Enable the temperature sense channel
    let mut temperature_sensor = adc.take_temp_sensor().unwrap();

    // Configure two pins as being I²C, not GPIO
    let sda_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio20.reconfigure();
    let scl_pin: Pin<_, FunctionI2C, PullUp> = pins.gpio21.reconfigure();
    // Create the I²C drive, using the two pre-configured pins. This will fail
    // at compile time if the pins are in the wrong mode, or if this I²C
    // peripheral isn't available on these pins!
    let mut i2c = I2C::i2c0(
        pac.I2C0,
        sda_pin,
        scl_pin, // Try `not_an_scl_pin` here
        400.kHz(),
        &mut pac.RESETS,
        &clocks.system_clock,
    );

    // Write three bytes to the I²C device with 7-bit address 0x2C
    // i2c.write(0x2c, &[1, 2, 3]).unwrap(); // Abort(1) になる。宛先のアドレスのデバイスはないからそうなるのか。
    // I2C addresses are tipically 7 bits long, 0..127
    for address in 0..=127 {
        match i2c.write(address, &[1]) {
            Ok(_) => {
                info!("Found device on address {}\n", address);
            }
            Err(_) => {}
        }
    }
    // cf. https://github.com/JoaquinEduardoArreguez/stm32f1xx-rust-i2c-scanner/blob/c47a66b38b6b7a13e90d0b5543e7f2c598060c41/src/main.rs#L59

    let mut counter: u16 = 0; // 0 ~ 999, 1msごとに1カウントアップ
    let mut is_lighting = false;
    loop {
        if counter % 500 == 0 {
            if is_lighting {
                led_pin.set_low().unwrap();
            } else {
                led_pin.set_high().unwrap();
            }
            is_lighting = !is_lighting;
        }

        if counter == 0 {
            let conversion_factor: f32 = 3.3 / 4096.0; // センサーは12bit。Pythonの例だと2^16で割っている
            let reading: u16 = adc.read(&mut temperature_sensor).unwrap();
            let reading: f32 = f32::from(reading) * conversion_factor;

            let temperature = 27f32 - (reading - 0.706) / 0.001721;
            // https://github.com/raspberrypi/pico-micropython-examples/blob/master/adc/temperature.py
            info!("ADC readings: Temperature: {}", temperature);
        }

        if button_0.is_high().unwrap() {
            led_0.set_high().unwrap();
        } else {
            led_0.set_low().unwrap();
        }
        if button_1.is_high().unwrap() {
            led_1.set_high().unwrap();
        } else {
            led_1.set_low().unwrap();
        }
        if button_2.is_high().unwrap() {
            led_2.set_high().unwrap();
        } else {
            led_2.set_low().unwrap();
        }
        if button_3.is_high().unwrap() {
            led_3.set_high().unwrap();
        } else {
            led_3.set_low().unwrap();
        }

        if counter < 999 {
            counter += 1
        } else {
            counter = 0
        }
        delay.delay_ms(1);
    }
}

// End of file
