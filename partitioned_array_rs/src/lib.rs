//! A Rust partitioned array with independently persisted JSON partitions.
//!
//! The collection presents one logical array while dividing storage into fixed
//! partitions. Partitions can be saved or unloaded independently, and capacity
//! grows by whole partitions when dynamic allocation is enabled.

use serde::{Deserialize, Serialize, de::DeserializeOwned};
use std::fmt;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Config {
    pub partition_size: usize,
    pub initial_partitions: usize,
    pub addition_partitions: usize,
    pub dynamically_allocates: bool,
    pub db_path: PathBuf,
    pub db_name: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            partition_size: 16,
            initial_partitions: 32,
            addition_partitions: 8,
            dynamically_allocates: true,
            db_path: PathBuf::from("partitioned_array"),
            db_name: "partitioned_array_slice".into(),
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidConfig(&'static str),
    Capacity,
    PartitionOutOfRange(usize),
    PartitionUnloaded(usize),
    Io(io::Error),
    Json(serde_json::Error),
    RecordIdOutOfRange(usize),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidConfig(message) => formatter.write_str(message),
            Self::Capacity => formatter.write_str("partitioned array is at capacity"),
            Self::PartitionOutOfRange(index) => {
                write!(formatter, "partition {index} is out of range")
            }
            Self::PartitionUnloaded(index) => {
                write!(formatter, "partition {index} is not loaded")
            }
            Self::Io(error) => write!(formatter, "partitioned array I/O error: {error}"),
            Self::Json(error) => write!(formatter, "partitioned array JSON error: {error}"),
            Self::RecordIdOutOfRange(id) => write!(formatter, "record {id} is out of range"),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

impl From<serde_json::Error> for Error {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

#[derive(Clone, Debug)]
pub struct PartitionedArray<T> {
    config: Config,
    partitions: Vec<Option<Vec<Option<T>>>>,
    next_id: usize,
}

impl<T> PartitionedArray<T> {
    pub fn new(config: Config) -> Result<Self, Error> {
        Self::validate_config(&config)?;
        let partitions = (0..config.initial_partitions)
            .map(|_| Some(Self::empty_partition(config.partition_size)))
            .collect();
        Ok(Self {
            config,
            partitions,
            next_id: 0,
        })
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub fn capacity(&self) -> usize {
        self.partitions.len() * self.config.partition_size
    }

    pub fn partition_count(&self) -> usize {
        self.partitions.len()
    }

    pub fn is_partition_loaded(&self, partition: usize) -> bool {
        self.partitions.get(partition).is_some_and(Option::is_some)
    }

    pub fn next_id(&self) -> usize {
        self.next_id
    }

    pub fn len(&self) -> usize {
        self.partitions
            .iter()
            .filter_map(Option::as_ref)
            .flatten()
            .filter(|item| item.is_some())
            .count()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn get(&self, id: usize) -> Option<&T> {
        let partition = id / self.config.partition_size;
        let offset = id % self.config.partition_size;
        self.partitions
            .get(partition)?
            .as_ref()?
            .get(offset)?
            .as_ref()
    }

    pub fn get_mut(&mut self, id: usize) -> Option<&mut T> {
        let partition = id / self.config.partition_size;
        let offset = id % self.config.partition_size;
        self.partitions
            .get_mut(partition)?
            .as_mut()?
            .get_mut(offset)?
            .as_mut()
    }

    pub fn set(&mut self, id: usize, value: T) -> Result<(), Error> {
        if id >= self.capacity() {
            return Err(Error::RecordIdOutOfRange(id));
        }
        let partition = id / self.config.partition_size;
        let offset = id % self.config.partition_size;
        let slot = self.partitions[partition]
            .as_mut()
            .ok_or(Error::PartitionUnloaded(partition))?
            .get_mut(offset)
            .expect("record offset is within its partition");
        *slot = Some(value);
        self.next_id = self.next_id.max(id + 1);
        Ok(())
    }

    pub fn insert(&mut self, value: T) -> Result<usize, Error> {
        for partition_index in 0..self.partitions.len() {
            let partition = self.partitions[partition_index]
                .as_mut()
                .ok_or(Error::PartitionUnloaded(partition_index))?;
            if let Some((offset, slot)) = partition
                .iter_mut()
                .enumerate()
                .find(|(_, item)| item.is_none())
            {
                *slot = Some(value);
                let id = partition_index * self.config.partition_size + offset;
                self.next_id = self.next_id.max(id + 1);
                return Ok(id);
            }
        }
        if self.config.dynamically_allocates {
            let added = self.config.addition_partitions;
            for _ in 0..added {
                self.partitions
                    .push(Some(Self::empty_partition(self.config.partition_size)));
            }
            return self.insert_loaded(value);
        }
        Err(Error::Capacity)
    }

    pub fn remove(&mut self, id: usize) -> Result<Option<T>, Error> {
        if id >= self.capacity() {
            return Err(Error::RecordIdOutOfRange(id));
        }
        let partition = id / self.config.partition_size;
        let offset = id % self.config.partition_size;
        Ok(self.partitions[partition]
            .as_mut()
            .ok_or(Error::PartitionUnloaded(partition))?[offset]
            .take())
    }

    pub fn records(&self) -> impl Iterator<Item = (usize, &T)> {
        let partition_size = self.config.partition_size;
        self.partitions
            .iter()
            .enumerate()
            .filter_map(|(partition, records)| records.as_ref().map(|records| (partition, records)))
            .flat_map(move |(partition, records)| {
                records
                    .iter()
                    .enumerate()
                    .filter_map(move |(offset, item)| {
                        item.as_ref()
                            .map(|item| (partition * partition_size + offset, item))
                    })
            })
    }

    pub fn partition(&self, index: usize) -> Result<&[Option<T>], Error> {
        self.partitions
            .get(index)
            .ok_or(Error::PartitionOutOfRange(index))?
            .as_deref()
            .ok_or(Error::PartitionUnloaded(index))
    }

    fn database_path(&self) -> PathBuf {
        self.config.db_path.join(&self.config.db_name)
    }

    fn partition_path(&self, index: usize) -> PathBuf {
        self.database_path()
            .join(format!("{}_part_{index}.json", self.config.db_name))
    }

    fn metadata_path(&self) -> PathBuf {
        self.database_path().join("metadata.json")
    }

    fn insert_loaded(&mut self, value: T) -> Result<usize, Error> {
        let partition = self.partitions.len() - self.config.addition_partitions;
        let id = partition * self.config.partition_size;
        self.partitions[partition]
            .as_mut()
            .expect("new partition is loaded")[0] = Some(value);
        self.next_id = self.next_id.max(id + 1);
        Ok(id)
    }

    fn validate_config(config: &Config) -> Result<(), Error> {
        if config.partition_size == 0 {
            return Err(Error::InvalidConfig(
                "partition_size must be greater than zero",
            ));
        }
        if config.initial_partitions == 0 {
            return Err(Error::InvalidConfig(
                "initial_partitions must be greater than zero",
            ));
        }
        if config.addition_partitions == 0 && config.dynamically_allocates {
            return Err(Error::InvalidConfig(
                "addition_partitions must be greater than zero when dynamic allocation is enabled",
            ));
        }
        if config.db_name.is_empty()
            || config.db_name == "."
            || config.db_name == ".."
            || config.db_name.contains('/')
            || config.db_name.contains('\\')
        {
            return Err(Error::InvalidConfig(
                "db_name must be a non-empty file-system-safe name",
            ));
        }
        Ok(())
    }

    fn empty_partition(size: usize) -> Vec<Option<T>> {
        std::iter::repeat_with(|| None).take(size).collect()
    }
}

impl<T: Serialize> PartitionedArray<T> {
    pub fn save_all(&self) -> Result<(), Error> {
        fs::create_dir_all(&self.config.db_path)?;
        let mut metadata = self.config.clone();
        metadata.initial_partitions = self.partitions.len();
        write_atomic_json(
            &self.metadata_path(),
            &Metadata {
                format: 1,
                config: metadata,
                next_id: self.next_id,
            },
        )?;
        for index in 0..self.partitions.len() {
            if self.is_partition_loaded(index) {
                self.save_partition(index)?;
            }
        }
        Ok(())
    }

    pub fn save_partition(&self, index: usize) -> Result<(), Error> {
        let partition = self.partition(index)?;
        fs::create_dir_all(&self.database_path())?;
        write_atomic_json(&self.partition_path(index), &partition)
    }

    pub fn save_and_unload_partition(&mut self, index: usize) -> Result<(), Error> {
        self.save_partition(index)?;
        self.partitions[index] = None;
        Ok(())
    }
}

impl<T: DeserializeOwned> PartitionedArray<T> {
    pub fn load(config: Config) -> Result<Self, Error> {
        Self::validate_config(&config)?;
        let metadata_path = config.db_path.join(&config.db_name).join("metadata.json");
        let metadata: Metadata = read_json(&metadata_path)?;
        if metadata.format != 1 {
            return Err(Error::InvalidConfig(
                "unsupported partitioned array metadata format",
            ));
        }
        Self::validate_config(&metadata.config)?;
        let partition_count = metadata.config.initial_partitions;
        let mut array = Self {
            config: metadata.config,
            partitions: std::iter::repeat_with(|| None)
                .take(partition_count)
                .collect(),
            next_id: metadata.next_id,
        };
        for index in 0..array.partitions.len() {
            array.load_partition(index)?;
        }
        Ok(array)
    }

    pub fn load_partition(&mut self, index: usize) -> Result<(), Error> {
        if index >= self.partitions.len() {
            return Err(Error::PartitionOutOfRange(index));
        }
        if self.partitions[index].is_some() {
            return Ok(());
        }
        let partition: Vec<Option<T>> = read_json(&self.partition_path(index))?;
        if partition.len() != self.config.partition_size {
            return Err(Error::InvalidConfig(
                "persisted partition length does not match partition_size",
            ));
        }
        self.partitions[index] = Some(partition);
        Ok(())
    }
}

#[derive(Serialize, Deserialize)]
struct Metadata {
    format: u32,
    config: Config,
    next_id: usize,
}

fn write_atomic_json<T: Serialize>(path: &Path, value: &T) -> Result<(), Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let temporary = path.with_extension("json.tmp");
    fs::write(&temporary, serde_json::to_vec_pretty(value)?)?;
    fs::rename(temporary, path)?;
    Ok(())
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Result<T, Error> {
    Ok(serde_json::from_slice(&read_bytes(path)?)?)
}

fn read_bytes(path: &Path) -> Result<Vec<u8>, Error> {
    Ok(fs::read(path)?)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::{Deserialize, Serialize};

    #[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
    struct Record {
        name: String,
        value: i32,
    }

    fn scratch(name: &str) -> PathBuf {
        let path =
            std::env::temp_dir().join(format!("partitioned-array-{name}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&path);
        path
    }

    fn config(path: PathBuf) -> Config {
        Config {
            partition_size: 2,
            initial_partitions: 2,
            addition_partitions: 2,
            dynamically_allocates: true,
            db_path: path,
            db_name: "records".into(),
        }
    }

    #[test]
    fn inserts_reuses_holes_and_grows_by_partitions() {
        let mut array = PartitionedArray::new(config(scratch("growth"))).unwrap();
        for value in 0..4 {
            assert_eq!(
                array
                    .insert(Record {
                        name: "item".into(),
                        value,
                    })
                    .unwrap(),
                value as usize
            );
        }
        let removed = array.remove(1).unwrap().unwrap();
        assert_eq!(removed.value, 1);
        let reused = array
            .insert(Record {
                name: "replacement".into(),
                value: 9,
            })
            .unwrap();
        assert_eq!(reused, 1);
        let added = array
            .insert(Record {
                name: "growth".into(),
                value: 10,
            })
            .unwrap();
        assert_eq!(added, 4);
        assert_eq!(array.partition_count(), 4);
        assert_eq!(array.capacity(), 8);
    }

    #[test]
    fn saves_loads_and_evicts_individual_partitions() {
        let path = scratch("persistence");
        let mut array = PartitionedArray::new(config(path.clone())).unwrap();
        array
            .insert(Record {
                name: "first".into(),
                value: 1,
            })
            .unwrap();
        array
            .insert(Record {
                name: "second".into(),
                value: 2,
            })
            .unwrap();
        array.save_all().unwrap();
        array.save_and_unload_partition(0).unwrap();
        assert!(!array.is_partition_loaded(0));
        assert!(array.get(0).is_none());
        assert_eq!(array.get(2), None);
        let mut loaded = PartitionedArray::<Record>::load(config(path.clone())).unwrap();
        assert_eq!(loaded.get(0).unwrap().name, "first");
        assert_eq!(loaded.get(1).unwrap().value, 2);
        loaded.save_and_unload_partition(0).unwrap();
        loaded.load_partition(0).unwrap();
        assert_eq!(loaded.get(0).unwrap().value, 1);
        fs::remove_dir_all(path).unwrap();
    }

    #[test]
    fn invalid_configuration_and_capacity_are_reported() {
        let invalid = Config {
            partition_size: 0,
            ..config(scratch("invalid"))
        };
        assert!(PartitionedArray::<u8>::new(invalid).is_err());
        assert!(
            PartitionedArray::<u8>::new(Config {
                db_name: "..".into(),
                ..config(scratch("unsafe-name"))
            })
            .is_err()
        );
        let fixed = Config {
            initial_partitions: 1,
            dynamically_allocates: false,
            ..config(scratch("capacity"))
        };
        let mut array = PartitionedArray::new(fixed).unwrap();
        array.insert(1).unwrap();
        array.insert(2).unwrap();
        assert!(matches!(array.insert(3), Err(Error::Capacity)));
    }
}
