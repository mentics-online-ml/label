use shared_types::*;
use shared_types::util::*;
use series_store::*;
use kv_store::*;

use crate::handler::*;
use crate::checks::Check;


pub(crate) struct Labeler<C> {
    version: VersionType,
    series: SeriesReader,
    store: KVStore,
    // topic: Topic,
    // checks: Vec<C>,
    handler: HandleEvents<C>,
}

impl<C: Check> Labeler<C> {
    pub(crate) fn new(version: VersionType, mut series: SeriesReader, store: KVStore, topic: Topic, checks: Vec<C>) -> anyhow::Result<Self> {
        series.subscribe(&topic, Offset::Beginning)?;
        // TODO: get latest eventid from store and try to find that in series and start from there - NUM_SERIES
        let base = Self::find_valid_sequence(&mut series)?;
        // println!("new base: {:?}", base);
        let handler = HandleEvents::new(checks, base);
        Ok(Labeler { version, series, store, handler })
    }

    fn find_valid_sequence(series: &mut SeriesReader) -> anyhow::Result<QuoteEvent> {
        'outer: loop {
            println!("Attempting to find valid sequence");
            let ev0 = loop {
                let ev = series.read_into()?;
                if event_in_trading_time(&ev) {
                    break ev;
                }
            };
            let date = to_date(&ev0);

            let mut counter = SERIES_LENGTH;
            let ev1 = loop {
                let ev = series.read_into()?;
                if !valid_time_and_date(&ev, date) {
                    // Start over from scratch because we went outside valid time
                    println!("Found invalid event for date {:?}, starting over: {:?}", date, ev);
                    continue 'outer;
                }

                counter -= 1;
                if counter == 0 { break ev }
            };
            println!("Found initial valid series from\n  {:?}\n  to\n  {:?}", ev0, ev1);
            break Ok(ev1);
        }
    }

    pub(crate) async fn run(&mut self) -> anyhow::Result<()> {
        if !self.proc_new().await {
            return Ok(())
        } else {
            println!("Ended on first at {:?}", self.handler.events.back());
            self.store_result().await?;
        }

        loop {
            self.handler.init_next();
            if !self.handler.is_done() && !self.proc_new().await {
                return Ok(())
            } else {
                println!("Ended at {:?}", self.handler.events.back());
                self.store_result().await?;
            }
        }
    }

    async fn proc_new(&mut self) -> bool {
        self.series.for_each_msg(&mut self.handler).await;
        if !self.handler.is_done() {
            println!("Ran out of messages before labelling");
            false
        } else {
            true
        }
    }

    async fn store_result(&self) -> anyhow::Result<()> {
        let labelled = Labelled {
            event_id: self.handler.base.event_id,
            timestamp: now(),
            label: Self::make_label(&self.handler.complete)
        };
        self.store.write_label(self.version, &labelled).await?;
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
