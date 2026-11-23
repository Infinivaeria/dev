# partitioned_array

Rust port of the core Selenite PartitionedArray model. A logical array is
divided into fixed-size JSON partitions that can be saved, loaded and evicted
independently.

```rust
use partitioned_array::{Config, PartitionedArray};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct Record {
    name: String,
}

let config = Config {
    db_path: "data".into(),
    db_name: "records".into(),
    partition_size: 64,
    initial_partitions: 4,
    ..Config::default()
};

let mut records = PartitionedArray::new(config)?;
let id = records.insert(Record { name: "Ada".into() })?;
records.save_all()?;
# Ok::<(), partitioned_array::Error>(())
```

Records are generic, must be JSON-serializable for persistence, and keep a
stable numeric ID. Deleted IDs can be reused. When dynamic allocation is enabled,
capacity grows by the configured number of complete partitions. Loading a
database and loading an individually evicted partition are explicit operations.

This crate intentionally does not yet migrate `mic_freq` or the Ruby-based
Selenite grid integration. It provides the Rust foundation for that later step.
