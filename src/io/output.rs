use crate::config::IoPin;
use anyhow::Result;
use sysfs_gpio::{Direction, Pin};

pub(crate) struct Output {
    pin: Pin,
    inverted: bool,
}

impl Output {
    pub(crate) fn new(config: &IoPin) -> Result<Self> {
        let pin = Pin::new(config.number);
        pin.export()?;
        pin.set_direction(Direction::Out)?;
        Ok(Output {
            pin,
            inverted: config.inverted,
        })
    }

    pub(crate) fn set(&self, on: bool) -> Result<()> {
        log::debug!("Setting pin {} on={}", self.pin.get_pin(), on);
        Ok(self.pin.set_value(match on ^ self.inverted {
            true => 1,
            false => 0,
        })?)
    }
}
