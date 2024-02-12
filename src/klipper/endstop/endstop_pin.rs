use embedded_hal_async::digital::Wait;

// This is a hack because these aren't the same types and we can't use generics with embassy tasks
pub enum EndstopPin {
    Floating(esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::Floating>, 7>),
    PullUp(esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Input<esp32c6_hal::gpio::PullUp>, 7>),
}

impl EndstopPin {
    pub async fn wait_for_high(&mut self) {
        match self {
            EndstopPin::Floating(p) => {
                p.wait_for_high().await.unwrap();
            }
            EndstopPin::PullUp(p) => {
                p.wait_for_high().await.unwrap();
            }
        }
    }

    pub async fn wait_for_low(&mut self) {
        match self {
            EndstopPin::Floating(p) => {
                p.wait_for_low().await.unwrap();
            }
            EndstopPin::PullUp(p) => {
                p.wait_for_low().await.unwrap();
            }
        }
    }
}
