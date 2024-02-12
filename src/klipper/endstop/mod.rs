use anchor::*;
use heapless::Entry;

use embassy_time::Instant;

use crate::klipper::oid_types::*;

mod endstop_pin;
mod global;
mod message;
mod task;

use endstop_pin::EndstopPin;
pub use global::*;
use message::EndstopMessage;
use task::endstop_runner;

#[klipper_command]
pub fn endstop_home(
    context: &mut crate::State,
    oid: u8,
    clock: u32,
    sample_ticks: u32,
    sample_count: u8,
    rest_ticks: u32,
    pin_value: u8,
    trsync_oid: u8,
    trigger_reason: u8,
) {
    log::trace!("[ANCHOR] Endstop Home - OID : {oid}, Clock : {clock}, sample_ticks: {sample_ticks}, sample_count: {sample_count}, rest_ticks: {rest_ticks}, pin_value: {pin_value}, trsync_oid: {trsync_oid}, trigger_reason: {trigger_reason}");

    if clock == 0
        && sample_ticks == 0
        && sample_count == 0
        && rest_ticks == 0
        && trsync_oid == 0
        && trigger_reason == 0
    {
        embassy_futures::block_on(ENDSTOP_CHANNEL.send(EndstopMessage::EndTask));
        return;
    }

    match context.oids.get_mut(&oid).unwrap() {
        OIDTypes::Endstop { _inner } => {
            context
                .spawner
                .spawn(endstop_runner(
                    clock,
                    sample_ticks,
                    sample_count,
                    rest_ticks,
                    pin_value,
                    trsync_oid,
                    trigger_reason,
                    EndstopPin::Floating(_inner.get_pin_clone()),
                ))
                .unwrap();
        }
        OIDTypes::EndstopPullup { _inner } => {
            context
                .spawner
                .spawn(endstop_runner(
                    clock,
                    sample_ticks,
                    sample_count,
                    rest_ticks,
                    pin_value,
                    trsync_oid,
                    trigger_reason,
                    EndstopPin::PullUp(_inner.get_pin_clone()),
                ))
                .unwrap();
        }
        _ => panic!("Expected OID to be a Endstop or EndstopPullup, but it wasn't!"),
    };
}

#[klipper_command]
pub fn endstop_query_state(context: &mut crate::State, oid: u8) {
    log::trace!("[ANCHOR] Endstop Query State - OID : {oid}");

    match context.oids.get(&oid).unwrap() {
        OIDTypes::Endstop { _inner } => {
            log::trace!(
                "Sending endstop_state : {oid}, {}, {}, {}",
                _inner.is_homing(),
                _inner.next_clock(),
                _inner.get_pin_val()
            );
            klipper_reply!(endstop_state, oid: u8 = oid, homing: u8 = _inner.is_homing() as u8, next_clock:u32 = Instant::now().as_ticks() as u32, pin_value: u8 = _inner.get_pin_val() as u8)
        }
        OIDTypes::EndstopPullup { _inner } => {
            log::trace!(
                "Sending endstop_state : {oid}, {}, {}, {}",
                _inner.is_homing(),
                _inner.next_clock(),
                _inner.get_pin_val()
            );
            klipper_reply!(endstop_state, oid: u8 = oid, homing: u8 = _inner.is_homing() as u8, next_clock:u32 = Instant::now().as_ticks() as u32, pin_value: u8 = _inner.get_pin_val() as u8)
        }
        _ => panic!("Expected OID to be a Endstop or EndstopPullup, but it wasn't!"),
    }
}

#[klipper_command]
pub fn config_endstop(context: &mut crate::State, oid: u8, pin: u8, pull_up: u8) {
    log::trace!("[ANCHOR] Config Endstop - oid: {oid}, pin: {pin}, pull_up: {pull_up}");

    let endstop_pin = context.endstop_pin.take().unwrap();

    if pull_up == 1 {
        let pull_up = endstop_pin.into_pull_up_input();

        match context.oids.entry(oid) {
            Entry::Occupied(mut o) => {
                log::trace!("[ANCHOR] Reconfiguring configured OID to Endspot Entry");
                let oid_entry = o.get_mut();
                *oid_entry = OIDTypes::EndstopPullup {
                    _inner: EndstopPullup::new(pull_up),
                }
            }
            Entry::Vacant(v) => {
                let _ = v.insert(OIDTypes::EndstopPullup {
                    _inner: EndstopPullup::new(pull_up),
                });
            }
        }
    } else {
        // TODO: I am not sure if floating is the assumed way to do this?
        let floating = endstop_pin.into_floating_input();
        match context.oids.entry(oid) {
            Entry::Occupied(mut o) => {
                log::trace!("[ANCHOR] Reconfiguring configured OID to Endspot Entry");
                let oid_entry = o.get_mut();
                *oid_entry = OIDTypes::Endstop {
                    _inner: Endstop::new(floating),
                }
            }
            Entry::Vacant(v) => {
                let _ = v.insert(OIDTypes::Endstop {
                    _inner: Endstop::new(floating),
                });
            }
        }
    }
}
