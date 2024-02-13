use core::cell::RefCell;

use embassy_sync::{
    self,
    blocking_mutex::{raw::CriticalSectionRawMutex, Mutex},
    channel::Channel,
    priority_channel::{Max, PriorityChannel},
    signal::Signal,
};

use super::StepperMessage;

// I don't love this idea, we may be able to move it to the `State` using a NoopRawMutex
// pub static STEPPER_MOVE_QUEUE: Channel<
//     CriticalSectionRawMutex,
//     StepperMessage,
//     { crate::MOVE_QUEUE as usize },
// > = Channel::new();

pub static STEPPER_MOVE_QUEUE: PriorityChannel<
    CriticalSectionRawMutex,
    StepperMessage,
    Max,
    { crate::MOVE_QUEUE as usize },
> = PriorityChannel::new();

pub static STEPPER_POSITION: Mutex<CriticalSectionRawMutex, RefCell<i32>> =
    Mutex::new(RefCell::new(0));
pub static STEPPER_STOP: Signal<CriticalSectionRawMutex, bool> = Signal::new();
