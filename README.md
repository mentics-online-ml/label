# label

Any time a code modification occurs that might change how a label is determined, the CURRENT_VERSION constant in lib.rs should be incremented so we don't mix labeling algorithms during training.

Identify and persist the ground truth for the rolling data stream window.

It inserts into kv-store:
(event_id, version_info) => label_info

The determination of the label may change over time.
For the initial version, it will be:
The label for Event0 will be the first subsequent EventN for which:
    (EventN.timestamp - Event0.timestamp) within closed range FORECAST_RANGE

It would seem that the labels could be inserted into another topic in series-store. However, the algorithm for labeling could change over time, and it might reprocess the same data. So, we could have multiple label entries for the same event id. Also, we might skip any number of events to keep up or other reasons. So, it wouldn't fit well in series-store.
