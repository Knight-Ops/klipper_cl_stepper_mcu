use embassy_futures::select::{
    select,
    Either::{First, Second},
};

use embassy_time::{Duration, Instant, Timer};

use crate::klipper::stepper::STEPPER_STOP;

use super::{trsync_report, TRSyncMessage, TRSYNC_CHANNEL};

#[embassy_executor::task]
pub async fn trsync_runner(oid: u8, report_clock: u32, report_ticks: u32, expire_reason: u8) {
    let mut timeout = None;
    let mut triggerable = true;
    let mut next_report = Instant::from_ticks(report_clock as u64)
        .checked_add(Duration::from_ticks(report_ticks as u64))
        .unwrap();
    let mut expire_reason = expire_reason;
    let mut end = false;

    loop {
        match select(Timer::at(next_report), TRSYNC_CHANNEL.receive()).await {
            First(_) => {
                // If we are past our timeout, just assume we have sent our last required message and kill the task
                if let Some(to) = timeout {
                    if Instant::now() > to {
                        log::error!(
                            "trsync_runner timed out at {}, untriggering...",
                            Instant::now().as_ticks()
                        );
                        triggerable = false;
                        timeout = None;
                    }
                }

                // Send our report status.
                trsync_report(
                    oid,
                    triggerable as u8,
                    expire_reason,
                    Instant::now().as_ticks() as u32,
                );
                next_report = Instant::now()
                    .checked_add(Duration::from_ticks(report_ticks as u64))
                    .unwrap();

                if end {
                    return;
                }
            }
            Second(pkt) => match pkt {
                TRSyncMessage::SetTimeout { to } => {
                    timeout = Some(to);
                }
                TRSyncMessage::NewTrigger {
                    reason,
                    trigger_time,
                } => {
                    log::trace!("TRSync : New Trigger {} {}", reason, trigger_time);
                    expire_reason = reason;
                    STEPPER_STOP.signal(true);
                    triggerable = false;
                }
                TRSyncMessage::HostRequest => {
                    end = true;
                }
                TRSyncMessage::EndTask => {
                    log::trace!("Trsync_runner ended upon request");
                    end = true;
                    expire_reason = 4;
                    triggerable = false;
                }
            },
        }
    }
}
