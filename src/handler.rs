use std::{cmp::min, collections::VecDeque};

use chrono::NaiveDate;
use shared_types::{util::{to_date, valid_time_and_date}, *};
use series_store::*;

use crate::checks::Check;

pub(crate) struct HandleEvents<C> {
    pub(crate) checks: Vec<C>,
    // This is the event being labeled
    pub(crate) base: QuoteEvent,
    pub(crate) base_date: NaiveDate,
    pub(crate) events: VecDeque<QuoteEvent>,
    pub(crate) complete: Vec<C>,
}

impl<C: Check> HandleEvents<C> {
    pub(crate) fn new(checks: Vec<C>, base: QuoteEvent) -> Self {
        let base_date = to_date(&base);
        Self { checks, base, base_date, events: VecDeque::new(), complete: Vec::new() }
    }

    pub(crate) fn init_next(&mut self) {
        assert!(self.checks.is_empty());

        // TODO: replace unwrap?
        let new_base = self.events.pop_front().unwrap();
        self.use_base(new_base);
        // println!("init_next base: {:?}", self.base);

        self.checks.append(&mut self.complete);
        assert!(self.complete.is_empty());
        for c in self.complete.iter_mut() {
            c.reset();
        }

        for event in self.events.iter() {
            Self::proc_event(&mut self.checks, &mut self.complete, &self.base, event);
        }
    }

    fn use_base(&mut self, new_base: QuoteEvent) {
        self.base = new_base;
        self.base_date = to_date(&self.base);
    }

    pub(crate) fn is_done(&self) -> bool {
        self.checks.is_empty()
    }

    pub(crate) fn proc_event(checks: &mut Vec<C>, complete: &mut Vec<C>, base: &QuoteEvent, quote: &QuoteEvent) -> bool {
        // Quotes coming in here have already been validated
        let bid_change = rounded_diff(quote.bid, base.ask, 2);
        let ask_change = rounded_diff(quote.ask, base.bid, 2);
        // println!("bid_change: {}, ask_change: {}", bid_change, ask_change);

        complete.extend(checks.extract_if(|check| {
            let should_continue = check.track(bid_change, ask_change);
            // if !should_continue {
            //     println!("{}: Check complete on event: {:?}", check.ordinal(), quote);
            // }
            !should_continue
        }));

        !checks.is_empty()
    }
}

impl<C: Check> EventHandler<QuoteEvent> for HandleEvents<C> {
    fn handle(&mut self, quote: QuoteEvent) -> bool {
        if !valid_time_and_date(&quote, self.base_date) {
            return false;
        }

        let should_continue = Self::proc_event(&mut self.checks, &mut self.complete, &self.base, &quote);
        self.events.push_back(quote);
        should_continue
    }
}

fn rounded_diff(a: f32, b: f32, digits: u8) -> f32 {
    let scale = 10f32.powi(digits as i32);
    ((a - b) * scale).round() / scale
}