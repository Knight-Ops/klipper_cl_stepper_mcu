use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Sender, signal::Signal};
use embassy_time::Instant;
use embedded_io::{Read, Write};
use esp32c6_hal::{gpio::InputPin, peripheral::Peripheral};

use super::trsync::TRSYNC_CHANNEL;
use crate::klipper::stepper::StepInfo;

pub const MAX_NUMBER_OIDS: u8 = 128;

pub enum OIDTypes<'a> {
    TMCUart { _inner: TMCUart<'a> },
    Stepper { _inner: Stepper },
    DigitalOut { _inner: DigitalOut },
    Endstop { _inner: Endstop },
    EndstopPullup { _inner: EndstopPullup },
    TRSync { _inner: TRSync },
}

pub struct TMCUart<'a> {
    pull_up: bool,
    uart: esp32c6_hal::uart::Uart<'a, esp32c6_hal::peripherals::UART1>,
    bit_time: u32,
}

impl<'a> TMCUart<'a> {
    pub fn new(
        uart: esp32c6_hal::Uart<'a, esp32c6_hal::peripherals::UART1>,
        pull_up: bool,
        bit_time: u32,
    ) -> Self {
        Self {
            uart,
            pull_up,
            bit_time,
        }
    }

    pub fn send(&mut self, buf: &[u8]) -> Result<usize, esp32c6_hal::uart::Error> {
        self.uart.write(buf)
    }

    pub fn read_exact(&mut self, buf: &mut [u8]) -> Result<usize, esp32c6_hal::uart::Error> {
        self.uart.read_exact(buf).unwrap();
        Ok(buf.len())
    }
}

// TODO: Position really shouldn't be a global Signal, it should be something we can put in this struct
pub struct Stepper {
    step_channel: Sender<
        'static,
        CriticalSectionRawMutex,
        crate::klipper::stepper::StepperMessage,
        { crate::MOVE_QUEUE as usize },
    >,
    // step_channel: embassy_sync::priority_channel::Sender<
    //     'static,
    //     CriticalSectionRawMutex,
    //     crate::klipper::stepper::StepperMessage,
    //     embassy_sync::priority_channel::Max,
    //     { crate::MOVE_QUEUE as usize },
    // >,
    dir: bool,
    position: i32,
}

impl Stepper {
    pub fn new(
        step_channel: Sender<
            'static,
            CriticalSectionRawMutex,
            crate::klipper::stepper::StepperMessage,
            { crate::MOVE_QUEUE as usize },
        >,
        // step_channel: embassy_sync::priority_channel::Sender<
        //     'static,
        //     CriticalSectionRawMutex,
        //     crate::klipper::stepper::StepperMessage,
        //     embassy_sync::priority_channel::Max,
        //     { crate::MOVE_QUEUE as usize },
        // >,
        dir: bool,
    ) -> Self {
        Self {
            step_channel,
            dir,
            position: 0,
        }
    }

    pub fn set_dir(&mut self, dir: bool) {
        self.dir = dir
    }

    pub async fn add_move_to_queue(&mut self, interval: u32, count: u16, add: i16) {
        self.step_channel
            .send(crate::klipper::stepper::StepperMessage::StepInfo {
                _inner: StepInfo::new(interval, count, add, self.dir),
            })
            .await
    }
}

pub struct DigitalOut {
    pin: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>, 4>,
    value: bool,
    default_value: bool,
    max_duration: u32,
}

impl DigitalOut {
    pub fn new(
        pin: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>, 4>,
        value: bool,
        default_value: bool,
        max_duration: u32,
    ) -> Self {
        Self {
            pin,
            value,
            default_value,
            max_duration,
        }
    }

    pub fn get_pin_clone(
        &mut self,
    ) -> esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>, 4> {
        unsafe { self.pin.clone_unchecked() }
    }
}

pub struct EndstopPullup {
    es: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::PullUp>, 7>,
    homing: bool,
    clock: Instant,
}

impl EndstopPullup {
    pub fn new(
        es: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::PullUp>, 7>,
    ) -> Self {
        Self {
            es,
            homing: false,
            clock: Instant::from_ticks(0),
        }
    }

    pub fn get_pin_val(&self) -> bool {
        self.es.is_input_high()
    }

    pub fn is_homing(&self) -> bool {
        self.homing
    }

    pub fn next_clock(&self) -> u32 {
        self.clock.as_ticks() as u32
    }

    pub fn get_pin_clone(
        &mut self,
    ) -> esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::PullUp>, 7> {
        unsafe { self.es.clone_unchecked() }
    }
}

pub struct Endstop {
    es: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::Floating>, 7>,
    homing: bool,
    clock: Instant,
}

impl Endstop {
    pub fn new(
        es: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::Floating>, 7>,
    ) -> Self {
        Self {
            es,
            homing: false,
            clock: Instant::from_ticks(0),
        }
    }

    pub fn get_pin_val(&self) -> bool {
        self.es.is_input_high()
    }

    pub fn is_homing(&self) -> bool {
        self.homing
    }

    pub fn next_clock(&self) -> u32 {
        self.clock.as_ticks() as u32
    }

    pub fn get_pin_clone(
        &mut self,
    ) -> esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::Floating>, 7> {
        unsafe { self.es.clone_unchecked() }
    }
}

pub struct TRSync {
    triggering_signal: Option<&'static Signal<CriticalSectionRawMutex, bool>>,
    signal_to_alert: Option<&'static Signal<CriticalSectionRawMutex, bool>>,
}

impl TRSync {
    pub fn new() -> Self {
        Self {
            triggering_signal: None,
            signal_to_alert: None,
        }
    }

    pub fn set_signal(&mut self, sig: &'static Signal<CriticalSectionRawMutex, bool>) {
        self.signal_to_alert = Some(sig);
    }

    pub fn signal(&self, val: bool) {
        if let Some(sig) = self.signal_to_alert {
            sig.signal(val);
        }
    }

    pub fn blocking_send(&self, data: crate::klipper::trsync::TRSyncMessage) {
        embassy_futures::block_on(TRSYNC_CHANNEL.send(data));
    }
}
