use std::collections::VecDeque;

use chrono::NaiveDate;
use shared_types::{util::{event_in_trading_time, same_date, to_date}, *};
use series_store::*;

use crate::checks::Check;

#[derive(Debug)]
pub struct LabelIds {
    pub event_id: EventId,
    pub offset_from: OffsetId,
    pub offset_to: OffsetId,
}

struct Values {
    bid: SeriesFloat,
    ask: SeriesFloat
}

impl Default for Values {
    fn default() -> Self {
        Self { bid: 0.0, ask: 0.0 }
    }
}

impl From<&QuoteEvent> for Values {
    fn from(event: &QuoteEvent) -> Self {
        Self { bid: event.bid, ask: event.ask }
    }
}

pub(crate) struct HandleEvents<C> {
    pub(crate) checks: Vec<C>,
    pub(crate) complete: Vec<C>,
    pub(crate) events: VecDeque<QuoteEvent>,
    start_date: NaiveDate,
    start_values: Values,
}

impl<C: Check> HandleEvents<C> {
    pub(crate) fn new(checks: Vec<C>) -> Self {
        // let start_date = to_date(&first_event);
        // let start_values = Values::from(&first_event);
        // let mut events = VecDeque::new();
        // events.push_back(first_event);
        Self { checks, complete: Vec::new(), events: VecDeque::new(), start_date: NaiveDate::default(), start_values: Values::default() }
    }

    pub(crate) fn is_done(&self) -> bool {
        self.checks.is_empty()
    }

    pub(crate) fn ids(&self) -> LabelIds {
        match (self.events.front(), self.events.back()) {
            (Some(front), Some(back)) => LabelIds {
                event_id: front.event_id, offset_from: front.offset, offset_to: back.offset
            },
            // TODO: use types to make this impossible
            _ => panic!("handler.offsets called when no data")
        }
    }

    pub(crate) fn move_to_next(&mut self) -> bool {
        assert!(self.checks.is_empty());
        self.reset_checks();
        // Move to the next start event
        self.events.pop_front();

        // TODO: replace unwrap?
        let new_first = self.events.front().unwrap();
        // TODO: some way to do this idiom without having to clone or put code in place?
        // self.start_with(new_first);
        self.start_values = Values::from(new_first);
        self.start_date = to_date(new_first);


        let HandleEvents { ref mut checks, ref mut complete, ref mut events, ref start_values, .. } = self;
        for event in events.iter() {
            // Must call proc and not handler so the events are not pushed onto queue again
            Self::proc(start_values, event, checks, complete);
        }
        // We don't have to check inside the above loop because it couldn't possibly finish earlier (unless algo changes?)
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

    fn reset(&mut self) {
        if self.events.is_empty() {
            // If it's empty, we're already reset
            return
        }
        self.reset_checks();
        self.events.clear();
    }

    fn start_with(&mut self, event: &QuoteEvent) {
        self.start_values = Values::from(event);
        self.start_date = to_date(event);
    }

    // fn start_with(&mut self, start_values: Values, start_date: NaiveDate) {
    //     self.start_values = start_values;
    //     self.start_date = start_date;
    //     to_date(&event)
    //     Values::from(event)
    // }

    fn proc(start_values: &Values, event: &QuoteEvent, checks: &mut Vec<C>, complete: &mut Vec<C>) -> bool {
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
}

impl<C: Check> EventHandler<QuoteEvent> for HandleEvents<C> {
    fn handle(&mut self, event: QuoteEvent) -> bool {
        if !event_in_trading_time(&event) {
            self.reset();
            return true;
        }

        if self.events.is_empty() {
            // It's the first event ever or after reset
            self.start_with(&event);
            self.events.push_back(event);
            true
        } else if !same_date(to_date(&event), self.start_date) {
            self.start_with(&event);
            self.events.push_back(event);
            true
        } else {
            let HandleEvents { ref mut checks, ref mut complete, ref start_values, .. } = self;
            // let should_continue = self.proc(&event);
            let should_continue = Self::proc(start_values, &event, checks, complete);
            self.events.push_back(event);
            should_continue
        }
    }
}

fn rounded_diff(a: f32, b: f32, digits: u8) -> f32 {
    let scale = 10f32.powi(digits as i32);
    ((a - b) * scale).round() / scale
}