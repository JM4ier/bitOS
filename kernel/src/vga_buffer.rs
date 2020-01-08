use core::fmt;
use volatile::Volatile;
use lazy_static::lazy_static;
use spin::Mutex;

use crate::serial_println;
#[cfg(test)]
use crate::serial_print;

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Color {
    Black = 0,
    Blue = 1,
    Green = 2,
    Cyan = 3,
    Red = 4,
    Magenta = 5,
    Brown = 6,
    LightGray = 7,
    DarkGray = 8,
    LightBlue = 9,
    LightGreen = 10,
    LightCyan = 11,
    LightRed = 12,
    Pink = 13,
    Yellow = 14,
    White = 15,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(transparent)]
pub struct ColorCode(u8);

impl ColorCode {
    pub fn new(foreground: Color, background: Color) -> ColorCode {
        ColorCode((background as u8) << 4 | (foreground as u8))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(C)]
struct ScreenChar {
    ascii_character: u8,
    color_code: ColorCode,
}

const BUFFER_HEIGHT: usize = 25;
const BUFFER_WIDTH: usize = 80;

#[repr(transparent)]
struct Buffer {
    chars: [[Volatile<ScreenChar>; BUFFER_WIDTH];BUFFER_HEIGHT],
}

pub struct Writer {
    column_position: usize,
    color_code: ColorCode,
    buffer: &'static mut Buffer,
}

impl Writer {
    pub fn col_pos (&self) -> usize {
        self.column_position
    }

    pub fn write_byte(&mut self, byte: u8) {
        match byte {
            b'\n' => self.new_line(),
            byte => {
                if self.column_position >= BUFFER_WIDTH {
                    self.new_line();
                }

                let row = BUFFER_HEIGHT - 1;
                let col = self.column_position;

                let color_code = self.color_code;
                self.buffer.chars[row][col].write(ScreenChar {
                    ascii_character: byte,
                    color_code,
                });
                self.column_position += 1;
            },
        }
    }

    pub fn write_string(&mut self, s: &str) {
        for byte in s.bytes() {
            match byte {
                // printable ASCII or newline
                0x20..=0x7e | b'\n' => self.write_byte(byte),
                _ => self.write_byte(0xfe),
            }
        }
    }

    fn new_line(&mut self) {
        for row in 1..BUFFER_HEIGHT {
            for col in 0..BUFFER_WIDTH {
                let character = self.buffer.chars[row][col].read();
                self.buffer.chars[row-1][col].write(character);
            }
        }
        self.clear_row(BUFFER_HEIGHT - 1);
        self.column_position = 0;
    }

    fn clear_row(&mut self, row: usize) {
        let blank = ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        };
        for col in 0..BUFFER_WIDTH {
            self.buffer.chars[row][col].write(blank);
        }
    }

    fn delete_character(&mut self) {
        if self.column_position == 0 {
            return;
        }
        self.column_position -= 1;
        self.buffer.chars[BUFFER_HEIGHT-1][self.column_position].write(ScreenChar {
            ascii_character: b' ',
            color_code: self.color_code,
        });
    }
}

lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Magenta, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}

impl fmt::Write for Writer {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        self.write_string(s);
        Ok(())
    }
}

#[macro_export]
macro_rules! print {
    ($($arg:tt)*) => ($crate::vga_buffer::_print(format_args!($($arg)*)));
}

#[macro_export]
macro_rules! println {
    () => ($crate::print!("\n"));
    ($($arg:tt)*) => ($crate::print!("{}\n", format_args!($($arg)*)));
}

#[doc(hidden)]
pub fn _print(args: fmt::Arguments) {
    use core::fmt::Write;
    use x86_64::instructions::interrupts;

    // no interrupts should happen during this code segment
    // this means that the mutex can't be locked and not unlocked
    // because it is being unlocked at the end of the closure
    interrupts::without_interrupts(|| {
        {
            match WRITER.try_lock() {
                None => serial_println!("surprisingly locked"),
                _ => {},
            }
            // lock dropped when going out of scope
        }
        WRITER.lock().write_fmt(args).unwrap();
        //serial_println!("{}", args); // debug
    });
}

pub fn set_color(color_code: ColorCode) {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().color_code = color_code;
    });
}

pub fn backspace() {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        WRITER.lock().delete_character();
    });
}

#[test_case]
fn sleep_test() {
    use x86_64::instructions::interrupts;
    serial_print!("sleep_test... ");
    interrupts::without_interrupts(|| {
        for _ in 0..1_000_000 {}
    });
    serial_println!("[ok]");
}

#[test_case]
fn test_println_simple() {
    use x86_64::instructions::interrupts;
    interrupts::without_interrupts(|| {
        serial_print!("test_println_simple... ");
        println!("test");
        serial_println!("[ok]");
    });
}

#[test_case]
fn test_println_many() {
    use x86_64::instructions::interrupts;
    serial_print!("test_println_many... ");
    interrupts::without_interrupts(|| {
        for i in 0..500 {
            println!("test qwertzuiop {}", i);
            for _ in 0..10000{}
        }
    });
    serial_println!("[ok]");
}

