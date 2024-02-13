#[derive(Clone)]
pub enum CLMonitorMessage {
    Calibrate,
    CheckPosition(u32, bool),
}
