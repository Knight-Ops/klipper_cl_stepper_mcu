use anchor::*;
use as5600_async::{status::Status, As5600};
use embassy_sync::{self, blocking_mutex::raw::CriticalSectionRawMutex};
use embassy_time::block_for;
use embassy_time::{Duration, Instant, Timer};
use esp32c6_hal::rmt::TxChannel;
use esp32c6_hal::{
    prelude::{_embedded_hal_digital_v2_OutputPin, _embedded_hal_digital_v2_StatefulOutputPin},
    rmt::PulseCode,
};

use crate::cl_monitor::{CLMonitorMessage, CL_MONITOR_CHANNEL};

use super::{StepperMessage, STEPPER_POSITION, STEPPER_STOP};

#[embassy_executor::task]
pub async fn step_driver(
    #[cfg(not(feature = "rmt_step"))] mut step: esp32c6_hal::gpio::GpioPin<
        esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>,
        5,
    >,
    #[cfg(feature = "rmt_step")] mut step: esp32c6_hal::rmt::Channel<0>,
    mut dir: esp32c6_hal::gpio::GpioPin<esp32c6_hal::gpio::Output<esp32c6_hal::gpio::PushPull>, 6>,
    invert_step: bool,
    step_pulse_ticks: u32,
    // step_queue: embassy_sync::channel::Receiver<
    //     'static,
    //     CriticalSectionRawMutex,
    //     StepperMessage,
    //     { crate::MOVE_QUEUE as usize },
    // >,
    step_queue: embassy_sync::priority_channel::Receiver<
        'static,
        CriticalSectionRawMutex,
        StepperMessage,
        embassy_sync::priority_channel::Max,
        { crate::MOVE_QUEUE as usize },
    >,
) {
    let mut step_counter = 0i32;
    let mut step_clock = Instant::from_ticks(0);

    #[cfg(not(feature = "rmt_step"))]
    let pulse_duration = Duration::from_ticks(step_pulse_ticks as u64);
    #[cfg(feature = "rmt_step")]
    let pulse = if !invert_step {
        PulseCode {
            level1: true,
            length1: 4,
            level2: false,
            length2: 0,
        }
    } else {
        PulseCode {
            level1: false,
            length1: 4,
            level2: true,
            length2: 0,
        }
    };

    loop {
        let step_info = step_queue.receive().await;
        // if let Some(step_info) = step_queue.receive().await {
        match step_info {
            StepperMessage::StepInfo { _inner: step_info } => {
                // Set our dir pin for these steps
                if step_info.dir() && !dir.is_set_high().unwrap() {
                    dir.set_high().unwrap();
                } else if !step_info.dir() && dir.is_set_high().unwrap() {
                    dir.set_low().unwrap();
                }

                // When we reset the step clock, this should just drain out everything that is left over
                // from previous runs, and should ignore all stray queue_step commands that don't schedule at least
                // later than "now"
                if step_clock == Instant::from_ticks(0)
                    && step_info.interval() < Instant::now().as_ticks() as u32
                {
                    continue;
                }

                let mut delay_between_pulses = Duration::from_ticks(step_info.interval() as u64);

                for _ in 0..step_info.count() {
                    // Not sure if this should go in the hot loop, this should be a pretty cheap check, but we could probably check between step groups
                    // The downside being they can be pretty large
                    if STEPPER_STOP.signaled() {
                        log::trace!("STEPPER_STOP has been signaled, drop everything");
                        // Reset the bat signal
                        STEPPER_STOP.reset();
                        // Clock reset is built into stepper stop
                        step_clock = Instant::from_ticks(0);
                        // Break out of our current step set
                        break;
                    } else {
                        let scheduled_time = step_clock.checked_add(delay_between_pulses).unwrap();

                        if Instant::now() > scheduled_time {
                            log::error!("Trying to schedule step in the past, it is currently {}, scheduled at {} | {} in the past", Instant::now().as_ticks(), scheduled_time.as_ticks(), Instant::now().duration_since(scheduled_time).as_ticks());
                            klipper_shutdown!(
                                "Stepper too far in past",
                                Instant::now().as_ticks() as u32
                            );
                        }

                        Timer::at(scheduled_time).await;

                        // Step Pulse
                        #[cfg(not(feature = "rmt_step"))]
                        if invert_step {
                            step.set_low().unwrap();
                            // Timer::after(pulse_duration).await;
                            block_for(pulse_duration);
                            step.set_high().unwrap();
                        } else {
                            step.set_high().unwrap();
                            // Timer::after(pulse_duration).await;
                            block_for(pulse_duration);
                            step.set_low().unwrap();
                        };
                        #[cfg(feature = "rmt_step")]
                        {
                            step = step.transmit(&[pulse]).wait().unwrap();
                        }

                        step_clock = Instant::now();

                        if step_info.add() != 0 {
                            if step_info.add().is_positive() {
                                delay_between_pulses = delay_between_pulses
                                    .checked_add(Duration::from_ticks(step_info.add() as u64))
                                    .unwrap()
                            } else {
                                delay_between_pulses = delay_between_pulses
                                    .checked_sub(Duration::from_ticks(step_info.add().abs() as u64))
                                    .unwrap()
                            }
                        }
                    }
                }

                if step_info.count() > 100 {
                    CL_MONITOR_CHANNEL.immediate_publisher().publish_immediate(
                        CLMonitorMessage::CheckPosition(step_info.interval(), step_info.dir()),
                    );
                }

                // Step counter
                if step_info.dir() {
                    step_counter = step_counter.wrapping_add(step_info.count() as i32);
                    STEPPER_POSITION.lock(|unlocked| {
                        *unlocked.borrow_mut() = step_counter;
                    });
                } else {
                    step_counter = step_counter.wrapping_sub(step_info.count() as i32);
                    STEPPER_POSITION.lock(|unlocked| {
                        *unlocked.borrow_mut() = step_counter;
                    });
                }
            }

            StepperMessage::StepCorrection { _inner: step_info } => {
                log::info!(
                    "Step Correction : {} | {} | {}",
                    step_info.interval(),
                    step_info.count(),
                    step_info.dir()
                );
                // Set our dir pin for these steps
                if step_info.dir() && !dir.is_set_high().unwrap() {
                    dir.set_high().unwrap();
                } else if !step_info.dir() && dir.is_set_high().unwrap() {
                    dir.set_low().unwrap();
                }

                for _ in 0..step_info.count() {
                    Timer::after_ticks(step_info.interval() as u64).await;

                    // Step Pulse
                    #[cfg(not(feature = "rmt_step"))]
                    if invert_step {
                        step.set_low().unwrap();
                        // Timer::after(pulse_duration).await;
                        block_for(pulse_duration);
                        step.set_high().unwrap();
                    } else {
                        step.set_high().unwrap();
                        // Timer::after(pulse_duration).await;
                        block_for(pulse_duration);
                        step.set_low().unwrap();
                    };
                    #[cfg(feature = "rmt_step")]
                    {
                        step = step.transmit(&[pulse]).wait().unwrap();
                    }
                }
            }

            StepperMessage::ResetStepClock => {
                step_clock = Instant::from_ticks(0);
            }
        }
    }
}
