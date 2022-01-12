use std::{collections::VecDeque, time::Instant};

#[derive(Debug)]
pub struct TimeTracker {
    start: Instant,
    /// Average delta time of frame
    avg_dt: f64,
    /// Average fps of frame over 1 second
    avg_fps: f64,
    /// Frame counter
    frame_counter: usize,
    /// Delta time of last frame
    dt: f64,
    /// Delta time of last fixed frame (e.g. 60fps)
    dt_fixed: f64,
    /// Delta time sum to calculate averages
    dt_sum: f64,
    /// Delta time sum to handle fixed frame times
    dt_sum_fixed: f64,
    prev_time: Instant,
}

impl TimeTracker {
    pub fn new() -> TimeTracker {
        TimeTracker {
            start: Instant::now(),
            avg_dt: 0.0,
            avg_fps: 0.0,
            frame_counter: 0,
            dt: 0.0,
            dt_fixed: 0.0,
            dt_sum: 0.0,
            dt_sum_fixed: 0.0,
            prev_time: Instant::now(),
        }
    }

    /// Deltatime in milliseconds
    pub fn dt(&self) -> f64 {
        self.dt
    }

    pub fn dt_sum_fixed(&self) -> f64 {
        self.dt_sum_fixed
    }

    /// Average fps over last second
    pub fn avg_fps(&self) -> f64 {
        self.avg_fps
    }

    pub fn fps(&self) -> f64 {
        1000.0 / self.dt
    }

    pub fn fixed_fps(&self) -> f64 {
        1000.0 / self.dt_fixed
    }

    pub fn time_secs(&self) -> f64 {
        self.start.elapsed().as_secs_f64()
    }

    /// Reset delta time sum for fixed after update, set dt_fixed
    pub fn reset_fixed(&mut self) {
        self.dt_fixed = self.dt_sum_fixed;
        self.dt_sum_fixed = 0.0;
    }

    /// Update time every frame
    pub fn update(&mut self) {
        let now = Instant::now();
        self.dt = now.duration_since(self.prev_time).as_nanos() as f64 / 1_000_000.0;
        self.dt_sum += self.dt;
        if self.dt_sum >= 1000. {
            self.avg_dt = self.dt_sum / self.frame_counter as f64;
            self.avg_fps = 1000. / self.avg_dt;
            self.dt_sum = 0.0;
            self.frame_counter = 0;
        }
        self.prev_time = now;
        self.frame_counter += 1;
        self.dt_sum_fixed += self.dt;
    }
}

impl Default for TimeTracker {
    fn default() -> Self {
        TimeTracker::new()
    }
}

const NUM_TIME_SAMPLES: usize = 150;

#[allow(unused)]
pub struct PerformanceTimer {
    time: Instant,
    data: VecDeque<f64>,
}

impl PerformanceTimer {
    pub fn new() -> Self {
        Self {
            time: Instant::now(),
            data: VecDeque::new(),
        }
    }

    pub fn start(&mut self) {
        self.time = Instant::now()
    }

    #[allow(unused)]
    pub fn end(&self) -> f64 {
        Instant::now().duration_since(self.time).as_nanos() as f64 / 1_000_000.0
    }

    pub fn time_it(&mut self) {
        let time = Instant::now().duration_since(self.time).as_nanos() as f64 / 1_000_000.0;
        self.data.push_back(time);
        if self.data.len() >= NUM_TIME_SAMPLES {
            self.data.pop_front();
        }
    }

    pub fn push_dt_ms(&mut self, dt: f64) {
        self.data.push_back(dt);
        if self.data.len() >= NUM_TIME_SAMPLES {
            self.data.pop_front();
        }
    }

    pub fn time_average_ms(&self) -> f64 {
        self.data.iter().sum::<f64>() / self.data.len() as f64
    }
}

impl Default for PerformanceTimer {
    fn default() -> Self {
        PerformanceTimer::new()
    }
}
