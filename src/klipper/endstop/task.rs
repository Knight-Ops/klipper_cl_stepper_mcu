use embassy_futures::select::Either;

use embassy_time::Instant;

use crate::cl_monitor::CL_MONITOR_CHANNEL;
use crate::klipper::trsync::{TRSyncMessage, TRSYNC_CHANNEL};

use super::EndstopPin;
use super::ENDSTOP_CHANNEL;

#[embassy_executor::task]
pub async fn endstop_runner(
    _clock: u32,
    _sample_ticks: u32,
    _sample_count: u8,
    _rest_ticks: u32,
    pin_value: u8,
    _trsync_oid: u8,
    trigger_reason: u8,
    mut pin: EndstopPin,
) {
    let mut triggered = false;
    loop {
        match triggered {
            false => {
                if pin_value == 1 {
                    match embassy_futures::select::select(
                        pin.wait_for_high(),
                        ENDSTOP_CHANNEL.receive(),
                    )
                    .await
                    {
                        Either::First(_) => {
                            // Shoot up the flare
                            TRSYNC_CHANNEL
                                .send(TRSyncMessage::NewTrigger {
                                    reason: trigger_reason,
                                    trigger_time: Instant::now().as_ticks() as u32,
                                })
                                .await;
                            triggered = true;
                        }
                        Either::Second(_) => {
                            // The only message we can get is that we should die since the Trsync timed out
                            // We should probably match here to be better
                            return;
                        }
                    }
                } else {
                    match embassy_futures::select::select(
                        pin.wait_for_low(),
                        ENDSTOP_CHANNEL.receive(),
                    )
                    .await
                    {
                        Either::First(_) => {
                            // Shoot up the flare
                            TRSYNC_CHANNEL
                                .send(TRSyncMessage::NewTrigger {
                                    reason: trigger_reason,
                                    trigger_time: Instant::now().as_ticks() as u32,
                                })
                                .await;
                            triggered = true;
                        }
                        Either::Second(_) => {
                            // The only message we can get is that we should die since the Trsync timed out
                            // We should probably match here to be better
                            return;
                        }
                    }
                }
            }

            true => {
                // The only message we can get is that we should die since the Trsync timed out
                // We should probably match here to be better
                let _ = ENDSTOP_CHANNEL.receive().await;
                CL_MONITOR_CHANNEL
                    .send(crate::cl_monitor::CLMonitorMessage::Calibrate)
                    .await;

                return;
            }
        }
    }
}
