#![no_main]
#![no_std]

use embedded_hal::digital::v2::OutputPin;
use sherline_power_feed::app::App;
use sherline_power_feed::{
    self as _, // global logger + panicking-behavior + memory layout
    app::MotorState,
};

use embedded_graphics::prelude::*;
use ssd1306::{prelude::*, I2CDisplayInterface, Ssd1306};
use stm32f1xx_hal::qei::QeiOptions;
use stm32f1xx_hal::{
    delay::Delay,
    i2c::{BlockingI2c, DutyCycle, Mode},
    prelude::*,
    pwm::Channel,
    qei::Qei,
    // spi::{Mode, Phase, Polarity, Spi},
    stm32::{Peripherals, TIM4},
    time::U32Ext,
    timer::{Tim2NoRemap, Tim4NoRemap, Timer},
};
use switch_hal::{InputSwitch, IntoSwitch};

const MOTOR_STEPS_PER_REV: u16 = 12800;

#[cortex_m_rt::entry]
fn main() -> ! {
    defmt::info!("main starting");

    let mut app = App::default();

    // Periphs/Clocks
    let dp = Peripherals::take().unwrap();
    let cp = cortex_m::Peripherals::take().unwrap();
    let mut rcc = dp.RCC.constrain();
    let mut flash = dp.FLASH.constrain();
    let mut afio = dp.AFIO.constrain(&mut rcc.apb2);
    let clocks = rcc.cfgr.freeze(&mut flash.acr);
    let mut delay = Delay::new(cp.SYST, clocks);

    // GPIO
    let mut gpioa = dp.GPIOA.split(&mut rcc.apb2);
    let mut gpiob = dp.GPIOB.split(&mut rcc.apb2);

    // Stepper Control
    let step_pul = gpioa.pa0.into_alternate_push_pull(&mut gpioa.crl);
    let mut step_dir = gpioa.pa1.into_push_pull_output(&mut gpioa.crl);
    let mut step_pwm = Timer::tim2(dp.TIM2, &clocks, &mut rcc.apb1).pwm::<Tim2NoRemap, _, _, _>(
        step_pul,
        &mut afio.mapr,
        1.khz(),
    );
    step_pwm.disable(Channel::C1);
    step_pwm.set_period(46.us());
    step_pwm.set_duty(Channel::C1, 50);

    // Display pins
    let scl = gpiob.pb8.into_alternate_open_drain(&mut gpiob.crh);
    let sda = gpiob.pb9.into_alternate_open_drain(&mut gpiob.crh);
    let i2c = BlockingI2c::i2c1(
        dp.I2C1,
        (scl, sda),
        &mut afio.mapr,
        Mode::Fast {
            frequency: 400_000.hz(),
            duty_cycle: DutyCycle::Ratio2to1,
        },
        clocks,
        &mut rcc.apb1,
        1000,
        10,
        1000,
        1000,
    );

    // Display
    let interface = I2CDisplayInterface::new(i2c);
    let mut display = Ssd1306::new(interface, DisplaySize128x64, DisplayRotation::Rotate0)
        .into_buffered_graphics_mode();
    display.init().unwrap();

    // Speed control
    let mut rotary_encoder = {
        let pins = (gpiob.pb6, gpiob.pb7);
        let qei = Timer::tim4(dp.TIM4, &clocks, &mut rcc.apb1).qei(
            pins,
            &mut afio.mapr,
            QeiOptions::default(),
        );

        RotaryEncoder::new(qei)
    };

    // Switches
    let dir_switch_left = gpioa
        .pa8
        .into_pull_up_input(&mut gpioa.crh)
        .into_active_low_switch();
    let dir_switch_right = gpioa
        .pa9
        .into_pull_up_input(&mut gpioa.crh)
        .into_active_low_switch();
    let rapid_feed_switch = gpioa
        .pa10
        .into_pull_up_input(&mut gpioa.crh)
        .into_active_low_switch();
    let limit_switch = gpioa
        .pa11
        .into_pull_up_input(&mut gpioa.crh)
        .into_active_low_switch();

    let mut last_motor_state = MotorState::Stop;

    loop {
        // process inputs
        {
            // limit switch is NC
            app.input_limit_switch(!limit_switch.is_active().unwrap());

            if dir_switch_left.is_active().unwrap() {
                app.input_dir_switch_left();
            } else if dir_switch_right.is_active().unwrap() {
                app.input_dir_switch_right();
            } else {
                app.input_dir_switch_off();
            };

            app.input_rapid_button(rapid_feed_switch.is_active().unwrap());

            if let Some(diff) = rotary_encoder.poll() {
                app.input_dial_change(diff);
            }
        }

        // motor control
        let motor_state = app.motor_state();
        if motor_state != last_motor_state {
            match motor_state {
                MotorState::Stop => step_pwm.disable(Channel::C1),
                MotorState::CW(rpm) => {
                    step_dir.set_high().unwrap();
                    step_pwm.set_period(pulse_interval_us(rpm).us());
                    step_pwm.enable(Channel::C1);
                }
                MotorState::CCW(rpm) => {
                    step_dir.set_low().unwrap();
                    step_pwm.set_period(pulse_interval_us(rpm).us());
                    step_pwm.enable(Channel::C1);
                }
            }

            last_motor_state = motor_state
        }

        // draw display
        display.clear();
        app.draw(&mut display).unwrap();
        display.flush().unwrap();

        delay.delay_ms(25_u16);
    }
}

fn pulse_interval_us(rpm: u16) -> u32 {
    let pulses_per_second = (rpm as f32 / 60_f32) * MOTOR_STEPS_PER_REV as f32;
    (1000000_f32 / pulses_per_second) as u32
}

pub struct RotaryEncoder<PINS> {
    qei: Qei<TIM4, Tim4NoRemap, PINS>,
    last_count: u16,
}

impl<PINS> RotaryEncoder<PINS> {
    pub fn new(qei: Qei<TIM4, Tim4NoRemap, PINS>) -> Self {
        let last_count = qei.count();
        RotaryEncoder { qei, last_count }
    }

    pub fn poll(&mut self) -> Option<i16> {
        let count = self.qei.count();
        let diff = count.wrapping_sub(self.last_count) as i16;

        if diff.abs() >= 4 {
            self.last_count = count;
            Some(diff / 4)
        } else {
            None
        }
    }
}
