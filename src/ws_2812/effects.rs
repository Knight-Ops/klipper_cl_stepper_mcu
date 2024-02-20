use embassy_time::{Duration, Timer};
use smart_leds::RGB8;

pub struct LedEffect {
    effect: LedEffects,
}

impl LedEffect {
    pub fn new(effect: LedEffects) -> Self {
        LedEffect { effect }
    }

    pub async fn next(&mut self, rgb: &mut RGB8) {
        match self.effect {
            _ => self.effect.next(rgb).await,
        }
    }
}

#[derive(Default)]
pub enum LedEffects {
    #[default]
    None,
    Rainbow {
        state: RainbowEffectStateMachine,
        max_brightness: u8,
        delay_ms: u64,
    },
}

impl LedEffects {
    async fn next(&mut self, rgb: &mut RGB8) {
        match self {
            Self::None => {}
            Self::Rainbow {
                state,
                max_brightness,
                delay_ms,
            } => {
                match state {
                    RainbowEffectStateMachine::IncRed => {
                        if rgb.r == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.r += 1;
                        }
                    }
                    RainbowEffectStateMachine::IncGreen => {
                        if rgb.g == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.g += 1;
                        }
                    }
                    RainbowEffectStateMachine::IncBlue => {
                        if rgb.b == *max_brightness {
                            *state = state.next_state();
                        } else {
                            rgb.b += 1;
                        }
                    }
                    RainbowEffectStateMachine::DecRed => {
                        if rgb.r == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.r -= 1;
                        }
                    }
                    RainbowEffectStateMachine::DecGreen => {
                        if rgb.g == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.g -= 1;
                        }
                    }
                    RainbowEffectStateMachine::DecBlue => {
                        if rgb.b == 0 {
                            *state = state.next_state();
                        } else {
                            rgb.b -= 1;
                        }
                    }
                };
                Timer::after(Duration::from_millis(*delay_ms)).await;
            }
        }
    }
}

#[derive(Default)]
pub enum RainbowEffectStateMachine {
    #[default]
    IncRed,
    IncGreen,
    IncBlue,
    DecRed,
    DecGreen,
    DecBlue,
}

impl RainbowEffectStateMachine {
    fn next_state(&self) -> Self {
        match self {
            RainbowEffectStateMachine::IncRed => RainbowEffectStateMachine::DecGreen,
            RainbowEffectStateMachine::DecGreen => RainbowEffectStateMachine::IncBlue,
            RainbowEffectStateMachine::IncBlue => RainbowEffectStateMachine::DecRed,
            RainbowEffectStateMachine::DecRed => RainbowEffectStateMachine::IncGreen,
            RainbowEffectStateMachine::IncGreen => RainbowEffectStateMachine::DecBlue,
            RainbowEffectStateMachine::DecBlue => RainbowEffectStateMachine::IncRed,
        }
    }
}
