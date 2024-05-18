#![feature(extract_if)]

use std::{cmp::min, collections::VecDeque, time::Duration};

use shared_types::*;
use series_store::*;
use kv_store::*;

const CURRENT_VERSION: u32 = 1;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = StdoutLogger::boxed();
    let mut reader = SeriesReader::new(logger)?;

    let topic = Topic::new("raw", "SPY", "quote");
    reader.subscribe(&topic)?;
    reader.seek(&topic, Offset::Beginning)?;
    let msgs = reader.read_count(100)?;
    // TODO: validate the series is continuous and inside trading time
    let base_msg = msgs.last().expect("No messages from read_count");

    let base_quote: Quote = msg_to(base_msg)?;
    println!("Base quote: {:?}", base_quote);

    let check_down_1 = CheckDown::new(-0.02, 0.01);
    let check_up_1 = CheckUp::new(0.02, -0.01);

    let checks: Vec<Box<dyn Check>> = vec!(Box::new(check_down_1), Box::new(check_up_1));

    let mut handler: HandleEvents<Box<dyn Check>> = HandleEvents::new(base_quote, checks);
    reader.for_each_msg(&mut handler).await;

    Ok(())
}

impl Check for std::boxed::Box<dyn Check> {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        self.as_mut().track(bid_change, ask_change)
    }
}

pub trait Check {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool;
}

// #[derive(Default)]
pub struct CheckUp {
    pub threshold_up: f32,
    pub threshold_down: f32,
    pub result: Option<bool>
}

impl CheckUp {
    pub fn new(threshold_up: f32, threshold_down: f32) -> Self {
        Self { threshold_up, threshold_down, result: None }
    }
}

impl Check for CheckUp {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        if bid_change >= self.threshold_up {
            self.result = Some(true);
            println!("CheckUp surpassed threshold_up {} > {}", bid_change, self.threshold_up);
            false
        } else if ask_change <= self.threshold_down {
            self.result = Some(false);
            println!("CheckUp surpassed threshold_down {} < {}", ask_change, self.threshold_down);
            false
        } else {
            true
        }
    }
}

// #[derive(Default)]
pub struct CheckDown {
    pub threshold_down: f32,
    pub threshold_up: f32,
    pub result: Option<bool>
}

impl CheckDown {
    pub fn new(threshold_down: f32, threshold_up: f32) -> Self {
        Self { threshold_down, threshold_up, result: None }
    }
}

impl Check for CheckDown {
    fn track(&mut self, bid_change: f32, ask_change: f32) -> bool {
        if ask_change <= self.threshold_down {
            self.result = Some(true);
            println!("CheckDown surpassed threshold_down {} < {}", ask_change, self.threshold_down);
            false
        } else if bid_change >= self.threshold_up {
            self.result = Some(false);
            println!("CheckDown surpassed threshold_up {} > {}", bid_change, self.threshold_up);
            false
        } else {
            true
        }
    }
}

struct HandleEvents<C> {
    i: u32,
    base: Quote,
    start_ts: Timestamp,
    events: VecDeque<Quote>,
    checks: Vec<C>,
    complete: Vec<C>,
}

impl<C> HandleEvents<C> {
    fn new(base: Quote, checks: Vec<C>) -> Self {
        let start_ts = min(base.biddate, base.askdate);
        Self { i: 0, base, start_ts, events: VecDeque::new(), checks, complete: Vec::new() }
    }
}

// From<Box<dyn Check>>
impl<C: Check> EventHandler<Quote> for HandleEvents<C> {
    fn handle(&mut self, id: EventId, quote: Quote) -> bool {
        // If more than a day has passed, too long.
        if quote.biddate - self.start_ts > 8*60*60*1000 {
            return false;
        }
        let bid_change = quote.bid - self.base.bid;
        let ask_change = quote.ask - self.base.ask;
        println!("Change: {}, {}", bid_change, ask_change);

        self.complete.extend(self.checks.extract_if(|check| {
            // let x: &dyn Check = check.into();
            let should_continue = check.track(bid_change, ask_change);
            println!("Result for check: {}", should_continue);
            !should_continue
        }));

        // self.checks.retain_mut(|check| {
        //     let remove = check.track(bid_change, ask_change);
        //     self.complete.push(std::mem::take(check));
        //     remove
        // });

        // TODO: check market timing
        self.i += 1;
        let res = self.i < 1000 && !self.checks.is_empty();
        println!("Result for handle: {}", res);
        res
    }
}
