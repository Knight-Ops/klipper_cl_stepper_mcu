use anchor::*;
use esp32c6_hal::prelude::_embedded_hal_digital_v2_OutputPin;
use esp32c6_hal::rmt::{TxChannelConfig, TxChannelCreator};
use heapless::Entry;

use crate::klipper::oid_types::*;

mod global;
mod message;
mod step_info;
mod task;

pub use global::*;
pub use message::StepperMessage;
pub use step_info::StepInfo;
use task::step_driver;

#[klipper_command]
pub fn queue_step(context: &mut crate::State, oid: u8, interval: u32, count: u16, add: i16) {
    log::trace!(
        "[ANCHOR] Queue Step - OID : {oid}, Interval : {interval}, count: {count}, add: {add}"
    );

    match context.oids.get_mut(&oid).unwrap() {
        OIDTypes::Stepper { _inner } => {
            embassy_futures::block_on(_inner.add_move_to_queue(interval, count, add));
        }
        _ => panic!("Expected OIDType::Stepper, but it is something else"),
    }
}

#[klipper_command]
pub fn set_next_step_dir(context: &mut crate::State, oid: u8, dir: u8) {
    log::trace!("[ANCHOR] Set Next Step Dir - OID : {oid}, dir: {dir}");

    match context.oids.get_mut(&oid).unwrap() {
        OIDTypes::Stepper { _inner } => _inner.set_dir(if dir != 0 { true } else { false }),
        _ => panic!("Expected OIDType::Stepper, but it is something else!"),
    }
}

#[klipper_command]
pub fn reset_step_clock(_context: &mut crate::State, oid: u8, clock: u32) {
    log::trace!("[ANCHOR] Reset Step Clock - OID : {oid}, clock: {clock}");
    embassy_futures::block_on(STEPPER_MOVE_QUEUE.send(StepperMessage::ResetStepClock));
}

#[klipper_command]
pub fn stepper_get_position(context: &mut crate::State, oid: u8) {
    log::trace!("[ANCHOR] Stepper Get Position - OID : {oid}");
    match context.oids.get(&oid).unwrap() {
        OIDTypes::Stepper { _inner } => {
            klipper_reply!(stepper_position, oid: u8 = oid, pos: i32 = STEPPER_POSITION.lock(|unlocked| {*unlocked.borrow()}));
        }
        _ => panic!("Expected OIDType::Stepper, but it is something else!"),
    }
}

#[klipper_command]
pub fn stepper_stop_on_trigger(context: &mut crate::State, oid: u8, trsync_oid: u8) {
    log::trace!("[ANCHOR] Stepper Stop On Trigger - oid: {oid}, trsync_oid: {trsync_oid}");
    match context.oids.get_mut(&trsync_oid).unwrap() {
        OIDTypes::TRSync { _inner } => {
            _inner.set_signal(&STEPPER_STOP);
        }
        _ => panic!("Expected OIDType::TRSyync, but it is something else!"),
    }
}

#[klipper_command]
pub fn config_stepper(
    context: &mut crate::State,
    oid: u8,
    step_pin: u32,
    dir_pin: u8,
    invert_step: u8,
    step_pulse_ticks: u32,
) {
    log::trace!("[ANCHOR] Config Stepper - oid: {oid}, step_pin: {step_pin}, dir_pin: {dir_pin}, invert_step: {invert_step}, step_pulse_ticks: {step_pulse_ticks}");

    let mut step = context.step.take().unwrap().into_push_pull_output();
    step.set_low().unwrap();
    let mut dir = context.dir.take().unwrap().into_push_pull_output();

    if invert_step != 0 {
        dir.set_high().unwrap();
    } else {
        dir.set_low().unwrap();
    }

    #[cfg(feature = "rmt_step")]
    {
        let channel = context.rmt.take().unwrap().channel0;

        let config = TxChannelConfig {
            clk_divider: 1,
            idle_output_level: if invert_step > 0 { true } else { false },
            carrier_modulation: false,
            idle_output: false,

            ..TxChannelConfig::default()
        };

        let channel = channel.configure(step, config).unwrap();

        log::debug!("Step Driver Task");
        context
            .spawner
            .spawn(step_driver(
                // step,
                channel,
                dir,
                if invert_step == 0 { true } else { false },
                step_pulse_ticks,
                STEPPER_MOVE_QUEUE.receiver(),
            ))
            .unwrap();
    }

    #[cfg(not(feature = "rmt_step"))]
    {
        log::debug!("Step Driver Task");
        context
            .spawner
            .spawn(step_driver(
                step,
                dir,
                if invert_step == 0 { true } else { false },
                step_pulse_ticks,
                STEPPER_MOVE_QUEUE.receiver(),
            ))
            .unwrap();
    }

    match context.oids.entry(oid) {
        Entry::Occupied(mut o) => {
            log::trace!("[ANCHOR] Reconfiguring configured OID to Stepper Entry");
            let oid_entry = o.get_mut();
            *oid_entry = OIDTypes::Stepper {
                _inner: Stepper::new(STEPPER_MOVE_QUEUE.sender(), true),
            }
        }
        Entry::Vacant(v) => {
            let _ = v.insert(OIDTypes::Stepper {
                _inner: Stepper::new(STEPPER_MOVE_QUEUE.sender(), true),
            });
        }
    }
}
