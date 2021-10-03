use core::fmt::Write;
use embedded_graphics::{
    mono_font::{ascii::FONT_9X18, MonoTextStyle},
    pixelcolor::BinaryColor,
    prelude::*,
    text::{Alignment, Text},
};
use heapless::String;

const DEFAULT_FEED_RATE: u16 = 10;
const DEFAULT_RAPID_FEED_RATE: u16 = 75;
const MIN_FEED_RATE: u16 = 1;
const MAX_FEED_RATE: u16 = 100;

#[derive(PartialEq)]
enum Mode {
    Stop,
    // Forward is towards the chuck
    RunForward,
    // Reverse is towards the tailstock
    RunReverse,
    // Alarm stops the motor, can only be cleared by moving switch to Off
    Alarm(Alarm),
}

#[derive(PartialEq)]
pub enum MotorState {
    // Motor is not running
    Stop,
    // Motor should turn clockwise with specified RPM
    CW(u16),
    // Motor should turn counter clockwise with specified RPM
    CCW(u16),
}

#[derive(PartialEq)]
enum Alarm {
    // Limit switch has been triggered
    Limit,
}

pub struct App {
    mode: Mode,

    feed_rate: u16,
    rapid_feed_rate: u16,
    rapid: bool,
}

impl Default for App {
    fn default() -> Self {
        App {
            mode: Mode::Stop,
            feed_rate: DEFAULT_FEED_RATE,
            rapid_feed_rate: DEFAULT_RAPID_FEED_RATE,
            rapid: false,
        }
    }
}

impl App {
    pub fn input_dir_switch_off(&mut self) {
        self.mode = Mode::Stop
    }

    pub fn input_dir_switch_left(&mut self) {
        if self.mode == Mode::Stop {
            self.mode = Mode::RunForward
        }
    }

    pub fn input_dir_switch_right(&mut self) {
        if self.mode == Mode::Stop {
            self.mode = Mode::RunReverse
        }
    }

    pub fn input_rapid_button(&mut self, pressed: bool) {
        self.rapid = pressed;
    }

    pub fn input_limit_switch(&mut self, pressed: bool) {
        if pressed {
            self.mode = Mode::Alarm(Alarm::Limit);
        }
    }

    pub fn input_dial_change(&mut self, diff: i16) {
        if self.rapid {
            self.rapid_feed_rate = ((self.rapid_feed_rate as i16) + diff)
                .clamp(MIN_FEED_RATE as i16, MAX_FEED_RATE as i16)
                as u16;
        } else {
            self.feed_rate = ((self.feed_rate as i16) + diff)
                .clamp(MIN_FEED_RATE as i16, MAX_FEED_RATE as i16)
                as u16;
        }
    }

    pub fn motor_state(&self) -> MotorState {
        match self.mode {
            Mode::Stop => MotorState::Stop,
            Mode::RunForward => {
                if self.rapid {
                    MotorState::CCW(self.rapid_feed_rate)
                } else {
                    MotorState::CCW(self.feed_rate)
                }
            }
            Mode::RunReverse => {
                if self.rapid {
                    MotorState::CW(self.rapid_feed_rate)
                } else {
                    MotorState::CW(self.feed_rate)
                }
            }
            Mode::Alarm(_) => MotorState::Stop,
        }
    }
}

impl Drawable for App {
    type Color = BinaryColor;
    type Output = ();

    fn draw<D>(&self, display: &mut D) -> Result<(), D::Error>
    where
        D: DrawTarget<Color = BinaryColor>,
    {
        let status_text = match self.mode {
            Mode::Stop => "STOP",
            Mode::RunForward => "FWD",
            Mode::RunReverse => "REV",
            Mode::Alarm(Alarm::Limit) => "ALARM",
        };

        let rate = if self.rapid {
            self.rapid_feed_rate
        } else {
            self.feed_rate
        };

        let mut rate_text: String<32> = String::new();
        match self.mode {
            Mode::Alarm(Alarm::Limit) => write!(rate_text, "LIMIT SWITCH"),
            _ => match rate {
                MIN_FEED_RATE => write!(rate_text, "  {}mm/min +", rate),
                MAX_FEED_RATE => write!(rate_text, "- {}mm/min  ", rate),
                _ => write!(rate_text, "- {}mm/min +", rate),
            },
        }
        .unwrap();

        Text::with_alignment(
            status_text,
            Point::new(display.bounding_box().center().x, 15),
            MonoTextStyle::new(&FONT_9X18, BinaryColor::On),
            Alignment::Center,
        )
        .draw(display)?;

        Text::with_alignment(
            &rate_text,
            display.bounding_box().center() + Point::new(0, 10),
            MonoTextStyle::new(&FONT_9X18, BinaryColor::On),
            Alignment::Center,
        )
        .draw(display)?;

        Ok(())
    }
}
