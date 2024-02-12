#[cfg(target_features = "esp32c6")]
pub mod esp32c6;

pub fn init() {
    #[cfg(target_features = "esp32c6")]
    {
        esp32c6::init();
    }
}
