use embassy_time::Instant;

pub enum TRSyncMessage {
    SetTimeout { to: Instant },
    NewTrigger { reason: u8, trigger_time: u32 },
    HostRequest,
    EndTask,
}
