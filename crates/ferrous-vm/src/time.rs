use core::time::Duration;

/// Simulated time instant (monotonic)
#[derive(Debug, Copy, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct SimulatedInstant {
    ticks: u64,
}

impl SimulatedInstant {
    // pub fn elapsed_since(&self, earlier: SimulatedInstant) -> Duration { ... }
    // pub fn duration_since(&self, earlier: SimulatedInstant) -> Duration { ... }
}

