use std::collections::VecDeque;

use quote::{QuoteEvent, QuoteValues};
use series_proc::Processor;
use shared_types::*;
use crate::checks::Check;

#[derive(Debug)]
pub struct LabelIds {
    pub event_id: EventId,
    pub offset_from: OffsetId,
    pub offset_to: OffsetId,
}

pub(crate) struct HandleEvents<C> {
    pub(crate) checks: Vec<C>,
    pub(crate) complete: Vec<C>,
}

impl<C: Check> HandleEvents<C> {
    pub(crate) fn new(checks: Vec<C>) -> Self {
        Self { checks, complete: Vec::new() }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.checks.is_empty()
    }

    pub(crate) fn ids(&self, events: &VecDeque<QuoteEvent>) -> LabelIds {
        match (events.front(), events.back()) {
            (Some(front), Some(back)) => LabelIds {
                event_id: front.event_id, offset_from: front.offset, offset_to: back.offset
            },
            // TODO: use types to make this impossible
            _ => panic!("handler.offsets called when no data")
        }
    }

    pub(crate) fn move_to_next(&mut self, start_values: &QuoteValues, events: &mut VecDeque<QuoteEvent>) -> bool {
        assert!(self.checks.is_empty());
        self.reset_checks();
        // Move to the next start event
        // handler.move_to_next();
        // let new_start_values = &handler.start_values;

        // TODO: some way to do this idiom without having to clone or put code in place?
        // self.start_with(new_first);
        // let start_values = handler.start_values;
        //  QuoteValues::convert_from(new_first);

        let HandleEvents { ref mut checks, ref mut complete } = self;
        for event in events.iter() {
            // Must call proc and not handler so the events are not pushed onto queue again
            Self::proc(start_values, event, checks, complete);
        }
        // assert!(handler.events.front().unwrap().bid == new_start_values.bid);
        // assert!(handler.events.front().unwrap().ask == new_start_values.ask);
        // We don't have to check inside the above loop because it couldn't possibly finish earlier (unless algo changes?)
        // (self.is_done(), self.ids(events))
        self.is_done()
    }

    // pub(crate) fn init_next(&mut self) {
    //     assert!(self.checks.is_empty());

    //     // TODO: replace unwrap?
    //     let new_base = self.events.pop_front().unwrap();
    //     self.use_base(new_base);
    //     // println!("init_next base: {:?}", self.base);

    //     self.checks.append(&mut self.complete);
    //     assert!(self.complete.is_empty());
    //     for c in self.complete.iter_mut() {
    //         c.reset();
    //     }

    //     for event in self.events.iter() {
    //         Self::proc_event(&mut self.checks, &mut self.complete, &self.base, event);
    //     }
    // }

    // fn use_base(&mut self, new_base: QuoteEvent) {
    //     self.base = new_base;
    //     self.base_date = to_date(&self.base);
    // }

    // pub(crate) fn proc_event(checks: &mut Vec<C>, complete: &mut Vec<C>, base: &QuoteEvent, quote: &QuoteEvent) -> bool {
    //     // Quotes coming in here have already been validated
    //     let bid_change = rounded_diff(quote.bid, base.ask, 2);
    //     let ask_change = rounded_diff(quote.ask, base.bid, 2);
    //     // println!("bid_change: {}, ask_change: {}", bid_change, ask_change);

    //     complete.extend(checks.extract_if(|check| {
    //         let should_continue = check.track(bid_change, ask_change);
    //         // if !should_continue {
    //         //     println!("{}: Check complete on event: {:?}", check.ordinal(), quote);
    //         // }
    //         !should_continue
    //     }));

    //     !checks.is_empty()
    // }

    fn reset_checks(&mut self) {
        self.checks.append(&mut self.complete);
        assert!(self.complete.is_empty());
        for c in self.complete.iter_mut() {
            c.reset();
        }
    }

    // fn start_with(&mut self, start_values: Values, start_date: NaiveDate) {
    //     self.start_values = start_values;
    //     self.start_date = start_date;
    //     to_date(&event)
    //     Values::from(event)
    // }

    fn proc(start_values: &QuoteValues, event: &QuoteEvent, checks: &mut Vec<C>, complete: &mut Vec<C>) -> bool {
        // Quotes coming in here have already been validated
        let bid_change = rounded_diff(event.bid, start_values.ask, 2);
        let ask_change = rounded_diff(event.ask, start_values.bid, 2);
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

    fn process(&mut self, start_values: &QuoteValues, event: &QuoteEvent) -> bool {
        let HandleEvents { ref mut checks, ref mut complete } = self;
        Self::proc(start_values, event, checks, complete)
    }
}

impl<C: Check> Processor<VecDeque<QuoteEvent>,QuoteValues> for HandleEvents<C> {
    fn process(&mut self, start_values: &QuoteValues, events: &mut VecDeque<QuoteEvent>) -> bool {
        self.process(start_values, events.back().unwrap())

        // if !event_in_trading_time(&event) {
        //     self.reset();
        //     return true;
        // }

        // if self.events.is_empty() {
        //     // It's the first event ever or after reset
        //     self.start_with(&event);
        //     self.events.push_back(event);
        //     true
        // } else if !same_date(event.to_date_or_0(), self.start_date) {
        //     self.start_with(&event);
        //     self.events.push_back(event);
        //     true
        // } else {
        //     let should_continue = self.process(&event);
        //     self.events.push_back(event);
        //     should_continue
        // }
    }

    fn reset(&mut self) {
        self.reset_checks();
    }
}

fn rounded_diff(a: f32, b: f32, digits: u8) -> f32 {
    let scale = 10f32.powi(digits as i32);
    ((a - b) * scale).round() / scale
}