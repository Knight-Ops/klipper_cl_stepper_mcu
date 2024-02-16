use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use super::CLMonitorMessage;

// pub static TRIGGER_MAGNET_READ: Signal<CriticalSectionRawMutex, ()> = Signal::new();
// static MAGNET_SENSOR: Signal<CriticalSectionRawMutex, u16> = Signal::new();
// pub static CALIBRATE: Signal<CriticalSectionRawMutex, ()> = Signal::new();

pub static CL_MONITOR_CHANNEL: Channel<CriticalSectionRawMutex, CLMonitorMessage, 64> =
    Channel::new();
