#[allow(dead_code)]
pub struct Options {
    pub(crate) front_cpu: usize,
    pub(crate) back_cpu: usize,
    pub(crate) enable_wal: bool,
    pub(crate) task_buffer_size: usize,
    pub(crate) lrucache: CacheConfig,
    // pub(crate) write_batch: WriteBatchConfig,
    pub(crate) compact_conf: CompactConfig,
}
#[allow(dead_code)]
pub struct CacheConfig {}

#[allow(dead_code)]
pub struct WriteBatchConfig {}

pub struct CompactConfig {}

pub struct TimeRange {}

#[allow(dead_code)]
pub struct QueryOption {
    timerange: TimeRange,
    db: String,
    table: String,
    series_id: u64,
}
