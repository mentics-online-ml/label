use chrono_util::{now, to_datetime};
use data_info::LabelType;
use label::LabelEvent;
use quote::{QuoteEvent, QuoteValues};
use series_proc::BaseHandler;
use shared_types::*;
use series_store::*;
use kv_store::*;
use stored::LabelStored;

use crate::handler::*;
use crate::checks::Check;

pub(crate) struct Labeler<C:Check> {
    series_events: SeriesReader,
    series_label: SeriesWriter,
    store: KVStore,
    handler: BaseHandler<QuoteValues, QuoteEvent, HandleEvents<C>>,
    // processor: HandleEvents<C>,
}

impl<C: Check> Labeler<C> {
    pub(crate) async fn new(series_events: SeriesReader, series_label: SeriesWriter, store: KVStore, checks: Vec<C>) -> Self {
        // TODO: get latest eventid from store and try to find that in series and start from NUM_SERIES before that
        // let base = Self::find_valid_sequence(&mut series_events)?;
        // println!("new base: {:?}", base);


        let processor = HandleEvents::new(checks);
        let handler: BaseHandler<QuoteValues, QuoteEvent, _> = BaseHandler::new(processor);

        Labeler { series_events, series_label, store, handler }
    }

    pub(crate) async fn seek_start(&mut self) -> anyhow::Result<()> {
        // TODO: this can be simplified
        let max_offset_from = match self.store.label_max().await?.map(|lab| lab.offset_from) {
            Some(offset_from) => offset_from,
            // This was only to avoid error on receiving label events that would be too early to process downstream,
            // but we should handle those cases gracefully anyway.
            // None => self.series_events.offset_from_oldest(SERIES1_LENGTH)?
            None => self.series_events.offset_from_oldest(0)?
        };
        println!("Moving series_events to offset: {}", max_offset_from);
        self.series_events.seek(max_offset_from + 1)
    }

    pub(crate) async fn reset_all_label_data(&self)  -> anyhow::Result<()> {
        println!("**** RESETTING LABEL DATA ****");
        // No need to reset offsets, it seeks each run
        // println!("  resetting offsets");
        // self.series_events.reset_offset()?;
        println!("  deleting label data");
        self.store.reset_label_data().await?;
        Ok(())
    }

    // fn find_valid_sequence(series: &mut SeriesReader) -> anyhow::Result<QuoteEvent> {
    //     'outer: loop {
    //         println!("Attempting to find valid sequence");
    //         let ev0 = loop {
    //             let ev = series.read_into_event()?;
    //             if event_in_trading_time(&ev) {
    //                 break ev;
    //             }
    //         };
    //         let date = to_date(&ev0);

    //         let mut counter = SERIES1_LENGTH;
    //         let ev1 = loop {
    //             let ev = series.read_into_event()?;
    //             if !valid_time_and_date(&ev, date) {
    //                 // Start over from scratch because we went outside valid time
    //                 println!("Found invalid event for date {:?}, starting over: {:?}", date, ev);
    //                 continue 'outer;
    //             }

    //             counter -= 1;
    //             if counter == 0 { break ev }
    //         };
    //         println!("Found initial valid series from\n  {:?}\n  to\n  {:?}", ev0, ev1);
    //         break Ok(ev1);
    //     }
    // }

    pub(crate) async fn run(&mut self, run_count: usize) -> anyhow::Result<()> {
        let mut count = 0;
        let max = run_count;
        loop {
            self.series_events.for_each_msg(&mut self.handler).await;

            if !self.handler.proc.is_done() {
                // I think this can never happen. Maybe put in type system?
                println!("This shouldn't happen. Ran out of messages before labelling? Aborting.");
                return Ok(())
            }

            println!("Finished reading new events at {:?}", self.handler.proc.ids(&self.handler.events));

            // let labeled = Self::make_labeled(&self.handler.proc, &self.handler.events);
            // Self::store_result(&self.store, &labeled).await?;
            // Self::send_event(&self.series_label, &labeled).await?;

            // count += 1;
            // if count >= max {
            //     println!("TODO: debug stopping");
            //     return Ok(());
            // }

            // Future events might satisfy the checks, so keep checking until we need to process more new events.
            loop {
                // {
                    self.store_extra().await?;

                    count += 1;
                    if count >= max {
                        println!("TODO: debug stopping");
                        return Ok(());
                    }
                // }

                self.handler.move_to_next();
                let BaseHandler { ref mut proc, ref start_values, ref mut events } = self.handler;
                if !proc.move_to_next(start_values, events) {
                    break;
                }
            }

            // self.handler.move_to_next();
            // let BaseHandler { ref start_values, ref mut events, ref mut proc } = self.handler;

            // while proc.move_to_next(start_values, events) {
            //     println!("move to next");

            //     let labeled = Self::make_labeled(proc, events);
            //     Self::store_result(&self.store, &labeled).await?;
            //     Self::send_event(&self.series_label, &labeled).await?;

            //     count += 1;
            //     if count >= max {
            //         println!("TODO: debug stopping");
            //         return Ok(());
            //     }
            // }
        }
    }

    async fn store_extra(&mut self) -> anyhow::Result<()> {
        let BaseHandler { ref mut proc, ref mut events, .. } = self.handler;

        let labeled = Self::make_labeled(proc, events);
        Self::store_result(&self.store, &labeled).await?;
        Self::send_event(&self.series_label, &labeled).await?;
        Ok(())
    }

    async fn store_result(store: &KVStore, labeled: &LabelStored) -> anyhow::Result<()> {
        store.label_store(labeled).await?;
        Ok(())
    }

    async fn send_event(series: &SeriesWriter, labeled: &LabelStored) -> anyhow::Result<()> {
        let event = LabelEvent::new(labeled.event_id, labeled.timestamp, labeled.offset_from, labeled.offset_to, labeled.label);
        let json = serde_json::to_string(&event)?;
        // println!("Writing event_id: {}, offsets: {} - {} to label series", event.event_id, event.offset_from, event.offset_to);
        println!("Writing label for {}: {:?}", to_datetime(event.timestamp), event);
        series.write_topic(event.event_id, event.timestamp, &json).await?;
        Ok(())
    }

    fn make_labeled(proc: &HandleEvents<C>, events: &std::collections::VecDeque<QuoteEvent>) -> LabelStored {
        let timestamp = now();
        let ids = proc.ids(events);
        let LabelIds { event_id, offset_from, offset_to } = ids;
        LabelStored {
            event_id,
            timestamp,
            offset_from,
            offset_to,
            label: Self::make_label(&proc.complete)
        }
    }

    fn make_label(complete: &[C]) -> LabelType {
        // let value = [ModelFloat; NUM_CHECKS];
        let mut lab = LabelType::default();
        for check in complete.iter() {
            lab[check.ordinal() as usize] = if check.result() { 1.0 } else { 0.0 };
        }
        lab
    }
}

