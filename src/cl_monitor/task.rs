use as5600_async::{status::Status, As5600};
use esp32c6_hal::{i2c::I2C, peripherals::I2C0};

use crate::{
    klipper::stepper::{StepInfo, STEPPER_MOVE_QUEUE},
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

    let mut angle_calibrate = 0;
    let mut step_calibrate = 0;
    let mut subscription = CL_MONITOR_CHANNEL.subscriber().unwrap();

    loop {
        match subscription.next_message().await {
            embassy_sync::pubsub::WaitResult::Lagged(count) => {
                log::debug!("Dropped {count} messages");
            }
            embassy_sync::pubsub::WaitResult::Message(msg) => match msg {
                CLMonitorMessage::Calibrate => {
                    angle_calibrate = driver.angle().await.unwrap();
                    step_calibrate =
                        STEPPER_POSITION.lock(|unlocked| *unlocked.borrow()).abs() % 3200;
                }
                CLMonitorMessage::CheckPosition(interval, dir) => {
                    if angle_calibrate == 0 && step_calibrate == 0 {
                        continue;
                    }

                    let angle = driver.angle().await.unwrap();
                    let pos = STEPPER_POSITION.lock(|unlocked| *unlocked.borrow()) % 3200;
                    // log::info!(
                    //     "Magnet sensor reading : {} | pos : {}",
                    //     (angle.abs_diff(angle_calibrate)) as f32 * DEG_PER_TICK,
                    //     (pos - step_calibrate) as f32 * DEG_PER_STEP,
                    // );

                    let error = ((angle.abs_diff(angle_calibrate) as f32) * DEG_PER_TICK)
                        - ((pos - step_calibrate).abs() as f32 * DEG_PER_STEP) % 360.;

                    let count = (error / DEG_PER_STEP) as i16;
                    if count < 0 {
                        log::info!("Currently behind where we should be by {error} degrees");
                        STEPPER_MOVE_QUEUE
                            .send(crate::klipper::stepper::StepperMessage::StepCorrection {
                                _inner: StepInfo::new(interval, count.abs() as u16, 0, dir),
                            })
                            .await;
                    } else if count > 0 {
                        log::info!("Currently ahead where we should be by {error} degrees");
                        STEPPER_MOVE_QUEUE
                            .send(crate::klipper::stepper::StepperMessage::StepCorrection {
                                _inner: StepInfo::new(interval, count as u16, 0, !dir),
                            })
                            .await;
                    } else {
                        log::info!("Currently at where we should be");
                    }
                }
            },
        }
    }
}
