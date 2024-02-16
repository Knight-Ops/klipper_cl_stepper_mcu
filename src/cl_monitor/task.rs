use as5600_async::{status::Status, As5600};
use embassy_time::Timer;
use esp32c6_hal::{i2c::I2C, peripherals::I2C0};

use crate::{
    klipper::stepper::{StepInfo, STEP_CORRECTION},
    STEPPER_POSITION,
};

use super::{message::CLMonitorMessage, CL_MONITOR_CHANNEL};

#[embassy_executor::task]
pub async fn closed_loop_monitor(mut driver: As5600<I2C<'static, I2C0>>) {
    loop {
        match driver.magnet_status().await {
            Ok(state) => match state {
                Status::MagnetDetected => {
                    log::info!("Magnet detected");
                    break;
                }
                _ => {
                    log::error!("Magnet not detected, or detected with error - {state:?}")
                }
            },
            Err(e) => {
                log::error!("Error with magnet detection occured : {:?}", e);
            }
        }
    }
    const DEG_PER_TICK: f32 = 360. / 4096.;
    // TODO: This needs to be calculated based on the microsteps (200*microsteps for 1.8 degree motors)
    const DEG_PER_STEP: f32 = 360. / 3200.;
    const ANGLE_PER_STEP: f32 = 3200. / 4096.;

    let mut angle_calibrate = 0;
    let mut step_calibrate = 0;

    let mut homes = 0;

    loop {
        match CL_MONITOR_CHANNEL.receive().await {
            CLMonitorMessage::Calibrate => {
                step_calibrate = STEPPER_POSITION.lock(|unlocked| *unlocked.borrow()).abs() % 3200;
                angle_calibrate = driver.raw_angle().await.unwrap();
                driver.set_zero_position(angle_calibrate).await.unwrap();
                Timer::after_millis(10).await;
                homes += 1;
            }
            CLMonitorMessage::CheckPosition(steps, interval, dir) => {
                if homes != 2 {
                    continue;
                }

                let angle = driver.angle().await.unwrap();

                if angle == 0 {
                    continue;
                }

                let exact_angle = angle as f32 * DEG_PER_TICK;
                let exact_motor = ((steps - step_calibrate) % 3200) as f32 * DEG_PER_STEP;

                let error = if exact_motor.is_sign_negative() {
                    let corrected_motor = 360. + exact_motor;
                    if (exact_angle > 345. && corrected_motor < 15.)
                        || (exact_angle < 15. && corrected_motor > 345.)
                    {
                        // These calculations are really problematic when we go from quad4 to quad1, so we will just
                        // skip them completely for the time being.
                        continue;
                    } else {
                        (exact_angle - corrected_motor) % 360.
                    }
                } else {
                    if (exact_angle > 345. && exact_motor < 15.)
                        || (exact_angle < 15. && exact_motor > 345.)
                    {
                        // These calculations are really problematic when we go from quad4 to quad1, so we will just
                        // skip them completely for the time being.
                        continue;
                    } else {
                        (exact_angle - exact_motor) % 360.
                    }
                };

                let step_correction_count = (error / DEG_PER_STEP) as i16;

                if step_correction_count.abs() > 8 {
                    if step_correction_count.is_negative() {
                        // log::info!("Currently behind where we should be by {error} degrees\nNeed to move in {} direction", dir);
                        STEP_CORRECTION.signal(StepInfo::new(
                            interval,
                            step_correction_count.abs() as u16,
                            0,
                            dir,
                        ));
                    } else if step_correction_count.is_positive() {
                        // log::info!("Currently ahead where we should be by {error} degrees\nNeed to move in {} direction", !dir);
                        STEP_CORRECTION.signal(StepInfo::new(
                            interval,
                            step_correction_count as u16,
                            0,
                            !dir,
                        ));
                    } else {
                        // log::info!("Currently at where we should be");
                    }
                }
            }
        }
    }
}
