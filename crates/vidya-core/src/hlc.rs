use serde::{Deserialize, Serialize};

use crate::error::DomainError;

/// A sortable hybrid logical clock value.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct Hlc {
    pub wall_ms: u64,
    pub counter: u32,
    pub device: String,
}

impl Hlc {
    /// Formats the HLC in its fixed-width database and wire representation.
    pub fn to_text(&self) -> String {
        format!("{:013}-{:06}-{}", self.wall_ms, self.counter, self.device)
    }

    /// Parses and validates an HLC wire value.
    pub fn parse(s: &str) -> Result<Self, DomainError> {
        let invalid = || DomainError::validation("hlc.error.invalid");
        let mut parts = s.splitn(3, '-');
        let wall = parts.next().ok_or_else(invalid)?;
        let counter = parts.next().ok_or_else(invalid)?;
        let device = parts.next().ok_or_else(invalid)?;
        if wall.len() != 13
            || counter.len() != 6
            || device.is_empty()
            || !wall.bytes().all(|b| b.is_ascii_digit())
            || !counter.bytes().all(|b| b.is_ascii_digit())
        {
            return Err(invalid());
        }
        Ok(Self {
            wall_ms: wall.parse().map_err(|_| invalid())?,
            counter: counter.parse().map_err(|_| invalid())?,
            device: device.to_owned(),
        })
    }
}

/// Stateful HLC generator. Physical time is always supplied by its caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HlcClock {
    last: Hlc,
}

impl HlcClock {
    /// Starts a device clock from a persisted value or zero.
    pub fn new(device: String, last: Option<Hlc>) -> Self {
        let last = match last {
            Some(mut persisted) => {
                persisted.device = device;
                persisted
            }
            None => Hlc {
                wall_ms: 0,
                counter: 0,
                device,
            },
        };
        Self { last }
    }

    /// Produces a strictly increasing local HLC for the supplied physical time.
    pub fn tick(&mut self, physical_ms: u64) -> Hlc {
        let next = if physical_ms > self.last.wall_ms {
            Hlc {
                wall_ms: physical_ms,
                counter: 0,
                device: self.last.device.clone(),
            }
        } else if self.last.counter == u32::MAX {
            Hlc {
                wall_ms: self.last.wall_ms.saturating_add(1),
                counter: 0,
                device: self.last.device.clone(),
            }
        } else {
            Hlc {
                wall_ms: self.last.wall_ms,
                counter: self.last.counter + 1,
                device: self.last.device.clone(),
            }
        };
        self.last = next.clone();
        next
    }

    /// Observes a remote HLC and returns a local value ordered after it.
    pub fn observe(&mut self, remote: &Hlc, physical_ms: u64) -> Hlc {
        let wall_ms = self.last.wall_ms.max(remote.wall_ms).max(physical_ms);
        let counter = match (wall_ms == self.last.wall_ms, wall_ms == remote.wall_ms) {
            (true, true) => self.last.counter.max(remote.counter).checked_add(1),
            (true, false) => self.last.counter.checked_add(1),
            (false, true) => remote.counter.checked_add(1),
            (false, false) => Some(0),
        };
        let (wall_ms, counter) = match counter {
            Some(counter) => (wall_ms, counter),
            None => (wall_ms.saturating_add(1), 0),
        };
        let next = Hlc {
            wall_ms,
            counter,
            device: self.last.device.clone(),
        };
        self.last = next.clone();
        next
    }
}
