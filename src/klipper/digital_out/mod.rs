use anchor::*;
use embedded_hal::digital::OutputPin;
use heapless::Entry;

use crate::klipper::oid_types::*;

mod task;
use task::execute_digital_out;

#[klipper_command]
pub fn queue_digital_out(context: &mut crate::State, oid: u8, clock: u32, on_ticks: u32) {
    log::trace!("[ANCHOR] Queue Digital Out - OID : {oid}, Clock : {clock}, on_ticks: {on_ticks}");

    match context.oids.get_mut(&oid).unwrap() {
        OIDTypes::DigitalOut { _inner } => {
            let pin = _inner.get_pin_clone();

            context
                .spawner
                .spawn(execute_digital_out(pin, clock, on_ticks))
                .unwrap();
        }
        _ => panic!("Expected to find DigitalOut OID, found something else"),
    }
}

#[klipper_command]
pub fn config_digital_out(
    context: &mut crate::State,
    oid: u8,
    pin: u32,
    value: u8,
    default_value: u8,
    max_duration: u32,
) {
    log::trace!("[ANCHOR] Config Digital Out - oid: {oid}, pin: {pin}, value: {value}, default_value: {default_value}, max_duration: {max_duration}");

    let mut pin = context
        .enable_stepper
        .take()
        .unwrap()
        .into_push_pull_output();

    if default_value != 0 {
        pin.set_high().unwrap();
    } else {
        pin.set_low().unwrap();
    }

    match context.oids.entry(oid) {
        Entry::Occupied(mut o) => {
            log::trace!("[ANCHOR] Reconfiguring configured OID to Digital Out Entry");
            let oid_entry = o.get_mut();
            *oid_entry = OIDTypes::DigitalOut {
                _inner: DigitalOut::new(
                    pin,
                    if value != 0 { true } else { false },
                    if default_value != 0 { true } else { false },
                    max_duration,
                ),
            }
        }
        Entry::Vacant(v) => {
            let _ = v.insert(OIDTypes::DigitalOut {
                _inner: DigitalOut::new(
                    pin,
                    if value != 0 { true } else { false },
                    if default_value != 0 { true } else { false },
                    max_duration,
                ),
            });
        }
    }
}
