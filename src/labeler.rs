use shared_types::*;
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
        let base = Self::seek_start(&mut series, &topic)?;
        // println!("new base: {:?}", base);
        let handler = HandleEvents::new(checks, base);
        Ok(Labeler { version, series, store, handler })
    }

    fn seek_start(series: &mut SeriesReader, topic: &Topic) -> anyhow::Result<QuoteEvent> {
        // TODO: get latest eventid from store and try to find that in series
        series.subscribe(topic)?;
        // TODO: validate the series is continuous and inside trading time, so will need to read all messages
        series.seek(topic, Offset::Offset(99))?;
        let msg = series.read()?;
        // let base_msg = msgs.last().expect("No messages from read_count");
        msg_to(&msg)
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
