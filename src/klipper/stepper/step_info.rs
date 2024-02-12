pub struct StepInfo {
    interval: u32,
    count: u16,
    add: i16,
    dir: bool,
}

impl StepInfo {
    pub fn new(interval: u32, count: u16, add: i16, dir: bool) -> Self {
        Self {
            interval,
            count,
            add,
            dir,
        }
    }

    pub fn interval(&self) -> u32 {
        self.interval
    }

    pub fn count(&self) -> u16 {
        self.count
    }

    pub fn add(&self) -> i16 {
        self.add
    }

    pub fn dir(&self) -> bool {
        self.dir
    }
}
