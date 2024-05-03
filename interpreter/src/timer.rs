use std::time::{Duration, Instant};

pub struct Timer {
    val: u8,
    wait: Duration,
    acc: Duration,
    last: Instant,
}

pub struct Freq {
    ticks: u32,
    duration: Duration,
}

impl Default for Freq {
    fn default() -> Self {
        Self {
            ticks: 1,
            duration: Duration::from_secs(1),
        }
    }
}

impl Timer {
    pub fn zero() -> Self {
        Timer::new(0)
    }

    pub fn new(val: u8) -> Self {
        Timer::with_freq(val, Default::default())
    }

    pub fn with_freq(val: u8, freq: Freq) -> Self {
        Self {
            val,
            wait: freq.duration / freq.ticks,
            acc: Duration::ZERO,
            last: Instant::now(),
        }
    }

    pub fn update(&mut self) {
        self.acc += Instant::now().duration_since(self.last);
        while self.val > 0 && self.acc >= self.wait {
            self.val = self.val.saturating_sub(1);
            self.acc -= self.wait;
        }
    }

    pub fn val(&self) -> u8 {
        self.val
    }
}
