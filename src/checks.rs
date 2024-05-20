// TODO: Create CheckComplete trait that has the result in it so it doesn't need to be option

pub trait Check {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool;
    fn reset(&mut self);
    fn ordinal(&self) -> u8;
    fn result(&self) -> bool;
}

impl Check for std::boxed::Box<dyn Check> {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        self.as_mut().track(bid_change, ask_change)
    }

    fn reset(&mut self) {
        self.as_mut().reset();
    }

    fn ordinal(&self) -> u8 {
        self.as_ref().ordinal()
    }

    fn result(&self) -> bool {
        self.as_ref().result()
    }
}

pub struct CheckUp {
    pub ordinal: u8,
    pub threshold_up: f32,
    pub threshold_down: f32,
    pub result: Option<bool>
}

impl CheckUp {
    pub fn new(ordinal: u8, threshold_up: f32, threshold_down: f32) -> Self {
        Self { ordinal, threshold_up, threshold_down, result: None }
    }
}

impl Check for CheckUp {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        if bid_change >= self.threshold_up {
            self.result = Some(true);
            // println!("{}: CheckUp surpassed threshold_up {} > {}", self.ordinal, bid_change, self.threshold_up);
            false
        } else if ask_change <= self.threshold_down {
            self.result = Some(false);
            // println!("{}: CheckUp surpassed threshold_down {} < {}", self.ordinal, ask_change, self.threshold_down);
            false
        } else {
            true
        }
    }

    fn reset(&mut self) {
        self.result = None;
    }

    fn ordinal(&self) -> u8 { self.ordinal }
    fn result(&self) -> bool { self.result.unwrap() }
}

// #[derive(Default)]
pub struct CheckDown {
    pub ordinal: u8,
    pub threshold_down: f32,
    pub threshold_up: f32,
    pub result: Option<bool>
}

impl CheckDown {
    pub fn new(ordinal: u8, threshold_down: f32, threshold_up: f32) -> Self {
        Self { ordinal, threshold_down, threshold_up, result: None }
    }
}

impl Check for CheckDown {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        if ask_change <= self.threshold_down {
            self.result = Some(true);
            // println!("{}: CheckDown surpassed threshold_down {} < {}: ({}, {})", self.ordinal, ask_change, self.threshold_down, bid_change, ask_change);
            false
        } else if bid_change >= self.threshold_up {
            self.result = Some(false);
            // println!("{}: CheckDown surpassed threshold_up {} > {}: ({}, {})", self.ordinal, bid_change, self.threshold_up, bid_change, ask_change);
            false
        } else {
            true
        }
    }

    fn reset(&mut self) {
        self.result = None;
    }

    fn ordinal(&self) -> u8 { self.ordinal }
    fn result(&self) -> bool { self.result.unwrap() }
}
