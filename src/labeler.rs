use shared_types::*;
use shared_types::util::*;
use series_store::*;
use kv_store::*;

use crate::handler::*;
use crate::checks::Check;


pub(crate) struct Labeler<C> {
    series_events: SeriesReader,
    series_label: SeriesWriter,
    label_topic: Topic,
    store: KVStore,
    handler: HandleEvents<C>,
}

impl<C: Check> Labeler<C> {
    pub(crate) async fn new(series_events: SeriesReader, series_label: SeriesWriter, label_topic: Topic, store: KVStore, checks: Vec<C>) -> Self {
        // TODO: get latest eventid from store and try to find that in series and start from NUM_SERIES before that
        // let base = Self::find_valid_sequence(&mut series_events)?;
        // println!("new base: {:?}", base);
        let handler = HandleEvents::new(checks);
        Labeler { series_events, series_label, label_topic, store, handler }
    }

    pub(crate) async fn seek_start(&mut self) -> anyhow::Result<()> {
        let max_offset_from = match self.store.label_max().await?.map(|lab| lab.offset_from) {
            Some(offset_from) => offset_from,
            None => self.series_events.offset_from_oldest(SERIES_LENGTH)?
        };
        println!("Moving series_events to offset: {}", max_offset_from);
        self.series_events.seek(max_offset_from + 1)
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

    //         let mut counter = SERIES_LENGTH;
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

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        let mut count = 0;
        let max = 50;
        loop {
            self.series_events.for_each_msg(&mut self.handler).await;

            if !self.handler.is_done() {
                // I think this can never happen. Maybe put in type system?
                println!("This shouldn't happen. Ran out of messages before labelling? Aborting.");
                return Ok(())
            }

            println!("Ended at {:?}", self.handler.ids());
            self.store_result().await?;

            count += 1;
            if count >= max {
                println!("TODO: debug stopping");
                return Ok(());
            }

            // Future events might satisfy the checks, so keep checking until we need to process more new events.
            while self.handler.move_to_next() {
                println!("move to next");
                self.store_result().await?;
                count += 1;
                if count >= max {
                    println!("TODO: debug stopping");
                    return Ok(());
                }
            }
        }
    }

    async fn store_result(&self) -> anyhow::Result<()> {
        let timestamp = now();
        let LabelIds { event_id, offset_from, offset_to } = self.handler.ids();
        let labeled = LabelStored {
            event_id,
            timestamp,
            partition: PARTITION,
            offset_from,
            offset_to,
            label: Self::make_label(&self.handler.complete)
        };
        self.store.label_store(&labeled).await?;
        let event = LabelEvent::new(event_id, timestamp, offset_from, offset_to, labeled.label);
        let json = serde_json::to_string(&event)?;
        println!("Writing event_id: {event_id}, offset_from: {offset_from} to label series {}", self.label_topic.name);
        self.series_label.write(event_id, &self.label_topic, "key", timestamp, &json).await?;
        Ok(())
    }

    fn make_label(complete: &[C]) -> Label {
        // let value = [ModelFloat; NUM_CHECKS];
        let mut lab = Label::default();
        for check in complete.iter() {
            lab.value[check.ordinal() as usize] = if check.result() { 1.0 } else { 0.0 };
        }
        lab
    }
}
