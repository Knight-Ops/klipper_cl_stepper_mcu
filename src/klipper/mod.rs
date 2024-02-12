use anchor::*;
// use core::borrow::BorrowMut;
use core::cell::RefCell;
// use critical_section::Mutex;
use crate::State;
use embassy_sync::blocking_mutex::{raw::CriticalSectionRawMutex, Mutex};
use embassy_sync::signal::Signal;
use embassy_time::Instant;

// pub mod commands;
pub mod digital_out;
pub mod endstop;
pub mod oid_types;
pub mod stepper;
pub mod tmc_uart;
pub mod trsync;

pub const USB_MAX_PACKET_SIZE: usize = 64;

pub static USB_READY_TO_SEND: Signal<CriticalSectionRawMutex, bool> = Signal::new();
pub static USB_TX_BUFFER: Mutex<
    CriticalSectionRawMutex,
    RefCell<FifoBuffer<{ USB_MAX_PACKET_SIZE * 2 }>>,
> = Mutex::new(RefCell::new(FifoBuffer::new()));
pub(crate) struct BufferTransportOutput;

impl TransportOutput for BufferTransportOutput {
    type Output = ScratchOutput;
    fn output(&self, f: impl FnOnce(&mut Self::Output)) {
        let mut scratch = ScratchOutput::new();
        f(&mut scratch);
        let output = scratch.result();

        USB_TX_BUFFER.lock(|unlocked| {
            let mut buf = unlocked.borrow_mut();
            buf.extend(output);
        });

        USB_READY_TO_SEND.signal(true);
    }
}

pub(crate) const TRANSPORT_OUTPUT: BufferTransportOutput = BufferTransportOutput;

#[klipper_constant]
const CLOCK_FREQ: u32 = 16_000_000;

#[klipper_constant]
const MCU: &str = "ESP32C6-Test";

#[klipper_constant]
const STATS_SUMSQ_BASE: u32 = 256;

#[klipper_constant]
const STEPPER_BOTH_EDGES: u32 = 1;

klipper_enumeration! {
    #[derive(Debug)]
    #[klipper_enumeration(name = "pin", rename_all = "UPPERCASE")]
    enum Pins {
        Range(GPIO, 0, 12),
        Range(GPIO, 14, 21)
    }
}

#[klipper_command]
pub fn emergency_stop() {
    log::trace!("[ANCHOR] Emergency Stop");
}

#[klipper_command]
pub fn get_config(context: &State) {
    log::trace!("[ANCHOR] Get Config");
    let crc = context.config_crc;
    klipper_reply!(
        config,
        is_config: bool = crc.is_some(),
        crc: u32 = crc.unwrap_or(0),
        is_shutdown: bool = false,
        move_count: u16 = context.move_queue
    );
}

#[klipper_command]
pub fn config_reset(context: &mut State) {
    log::trace!("[ANCHOR] Config Reset");
    context.config_crc = None;
}

#[klipper_command]
pub fn finalize_config(context: &mut State, crc: u32) {
    log::trace!("[ANCHOR] Finalize Config");
    context.config_crc = Some(crc);
}

#[klipper_command]
pub fn allocate_oids(_context: &mut State, count: u8) {
    log::trace!("[ANCHOR] Allocate OIDs - Count : {}", count);
    if count > crate::klipper::oid_types::MAX_NUMBER_OIDS {
        klipper_output!(
            "[ERROR] Attempting to allocate more than the max allowable OIDs on this MCU."
        )
    }
}

#[klipper_command]
pub fn get_uptime(_context: &mut crate::State) {
    log::trace!("[ANCHOR] Get uptime");
    let c = Instant::now().as_ticks();
    klipper_reply!(
        uptime,
        high: u32 = (c >> 32) as u32,
        clock: u32 = (c & 0xFFFF_FFFF) as u32
    );
}

#[klipper_command]
pub fn get_clock(_context: &mut crate::State) {
    // log::trace!("[ANCHOR] Get clock");
    klipper_reply!(clock, clock: u32 = Instant::now().as_ticks() as u32);
}
