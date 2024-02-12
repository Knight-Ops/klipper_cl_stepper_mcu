use super::StepInfo;

pub enum StepperMessage {
    StepInfo { _inner: StepInfo },
    ResetStepClock,
}

impl StepperMessage {
    fn get_priority(&self) -> u32 {
        match self {
            Self::ResetStepClock => 0,
            Self::StepInfo { _inner } => 0,
        }
    }
}

impl core::cmp::PartialEq for StepperMessage {
    fn eq(&self, other: &Self) -> bool {
        if self.get_priority() == other.get_priority() {
            true
        } else {
            false
        }
    }
}

impl core::cmp::Eq for StepperMessage {}

impl core::cmp::PartialOrd for StepperMessage {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.get_priority().cmp(&other.get_priority()))
    }
}

impl core::cmp::Ord for StepperMessage {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.get_priority().cmp(&other.get_priority())
    }
}
