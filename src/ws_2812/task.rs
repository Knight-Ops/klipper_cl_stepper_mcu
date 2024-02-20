use super::driver::SmartLedsAdapter;
use super::effects::{LedEffect, LedEffects, RainbowEffectStateMachine};
use smart_leds::{SmartLedsWrite, RGB8};

#[embassy_executor::task]
pub async fn onboard_rgb_led(mut rgb_driver: SmartLedsAdapter<esp32c6_hal::rmt::Channel<0>, 25>) {
    rgb_driver
        .write([RGB8::from((0, 0, 0)); 1].iter().cloned())
        .ok();

    let mut led_effect_driver = LedEffect::new(LedEffects::Rainbow {
        state: RainbowEffectStateMachine::IncRed,
        max_brightness: 16,
        delay_ms: 100,
    });

    let mut rgb = RGB8::from((0, 0, 0));
    loop {
        rgb_driver.write([rgb; 1].iter().cloned()).ok();

        led_effect_driver.next(&mut rgb).await;
    }
}
