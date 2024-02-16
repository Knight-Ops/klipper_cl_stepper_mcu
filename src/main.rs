#![no_std]
#![no_main]
#![feature(type_alias_impl_trait)]

use anchor::{klipper_config_generate, SliceInputBuffer};
use as5600_async::{status::Status, As5600};
use embassy_executor::{Executor, Spawner};
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, signal::Signal};
use embassy_time::{Duration, Timer};
use embedded_io::Write;
use embedded_io_async::{Read as AsyncRead, Write as AsyncWrite};
use esp32c6_hal::{
    clock::ClockControl,
    embassy, entry,
    gpio::{GpioPin, Unknown, IO},
    i2c::I2C,
    peripherals::{Peripherals, I2C0, UART1},
    prelude::*,
    rmt::Channel,
    system::SystemExt,
    systimer::SystemTimer,
    usb_serial_jtag::{UsbSerialJtagRx, UsbSerialJtagTx},
    Rmt, Uart, UsbSerialJtag,
};
use esp_backtrace as _;
use klipper::{stepper::STEPPER_POSITION, USB_MAX_PACKET_SIZE, USB_READY_TO_SEND};
use smart_leds::RGB8;
use smart_leds_trait::SmartLedsWrite;
use static_cell::StaticCell;

mod ws2812_driver;
// use ws2812_driver::Ws2812;
mod board;
mod cl_monitor;
mod klipper;

use cl_monitor::closed_loop_monitor;

#[cfg(feature = "task_tracing")]
mod rtos_trace_log;

static EXECUTOR: StaticCell<Executor> = StaticCell::new();

const MOVE_QUEUE: u16 = 0x800;

#[entry]
fn main() -> ! {
    let peripherals = Peripherals::take();
    let system = peripherals.SYSTEM.split();

    let clocks = ClockControl::max(system.clock_control).freeze();
    // let timer = TimerGroup::new(peripherals.TIMG0, &clocks);
    let timer = SystemTimer::new(peripherals.SYSTIMER);
    embassy::init(&clocks, timer);

    // setup logger
    // To change the log_level change the env section in .cargo/config.toml
    // or remove it and set ESP_LOGLEVEL manually before running cargo run
    // this requires a clean rebuild because of https://github.com/rust-lang/cargo/issues/10358
    esp_println::logger::init_logger_from_env();
    log::info!("Logger is setup");

    let io = IO::new(peripherals.GPIO, peripherals.IO_MUX);
    // GPIO 4 as output
    // let led = io.pins.gpio4.into_push_pull_output();
    let rmt = esp32c6_hal::rmt::Rmt::new(peripherals.RMT, 80u32.MHz(), &clocks).unwrap();
    // let ws_driver = ws2812_driver::SmartLedsAdapter::new(
    //     rmt.channel0,
    //     io.pins.gpio8.into_open_drain_output(),
    //     smartLedBuffer!(1),
    // );

    // ---- Configure single wire Uart for TMC driver

    let config = esp32c6_hal::uart::config::Config {
        baudrate: 115200,
        data_bits: esp32c6_hal::uart::config::DataBits::DataBits8,
        parity: esp32c6_hal::uart::config::Parity::ParityNone,
        stop_bits: esp32c6_hal::uart::config::StopBits::STOP1,
    };

    use esp32c6_hal::uart::TxRxPins;
    let pins = TxRxPins::new_tx_rx(
        io.pins.gpio21.into_push_pull_output(),
        io.pins.gpio20.into_floating_input(),
    );

    let tmc_serial =
        esp32c6_hal::uart::Uart::new_with_config(peripherals.UART1, config, Some(pins), &clocks);

    // ---- End

    let i2c = I2C::new(
        peripherals.I2C0,
        io.pins.gpio23,
        io.pins.gpio22,
        esp32c6_hal::prelude::_fugit_RateExtU32::MHz(1),
        &clocks,
    );

    let as5600_driver = As5600::new(i2c);
    // let usb_serial = UsbSerialJtag::new(peripherals.USB_DEVICE);
    let (usb_tx, usb_rx) = UsbSerialJtag::new(peripherals.USB_DEVICE).split();

    let executor = EXECUTOR.init(Executor::new());
    executor.run(|spawner| {
        // spawner.spawn(onboard_rgb_led(ws_driver)).ok();
        log::debug!("CL Task");
        spawner.spawn(closed_loop_monitor(as5600_driver)).ok();
        log::debug!("USB Writer");
        spawner.spawn(usb_writer(usb_tx)).ok();
        log::debug!("USB Reader");
        spawner
            .spawn(usb_reader(
                usb_rx,
                State {
                    spawner,
                    oids: heapless::FnvIndexMap::new(),
                    move_queue: MOVE_QUEUE,
                    config_crc: None,
                    tmc_serial: Some(tmc_serial),
                    endstop_pin: Some(io.pins.gpio7),
                    enable_stepper: Some(io.pins.gpio4),
                    rmt: Some(rmt),
                    step: Some(io.pins.gpio5),
                    dir: Some(io.pins.gpio6),
                },
            ))
            .ok();
    })
}

klipper_config_generate!(
    transport = crate::klipper::TRANSPORT_OUTPUT: crate::klipper::BufferTransportOutput,
    context = &'ctx mut crate::State,
);

pub struct State {
    spawner: Spawner,
    oids: heapless::FnvIndexMap<
        u8,
        klipper::oid_types::OIDTypes<'static>,
        { klipper::oid_types::MAX_NUMBER_OIDS as usize },
    >,
    move_queue: u16,
    config_crc: Option<u32>,
    tmc_serial: Option<Uart<'static, UART1>>,
    endstop_pin: Option<GpioPin<Unknown, 7>>,
    enable_stepper: Option<GpioPin<Unknown, 4>>,
    rmt: Option<Rmt<'static>>,
    step: Option<GpioPin<Unknown, 5>>,
    dir: Option<GpioPin<Unknown, 6>>,
}

#[embassy_executor::task]
async fn usb_reader(mut usb_serial: UsbSerialJtagRx<'static>, mut mcu_state: State) {
    let mut usb_buffer = [0; USB_MAX_PACKET_SIZE * 2];
    loop {
        let read_bytes = AsyncRead::read(&mut usb_serial, &mut usb_buffer)
            .await
            .unwrap();

        if read_bytes > 0 {
            let mut klipper_wrap = SliceInputBuffer::new(&usb_buffer[0..read_bytes]);
            KLIPPER_TRANSPORT.receive(&mut klipper_wrap, &mut mcu_state);
        }

        // We probably don't need to do this since we only ever write the read_bytes count
        // usb_buffer.fill(0);
    }
}

#[embassy_executor::task]
async fn usb_writer(mut usb_tx: UsbSerialJtagTx<'static>) {
    loop {
        USB_READY_TO_SEND.wait().await;

        klipper::USB_TX_BUFFER.lock(|unlocked| {
            let mut txbuf = unlocked.borrow_mut();

            if !txbuf.is_empty() {
                let written_bytes = embedded_io::Write::write(&mut usb_tx, txbuf.data()).unwrap();

                txbuf.pop(written_bytes);
            }
        });
        USB_READY_TO_SEND.reset();
    }
}

// pub static TRIGGER_MAGNET_READ: Signal<CriticalSectionRawMutex, ()> = Signal::new();
// static MAGNET_SENSOR: Signal<CriticalSectionRawMutex, u16> = Signal::new();
// pub static CALIBRATION_ANGLE: Signal<CriticalSectionRawMutex, u16> = Signal::new();

// #[embassy_executor::task]
// async fn as5600_task(mut driver: As5600<I2C<'static, I2C0>>) {
//     loop {
//         match driver.magnet_status().await {
//             Ok(state) => match state {
//                 Status::MagnetDetected => {
//                     log::info!("Magnet detected");
//                     break;
//                 }
//                 _ => {
//                     log::error!("Magnet not detected, or detected with error - {state:?}")
//                 }
//             },
//             Err(e) => {
//                 log::error!("Error with magnet detection occured : {:?}", e);
//             }
//         }
//     }
//     const DEG_PER_TICK: f32 = 360. / 4096.;
//     const DEG_PER_STEP: f32 = 360. / 3200.;
//     let start_angle = driver.angle().await.unwrap();

//     // This needs work for calcs
//     loop {
//         TRIGGER_MAGNET_READ.wait().await;
//         TRIGGER_MAGNET_READ.reset();
//         let angle = driver.angle().await.unwrap();
//         let pos = STEPPER_POSITION.lock(|unlocked| *unlocked.borrow()).abs() % 3200;
//         log::info!(
//             "Magnet sensor reading : {} | pos : {}",
//             (angle.abs_diff(start_angle)) as f32 * DEG_PER_TICK,
//             pos as f32 * DEG_PER_STEP,
//         );
//         MAGNET_SENSOR.signal(angle);
//     }
// }

#[embassy_executor::task]
async fn onboard_rgb_led(
    mut rgb_driver: ws2812_driver::SmartLedsAdapter<esp32c6_hal::rmt::Channel<0>, 25>,
) {
    rgb_driver
        .write([RGB8::from((0, 0, 0)); 1].iter().cloned())
        .ok();

    let mut led_effect_driver = LedEffect::new(crate::LedEffects::Rainbow {
        state: RainbowEffectStateMachine::IncRed,
        max_brightness: 16,
        delay_ms: 100,
    });

    let mut rgb = RGB8::from((0, 0, 0));
    loop {
        rgb_driver.write([rgb; 1].iter().cloned()).ok();

        led_effect_driver.next(&mut rgb).await;
    }
}

pub struct LedEffect {
    effect: LedEffects,
}

impl LedEffect {
    pub fn new(effect: LedEffects) -> Self {
        LedEffect { effect }
    }

    pub async fn next(&mut self, rgb: &mut RGB8) {
        match self.effect {
            _ => self.effect.next(rgb).await,
        }
    }
}

#[derive(Default)]
pub enum LedEffects {
    #[default]
    None,
    Rainbow {
        state: RainbowEffectStateMachine,
        max_brightness: u8,
        delay_ms: u64,
    },
}

impl LedEffects {
    async fn next(&mut self, rgb: &mut RGB8) {
        match self {
            Self::None => {}
            Self::Rainbow {
                state,
                max_brightness,
                delay_ms,
            } => {
                match state {
                    RainbowEffectStateMachine::IncRed => {
                        if rgb.r == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.r += 1;
                        }
                    }
                    RainbowEffectStateMachine::IncGreen => {
                        if rgb.g == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.g += 1;
                        }
                    }
                    RainbowEffectStateMachine::IncBlue => {
                        if rgb.b == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.b += 1;
                        }
                    }
                    RainbowEffectStateMachine::DecRed => {
                        if rgb.r == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.r -= 1;
                        }
                    }
                    RainbowEffectStateMachine::DecGreen => {
                        if rgb.g == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.g -= 1;
                        }
                    }
                    RainbowEffectStateMachine::DecBlue => {
                        if rgb.b == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.b -= 1;
                        }
                    }
                };
                Timer::after(Duration::from_millis(*delay_ms)).await;
            }
        }
    }
}

#[derive(Default)]
pub enum RainbowEffectStateMachine {
    #[default]
    IncRed,
    IncGreen,
    IncBlue,
    DecRed,
    DecGreen,
    DecBlue,
}

impl RainbowEffectStateMachine {
    fn next_state(&self) -> Self {
        match self {
            RainbowEffectStateMachine::IncRed => RainbowEffectStateMachine::DecGreen,
            RainbowEffectStateMachine::DecGreen => RainbowEffectStateMachine::IncBlue,
            RainbowEffectStateMachine::IncBlue => RainbowEffectStateMachine::DecRed,
            RainbowEffectStateMachine::DecRed => RainbowEffectStateMachine::IncGreen,
            RainbowEffectStateMachine::IncGreen => RainbowEffectStateMachine::DecBlue,
            RainbowEffectStateMachine::DecBlue => RainbowEffectStateMachine::IncRed,
        }
    }
}
