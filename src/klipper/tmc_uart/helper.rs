use bitvec::prelude::*;

// Klippy sends us data with UART Start and Stop bits per 8 bits of data, so we need to strip those if we
// are going to use a hardware UART
pub fn strip_uart_protocol_bits(buf: &mut [u8], bytes: &[u8]) -> usize {
    // Go through each chunk of 10 bits, strip off bit 0 and bit 10, map back into a byte and write them into the buffer
    bytes
        .view_bits::<Lsb0>()
        .chunks_exact(10)
        .zip(buf)
        .map(|(chunk, output_buffer)| {
            *output_buffer = unsafe { chunk.get_unchecked(1..9).load::<u8>() }
        })
        .count()
}

// We need to do the reverse for data going back to Klippy. For each 8 bits of data we have, we need to set
// a start and end bit
pub fn clad_uart_protocol_bits(buf: &mut [u8], bytes: &[u8]) -> usize {
    // From dalegaard @ Annex
    buf.view_bits_mut::<Lsb0>()
        .chunks_exact_mut(10)
        .zip(bytes.iter())
        .map(|(out, inp)| {
            out.set(0, false);
            out.set(9, true);
            unsafe {
                out.get_unchecked_mut(1..9).store(*inp);
            }
            10
        })
        .sum::<usize>()
        .div_ceil(8)
}
