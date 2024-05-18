use std::time::Duration;

use intervals::{Closed, Interval};
use series_store::SeriesReader;
use kv_store::*;
use shared_types::{Event, Label, Labelled, Logger, StdoutLogger};

// TODO: When it first comes up, check if there are enough messages in series-store in the past to fill the buffer.

const CURRENT_VERSION: u32 = 1;
const FORECAST_RANGE: (u64, u64) = (1, 5);

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let range = Interval::closed_unchecked(Duration::from_secs(FORECAST_RANGE.0), Duration::from_secs(FORECAST_RANGE.1));
    let reader = SeriesReader::new(Box::new(StdoutLogger()))?;
    let store = KVStore::new().await?;
    let logger = StdoutLogger();

    let label = RunLabel::new(Box::new(logger), reader, store, range).await?;

    label.run().await?;

    Ok(())
}

struct RunLabel {
    reader: SeriesReader,
    store: KVStore,
    logger: Box<dyn Logger>,
    range: Closed<Duration>,
    max_labelled_event_id: u64,
}

impl RunLabel {
    async fn new(logger: Box<dyn Logger>, reader: SeriesReader, store: KVStore, range: Closed<Duration>) -> anyhow::Result<Self> {
        let max_labelled_event_id = store.max_labelled_event_id(CURRENT_VERSION).await?;
        Ok(RunLabel { logger, reader, store, range, max_labelled_event_id })
    }

    // Get the most recent label so we know where to continue processing.
    // Do we just need the latest label id? It would be the latest event id that was labelled.
    // Maintain a buffer of Events so we can know to update them

    async fn run(&self) -> anyhow::Result<()> {
        self.reader.seek(&self.reader.topics.event, -100)?;
        let future = move |event| async move {
            self.on_event(event).await;
        };
        self.reader.foreach_event(future).await;

        Ok(())
    }

    async fn on_event(&self, event: Event) {
        println!("Processing event {:?}", event);
        // TODO: buffer
        // TODO: ensure processing starts at < max_event_id
        let labeld = Labelled { id: event.id, timestamp: shared_types::now(), label: Label::default() };
        let _ = self.store.write_label(&labeld).await.map_err(|e| {
            self.logger.log(format!("Error {:?} inserting inferred {:?}", &e, &labeld));
        });
    }
}
