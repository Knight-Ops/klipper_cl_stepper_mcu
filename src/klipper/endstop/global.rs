use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, channel::Channel};

use super::EndstopMessage;

//TODO: Probably a better solution here is a static_cell/lazy_static of a hashmap that contains TRSYNC channels for a few, for the time being this fine
pub static ENDSTOP_CHANNEL: Channel<CriticalSectionRawMutex, EndstopMessage, 4> = Channel::new();
