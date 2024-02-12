use anchor::*;
use embassy_time::Instant;

use heapless::Entry;

use crate::klipper::oid_types::*;

mod global;
mod message;
mod task;

pub use global::*;
pub use message::TRSyncMessage;
use task::trsync_runner;

#[klipper_command]
pub fn trsync_start(
    context: &mut crate::State,
    oid: u8,
    report_clock: u32,
    report_ticks: u32,
    expire_reason: u8,
) {
    log::trace!("[ANCHOR] TRSync Start - oid: {oid}, report_clock: {report_clock}, report_ticks: {report_ticks}, expire_reason: {expire_reason}");

    // Drain the TRSYNC Channel before we launch a new TRSync
    while TRSYNC_CHANNEL.try_receive().is_ok() {}
    context
        .spawner
        .spawn(trsync_runner(
            oid,
            report_clock,
            report_ticks,
            expire_reason,
        ))
        .unwrap();
}

#[klipper_command]
pub fn trsync_set_timeout(context: &mut crate::State, oid: u8, clock: u32) {
    log::trace!("[ANCHOR] TRSync Set Timeout - oid: {oid}, clock: {clock}");

    match context.oids.get(&oid).unwrap() {
        OIDTypes::TRSync { _inner } => _inner.blocking_send(TRSyncMessage::SetTimeout {
            to: Instant::from_ticks(clock as u64),
        }),
        _ => panic!("Expected OID to be a TRSync, but it wasn't!"),
    }
}

#[klipper_command]
pub fn trsync_trigger(context: &mut crate::State, oid: u8, reason: u8) {
    log::trace!("[ANCHOR] TRSync Trigger - oid: {oid}, reason: {reason}");

    match context.oids.get(&oid).unwrap() {
        OIDTypes::TRSync { _inner } => match reason {
            // Endstop Hit
            1 => _inner.signal(true),
            // Comms timeout
            2 => trsync_report(oid, 0, reason, 0),
            // Host Request
            3 => _inner.blocking_send(TRSyncMessage::HostRequest),
            // Past End time
            4 => _inner.blocking_send(TRSyncMessage::EndTask),
            _ => unreachable!("These TRSync Trigger reasons are not currently defined"),
        },
        _ => panic!("Expected OID to be a TRSync, but it wasn't!"),
    }
}

fn trsync_report(oid: u8, can_trigger: u8, trigger_reason: u8, clock: u32) {
    log::trace!("Sending trsync_report : {oid} {can_trigger}, {trigger_reason}, {clock}");
    klipper_reply!(trsync_state, oid: u8 = oid, can_trigger:u8 = can_trigger, trigger_reason: u8 = trigger_reason, clock: u32 = clock)
}

#[klipper_command]
pub fn config_trsync(context: &mut crate::State, oid: u8) {
    log::trace!("[ANCHOR] Config TRSync - oid : {oid}");

    let trsync = TRSync::new();

    match context.oids.entry(oid) {
        Entry::Occupied(mut o) => {
            log::trace!("[ANCHOR] Reconfiguring configured OID to TRsync Entry");
            let oid_entry = o.get_mut();
            *oid_entry = OIDTypes::TRSync { _inner: trsync }
        }
        Entry::Vacant(v) => {
            let _ = v.insert(OIDTypes::TRSync { _inner: trsync });
        }
    }
}
