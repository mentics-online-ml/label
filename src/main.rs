#![feature(extract_if)]

mod checks;
mod labeler;
mod handler;

use std::{env, io::Write};

use data_info::CURRENT_VERSION;
use shared_types::*;
use series_store::*;
use kv_store::*;
use checks::*;
use labeler::*;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let mut count = 1000;
    let mut reset = false;
    let args: Vec<String> = env::args().collect();
    if args.len() > 1 {
        let mut index = 1;
        let mut arg = &args[index];
        if arg == "reset" {
            reset = true;
            index += 1;
        }
        arg = &args[index];
        if let Ok(arg_count) = arg.parse::<usize>() {
            count = arg_count;
        }
    }
    // print!("arg 1 : {}", args[1]);

    std::io::stdout().flush()?;
    let events_topic = Topic::new("raw", "SPY", "quote");
    std::io::stdout().flush()?;
    let series_events = SeriesReader::new_topic("label", &events_topic)?;
    std::io::stdout().flush()?;

    let label_topic = Topic::new("label", "SPY", "notify");
    let series_label = SeriesWriter::new(label_topic);
    print!("before` store");
    let store = KVStore::new(CURRENT_VERSION).await?;
    print!("after store");

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

    let mut labeler = Labeler::new(series_events, series_label, store, checks).await;
    if reset {
        labeler.reset_all_label_data().await?;
    }
    labeler.seek_start().await?;
    labeler.run(count).await?;

    Ok(())
}
