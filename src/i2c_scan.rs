use embedded_hal::blocking::i2c::Read;

// https://docs.rs/rp2040-hal/0.9.1/rp2040_hal/i2c/index.html

pub fn i2c_scan<I2C: Read>(i2c: &mut I2C) -> [bool; 128] {
    let mut addr: [bool; 128] = [false; 128];
    for i in 0..=127 {
        let mut readbuf: [u8; 1] = [0; 1];
        let result = i2c.read(i.clone(), &mut readbuf);
        if let Ok(_) = result {
            // Do whatever work you want to do with found devices
            // writeln!(uart, "Device found at address{:?}", i).unwrap();
            addr[i as usize] = true;
        }
    }

    addr
}