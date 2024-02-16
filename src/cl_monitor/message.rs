#[derive(Clone)]
pub enum CLMonitorMessage {
    Calibrate,
    CheckPosition(i32, u32, bool),
}
