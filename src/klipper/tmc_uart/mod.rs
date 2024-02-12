use anchor::*;
use heapless::Entry;

use crate::klipper::oid_types::*;
use crate::State;

mod helper;
use helper::{clad_uart_protocol_bits, strip_uart_protocol_bits};

#[klipper_command]
/// TX and RX pins are completely ignored, it doesn't matter what you set them to
pub fn config_tmcuart(
    context: &mut State,
    oid: u8,
    rx_pin: u32,
    pull_up: u8,
    tx_pin: u32,
    bit_time: u32,
) {
    log::trace!("[ANCHOR] Config TMC UART - oid: {oid}, rx_pin: {rx_pin}, pull_up: {pull_up}, tx_pin: {tx_pin}, bit_time: {bit_time}");

    let tmc_uart = TMCUart::new(
        context.tmc_serial.take().unwrap(),
        if pull_up == 1 { true } else { false },
        bit_time,
    );

    match context.oids.entry(oid) {
        Entry::Occupied(mut o) => {
            log::trace!("[ANCHOR] Reconfiguring configured OID to TMC Uart Entry");
            let oid_entry = o.get_mut();
            *oid_entry = OIDTypes::TMCUart { _inner: tmc_uart }
        }
        Entry::Vacant(v) => {
            let _ = v.insert(OIDTypes::TMCUart { _inner: tmc_uart });
        }
    }
}

// TODO: This could use a rework. I don't like having hard coded buffer sizes, but it works for the time being
// and I don't think there could really ever be another message size based on the TMC2209 protocol
#[klipper_command]
pub fn tmcuart_send(context: &mut State, oid: u8, write: &[u8], read: u8) {
    // log::trace!("[ANCHOR] TMC UART Send - oid: {oid}, write: {write:X?}, read: {read}");
    let mut uart_bytes = [0; 8];
    let corrected_length = strip_uart_protocol_bits(&mut uart_bytes, write);

    // log::trace!(
    //     "[ANCHOR] TMC UART Send - real_bytes : {:X?}",
    //     &uart_bytes[0..corrected_length]
    // );

    let mut read_buffer = [0; 128];
    let mut corrected_read_buffer = [0; 128];
    match context.oids.get_mut(&oid).unwrap() {
        OIDTypes::TMCUart { _inner } => {
            _inner.send(&uart_bytes[..corrected_length]).unwrap();
            // Burn our own sent bytes from the buffer, we don't want those.
            _inner
                .read_exact(&mut read_buffer[..corrected_length])
                .unwrap();

            if read != 0 {
                if read != 10 {
                    unimplemented!(
                        "Haven't implemented any way to do reads of anything other than 10"
                    );
                }

                _inner.read_exact(&mut read_buffer[0..8 as usize]).unwrap();

                // log::trace!(
                //     "Read bytes : {read_bytes}, Data - {:X?}",
                //     &read_buffer[0..read_bytes]
                // );

                let corrected_byte_length = clad_uart_protocol_bits(
                    &mut corrected_read_buffer,
                    &read_buffer[0..8 as usize],
                );

                // log::trace!(
                //     "Fixed bytes : {:X?}",
                //     &corrected_read_buffer[0..corrected_byte_length]
                // );

                klipper_reply!(tmcuart_response, oid: u8 = oid, read: &[u8] = &corrected_read_buffer[0..corrected_byte_length as usize]);
            } else {
                klipper_reply!(tmcuart_response, oid: u8 = oid, read: &[u8] = &[])
            }
        }
        _ => panic!("Expected OID to be a TMCUart, but it wasn't!"),
    }
}
