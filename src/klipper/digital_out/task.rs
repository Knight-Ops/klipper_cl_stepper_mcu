use embassy_time::{Instant, Timer};
use embedded_hal::digital::OutputPin;

#[embassy_executor::task]
pub async fn execute_digital_out(
    mut pin: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>, 4>,
    start_clock: u32,
    on_ticks: u32,
) {
    Timer::at(Instant::from_ticks(start_clock as u64)).await;

    // We don't support the digital PWM currently so just use the `on_ticks` as the high/low
    if on_ticks == 0 {
        pin.set_low().unwrap()
    } else {
        pin.set_high().unwrap()
    }
}
