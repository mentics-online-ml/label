use std::{cmp::min, collections::VecDeque};

use shared_types::*;
use series_store::*;

use crate::checks::Check;

pub(crate) struct HandleEvents<C> {
    pub(crate) start_ts: Timestamp,
    pub(crate) base: QuoteEvent,
    pub(crate) checks: Vec<C>,
    pub(crate) complete: Vec<C>,
    pub(crate) events: VecDeque<QuoteEvent>
}

impl<C: Check> HandleEvents<C> {
    pub(crate) fn new(checks: Vec<C>, base: QuoteEvent) -> Self {
        let start_ts = min(base.biddate, base.askdate);
        // Self { i: 0, base, start_ts, events: VecDeque::new(), checks, complete: Vec::new() }
        Self { checks, base, start_ts, events: VecDeque::new(), complete: Vec::new() }
    }

    pub(crate) fn init_next(&mut self) {
        assert!(self.checks.is_empty());

        // TODO: replace unwraps?
        // base is right before the back event
        // assert!(self.events.back().unwrap().event_id == self.base.event_id);
        self.base = self.events.pop_back().unwrap();
        self.start_ts = min(self.base.biddate, self.base.askdate);

        self.checks.append(&mut self.complete);
        assert!(self.complete.is_empty());
        for c in self.complete.iter_mut() {
            c.reset();
        }

        for event in self.events.iter() {
            Self::proc_event(&mut self.checks, &mut self.complete, &self.base, event);
        }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.checks.is_empty()
    }

    pub(crate) fn proc_event(checks: &mut Vec<C>, complete: &mut Vec<C>, base: &QuoteEvent, quote: &QuoteEvent) -> bool {
        // Quotes coming in here have already been validated
        let bid_change = rounded_diff(quote.bid, base.ask, 2);
        let ask_change = rounded_diff(quote.ask, base.bid, 2);

        complete.extend(checks.extract_if(|check| {
            let should_continue = check.track(bid_change, ask_change);
            !should_continue
        }));

        !checks.is_empty()
    }
}

impl<C: Check> EventHandler<QuoteEvent> for HandleEvents<C> {
    fn handle(&mut self, quote: QuoteEvent) -> bool {
        // TODO: check within valid trading time
        // If more than a day has passed, too long.
        if quote.biddate - self.start_ts > 8*60*60*1000 {
            return false;
        }

        let should_continue = Self::proc_event(&mut self.checks, &mut self.complete, &self.base, &quote);
        self.events.push_front(quote);
        should_continue
    }
}

fn rounded_diff(a: f32, b: f32, digits: u8) -> f32 {
    let scale = 10f32.powi(digits as i32);
    ((a - b) * scale).round() / scale
}