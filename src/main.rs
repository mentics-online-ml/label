#![feature(extract_if)]

mod checks;
mod labeler;
mod handler;

use shared_types::*;
use series_store::*;
use kv_store::*;
use checks::*;
use labeler::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let logger = StdoutLogger::boxed();
    let events_topic = Topic::new("raw", "SPY", "quote");
    let series_events = SeriesReader::new_topic(logger, &events_topic)?;

    let series_label = SeriesWriter::new();
    let store = KVStore::new(CURRENT_VERSION).await?;

    let checks: Vec<Box<dyn Check>> = vec!(
        Box::new(CheckDown::new(0, -0.40, 0.20)),
        Box::new(CheckDown::new(1, -0.20, 0.10)),
        Box::new(CheckDown::new(2,-0.10, 0.05)),
        Box::new(CheckDown::new(3, -0.02, 0.01)),
        Box::new(CheckUp::new(4, 0.02, -0.01)),
        Box::new(CheckUp::new(5, 0.10, -0.05)),
        Box::new(CheckUp::new(6, 0.20, -0.10)),
        Box::new(CheckUp::new(7, 0.40, -0.20)),
    );

    let label_topic = Topic::new("label", "SPY", "notify");
    let mut labeler = Labeler::new(series_events, series_label, label_topic, store, checks).await;
    labeler.seek_start().await?;
    labeler.run().await?;

    Ok(())
}
