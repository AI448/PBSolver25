use std::f64::INFINITY;

pub struct PLBDWatcher {
    pub short_term_average: ShortTermAverage,
    pub long_term_average: LongTermAverage,
}

impl PLBDWatcher {
    pub fn new(short_time_constant: usize, long_time_constant: usize) -> Self {
        Self {
            short_term_average: ShortTermAverage::new(short_time_constant),
            long_term_average: LongTermAverage::new(long_time_constant),
        }
    }

    pub fn add(&mut self, value: usize) {
        self.short_term_average.add(value);
        self.long_term_average.add(self.short_term_average.mean());
    }

    pub fn lower_tail_probability(&self) -> f64 {
        let value = self.short_term_average.mean();
        let mean = self.long_term_average.mean();
        let variance = self.long_term_average.variance();
        if variance > 0.0 {
            return 0.5 * (1.0 + ((value - mean) / (2.0 * variance).sqrt()).erf());
        } else {
            return INFINITY;
        }
    }
}

pub struct ShortTermAverage {
    time_constant: usize,
    values: Vec<usize>,
    count: usize,
    sum: usize,
}

impl ShortTermAverage {
    pub fn new(time_constant: usize) -> Self {
        return Self {
            time_constant,
            values: Vec::default(),
            count: 0,
            sum: 0,
        };
    }

    pub fn add(&mut self, value: usize) {
        if self.values.len() == self.time_constant {
            self.sum -= self.values[self.count % self.time_constant];
            self.sum += value;
            self.values[self.count % self.time_constant] = value;
            self.count += 1;
        } else {
            self.sum += value;
            self.values.push(value);
        }
    }

    pub fn mean(&self) -> f64 {
        return self.sum as f64 / self.values.len() as f64;
    }
}

pub struct LongTermAverage {
    time_constant: f64,
    count: f64,
    mean: f64,
    variance: f64,
}

impl LongTermAverage {
    pub fn new(time_constant: usize) -> Self {
        Self {
            time_constant: time_constant as f64,
            count: 0.0,
            mean: f64::NAN,
            variance: f64::NAN,
        }
    }

    pub fn add(&mut self, value: f64) {
        let value2 = (value - self.mean).powf(2.0);

        if self.count == 0.0 {
            self.mean = value;
            self.variance = value;
        } else if self.count < self.time_constant {
            self.mean = (self.mean * self.count + value) / (self.count + 1.0);
            self.variance = (self.variance * self.count + value2) / (self.count + 1.0);
        } else {
            self.mean =
                1.0 / self.time_constant * value + (1.0 - 1.0 / self.time_constant) * self.mean;
            self.variance = 1.0 / self.time_constant * value2
                + (1.0 - 1.0 / self.time_constant) * self.variance;
        }

        self.count += 1.0;
    }

    pub fn mean(&self) -> f64 {
        return self.mean;
    }

    pub fn variance(&self) -> f64 {
        return self.variance;
    }
}
