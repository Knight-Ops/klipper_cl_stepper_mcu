use core::cell::RefCell;

use embassy_sync::{
    self,
    blocking_mutex::{raw::CriticalSectionRawMutex, Mutex},
    channel::Channel,
    signal::Signal,
};

use super::{StepInfo, StepperMessage};

// I don't love this idea, we may be able to move it to the `State` using a NoopRawMutex
pub static STEPPER_MOVE_QUEUE: Channel<
    CriticalSectionRawMutex,
    StepperMessage,
    { crate::MOVE_QUEUE as usize },
> = Channel::new();

pub static STEPPER_POSITION: Mutex<CriticalSectionRawMutex, RefCell<i32>> =
    Mutex::new(RefCell::new(0));
pub static STEPPER_STOP: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static STEP_CORRECTION: Signal<CriticalSectionRawMutex, StepInfo> = Signal::new();
