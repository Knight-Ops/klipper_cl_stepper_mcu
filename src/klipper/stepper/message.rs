use super::StepInfo;

pub enum StepperMessage {
    StepInfo { _inner: StepInfo },
    ResetStepClock,
}
