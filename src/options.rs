use crate::encoder::encode_value;
use libc::size_t;
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rocksdb::*;
use std::ops::Deref;
use std::os::raw::{c_int, c_uint};
use std::path::{Path, PathBuf};

#[pyclass(name = "Options")]
pub(crate) struct OptionsPy(pub(crate) Options);

#[pyclass(name = "WriteOptions")]
pub(crate) struct WriteOptionsPy {
    #[pyo3(get, set)]
    sync: bool,

    #[pyo3(get, set)]
    disable_wal: bool,

    #[pyo3(get, set)]
    ignore_missing_column_families: bool,

    #[pyo3(get, set)]
    no_slowdown: bool,

    #[pyo3(get, set)]
    low_pri: bool,

    #[pyo3(get, set)]
    memtable_insert_hint_per_batch: bool,
}

#[pyclass(name = "FlushOptions")]
#[derive(Copy, Clone)]
pub(crate) struct FlushOptionsPy {
    #[pyo3(get, set)]
    pub(crate) wait: bool,
}

#[pyclass(name = "ReadOptions")]
pub(crate) struct ReadOptionsPy(pub(crate) Option<ReadOptions>);

/// Defines the underlying memtable implementation.
/// See official [wiki](https://github.com/facebook/rocksdb/wiki/MemTable) for more information.
#[pyclass(name = "MemtableFactory")]
pub(crate) struct MemtableFactoryPy(MemtableFactory);

/// For configuring block-based file storage.
#[pyclass(name = "BlockBasedOptions")]
pub(crate) struct BlockBasedOptionsPy(BlockBasedOptions);

/// Configuration of cuckoo-based storage.
#[pyclass(name = "CuckooTableOptions")]
pub(crate) struct CuckooTableOptionsPy(CuckooTableOptions);

///
/// Used with DBOptions::set_plain_table_factory.
/// See official [wiki](https://github.com/facebook/rocksdb/wiki/PlainTable-Format) for more
/// information.
///
/// Defaults:
///  user_key_length: 0 (variable length)
///  bloom_bits_per_key: 10
///  hash_table_ratio: 0.75
///  index_sparseness: 16
///
#[pyclass(name = "PlainTableFactoryOptions")]
pub(crate) struct PlainTableFactoryOptionsPy {
    #[pyo3(get, set)]
    user_key_length: u32,

    #[pyo3(get, set)]
    bloom_bits_per_key: i32,

    #[pyo3(get, set)]
    hash_table_ratio: f64,

    #[pyo3(get, set)]
    index_sparseness: usize,
}

#[pyclass(name = "Cache")]
pub(crate) struct CachePy(Cache);

#[pyclass(name = "BlockBasedIndexType")]
pub(crate) struct BlockBasedIndexTypePy(BlockBasedIndexType);

#[pyclass(name = "BlockBasedIndexType")]
pub(crate) struct DataBlockIndexTypePy(DataBlockIndexType);

#[pyclass(name = "SliceTransform")]
pub(crate) struct SliceTransformPy(SliceTransformType);

pub(crate) enum SliceTransformType {
    Fixed(size_t),
    MaxLen(usize),
    NOOP,
}

#[pyclass(name = "DBPath")]
pub(crate) struct DBPathPy {
    path: PathBuf,
    target_size: u64,
}

#[pyclass(name = "DBCompressionType")]
pub(crate) struct DBCompressionTypePy(DBCompressionType);

#[pyclass(name = "DBCompactionStyle")]
pub(crate) struct DBCompactionStylePy(DBCompactionStyle);

#[pyclass(name = "DBRecoveryMode")]
pub(crate) struct DBRecoveryModePy(DBRecoveryMode);

#[pyclass(name = "Env")]
pub(crate) struct EnvPy(Env);

#[pyclass(name = "UniversalCompactOptions")]
pub(crate) struct UniversalCompactOptionsPy {
    #[pyo3(get, set)]
    size_ratio: c_int,

    #[pyo3(get, set)]
    min_merge_width: c_int,

    #[pyo3(get, set)]
    max_merge_width: c_int,

    #[pyo3(get, set)]
    max_size_amplification_percent: c_int,

    #[pyo3(get, set)]
    compression_size_percent: c_int,

    #[pyo3(get, set)]
    stop_style: UniversalCompactionStopStylePy,
}

#[pyclass(name = "UniversalCompactionStopStyle")]
#[derive(Copy, Clone)]
pub(crate) struct UniversalCompactionStopStylePy(UniversalCompactionStopStyle);

#[pyclass(name = "FifoCompactOptions")]
pub(crate) struct FifoCompactOptionsPy {
    #[pyo3(get, set)]
    set_max_table_files_size: u64,
}

#[pymethods]
impl OptionsPy {
    #[new]
    pub fn new() -> Self {
        let mut opt = Options::default();
        opt.create_if_missing(true);
        OptionsPy(opt)
    }

    pub fn increase_parallelism(&mut self, parallelism: i32) {
        self.0.increase_parallelism(parallelism)
    }

    pub fn optimize_level_style_compaction(&mut self, memtable_memory_budget: usize) {
        self.0
            .optimize_level_style_compaction(memtable_memory_budget)
    }

    pub fn optimize_universal_style_compaction(&mut self, memtable_memory_budget: usize) {
        self.0
            .optimize_universal_style_compaction(memtable_memory_budget)
    }

    /// default is changed to True.
    pub fn create_if_missing(&mut self, create_if_missing: bool) {
        self.0.create_if_missing(create_if_missing)
    }

    pub fn create_missing_column_families(&mut self, create_missing_cfs: bool) {
        self.0.create_missing_column_families(create_missing_cfs)
    }

    pub fn set_error_if_exists(&mut self, enabled: bool) {
        self.0.set_error_if_exists(enabled)
    }

    pub fn set_paranoid_checks(&mut self, enabled: bool) {
        self.0.set_paranoid_checks(enabled)
    }

    pub fn set_db_paths(&mut self, paths: &PyList) -> PyResult<()> {
        let mut db_paths = Vec::with_capacity(paths.len());
        for p in paths.iter() {
            let path: &PyCell<DBPathPy> = PyTryFrom::try_from(p)?;
            db_paths.push(
                match DBPath::new(&path.borrow().path, path.borrow().target_size) {
                    Ok(p) => p,
                    Err(e) => return Err(PyException::new_err(e.into_string())),
                },
            );
        }
        Ok(self.0.set_db_paths(&db_paths))
    }

    pub fn set_env(&mut self, env: PyRef<EnvPy>) {
        self.0.set_env(&env.0)
    }

    pub fn set_compression_type(&mut self, t: PyRef<DBCompressionTypePy>) {
        self.0.set_compression_type(t.0)
    }

    pub fn set_compression_per_level(&mut self, level_types: &PyList) -> PyResult<()> {
        let mut result = Vec::with_capacity(level_types.len());
        for py_any in level_types.iter() {
            let level_type: &PyCell<DBCompressionTypePy> = PyTryFrom::try_from(py_any)?;
            result.push(level_type.borrow().0)
        }
        Ok(self.0.set_compression_per_level(&result))
    }

    pub fn set_compression_options(
        &mut self,
        w_bits: c_int,
        level: c_int,
        strategy: c_int,
        max_dict_bytes: c_int,
    ) {
        self.0
            .set_compression_options(w_bits, level, strategy, max_dict_bytes)
    }

    pub fn set_zstd_max_train_bytes(&mut self, value: c_int) {
        self.0.set_zstd_max_train_bytes(value)
    }

    pub fn set_compaction_readahead_size(&mut self, compaction_readahead_size: usize) {
        self.0
            .set_compaction_readahead_size(compaction_readahead_size)
    }

    pub fn set_level_compaction_dynamic_level_bytes(&mut self, v: bool) {
        self.0.set_level_compaction_dynamic_level_bytes(v)
    }

    // pub fn set_merge_operator_associative<F: MergeFn + Clone>(&mut self, name: &str, full_merge_fn: F) {
    //     self.0.set_merge_operator_associative(name, full_merge_fn)
    // }

    // pub fn set_merge_operator<F: MergeFn, PF: MergeFn>(&mut self, name: &str, full_merge_fn: F, partial_merge_fn: PF,) {
    //     self.0.set_merge_operator(name, full_merge_fn, partial_merge_fn,)
    // }

    // pub fn add_merge_operator<F: MergeFn + Clone>(&mut self, name: &str, merge_fn: F) {
    //     self.0.add_merge_operator(name, merge_fn)
    // }

    // pub fn set_compaction_filter<F>(&mut self, name: &str, filter_fn: F) {
    //     self.0.set_compaction_filter(name, filter_fn)
    // }

    // pub fn set_compaction_filter_factory<F>(&mut self, factory: F) {
    //     self.0.set_compaction_filter_factory(factory)
    // }

    // pub fn set_comparator(&mut self, name: &str, compare_fn: CompareFn) {
    //     self.0.set_comparator(name, compare_fn)
    // }

    pub fn set_prefix_extractor(
        &mut self,
        prefix_extractor: PyRef<SliceTransformPy>,
    ) -> PyResult<()> {
        let transform = match prefix_extractor.0 {
            SliceTransformType::Fixed(len) => SliceTransform::create_fixed_prefix(len),
            SliceTransformType::MaxLen(len) => match create_max_len_transform(len) {
                Ok(f) => f,
                Err(_) => {
                    return Err(PyException::new_err(
                        "max len prefix only supports len from 1 to 128",
                    ))
                }
            },
            SliceTransformType::NOOP => SliceTransform::create_noop(),
        };
        Ok(self.0.set_prefix_extractor(transform))
    }

    // pub fn add_comparator(&mut self, name: &str, compare_fn: CompareFn) {
    //     self.0.add_comparator(name, compare_fn)
    // }

    pub fn optimize_for_point_lookup(&mut self, cache_size: u64) {
        self.0.optimize_for_point_lookup(cache_size)
    }

    pub fn set_optimize_filters_for_hits(&mut self, optimize_for_hits: bool) {
        self.0.set_optimize_filters_for_hits(optimize_for_hits)
    }

    pub fn set_delete_obsolete_files_period_micros(&mut self, micros: u64) {
        self.0.set_delete_obsolete_files_period_micros(micros)
    }

    pub fn prepare_for_bulk_load(&mut self) {
        self.0.prepare_for_bulk_load()
    }

    pub fn set_max_open_files(&mut self, nfiles: c_int) {
        self.0.set_max_open_files(nfiles)
    }

    pub fn set_max_file_opening_threads(&mut self, nthreads: c_int) {
        self.0.set_max_file_opening_threads(nthreads)
    }

    pub fn set_use_fsync(&mut self, useit: bool) {
        self.0.set_use_fsync(useit)
    }

    pub fn set_db_log_dir(&mut self, path: &str) {
        self.0.set_db_log_dir(Path::new(path))
    }

    pub fn set_bytes_per_sync(&mut self, nbytes: u64) {
        self.0.set_bytes_per_sync(nbytes)
    }

    pub fn set_wal_bytes_per_sync(&mut self, nbytes: u64) {
        self.0.set_wal_bytes_per_sync(nbytes)
    }

    pub fn set_writable_file_max_buffer_size(&mut self, nbytes: u64) {
        self.0.set_writable_file_max_buffer_size(nbytes)
    }

    pub fn set_allow_concurrent_memtable_write(&mut self, allow: bool) {
        self.0.set_allow_concurrent_memtable_write(allow)
    }

    pub fn set_enable_write_thread_adaptive_yield(&mut self, enabled: bool) {
        self.0.set_enable_write_thread_adaptive_yield(enabled)
    }

    pub fn set_max_sequential_skip_in_iterations(&mut self, num: u64) {
        self.0.set_max_sequential_skip_in_iterations(num)
    }

    pub fn set_use_direct_reads(&mut self, enabled: bool) {
        self.0.set_use_direct_reads(enabled)
    }

    pub fn set_use_direct_io_for_flush_and_compaction(&mut self, enabled: bool) {
        self.0.set_use_direct_io_for_flush_and_compaction(enabled)
    }

    pub fn set_is_fd_close_on_exec(&mut self, enabled: bool) {
        self.0.set_is_fd_close_on_exec(enabled)
    }

    pub fn set_table_cache_num_shard_bits(&mut self, nbits: c_int) {
        self.0.set_table_cache_num_shard_bits(nbits)
    }

    pub fn set_target_file_size_multiplier(&mut self, multiplier: i32) {
        self.0.set_target_file_size_multiplier(multiplier)
    }

    pub fn set_min_write_buffer_number(&mut self, nbuf: c_int) {
        self.0.set_min_write_buffer_number(nbuf)
    }

    pub fn set_max_write_buffer_number(&mut self, nbuf: c_int) {
        self.0.set_max_write_buffer_number(nbuf)
    }

    pub fn set_write_buffer_size(&mut self, size: usize) {
        self.0.set_write_buffer_size(size)
    }

    pub fn set_db_write_buffer_size(&mut self, size: usize) {
        self.0.set_db_write_buffer_size(size)
    }

    pub fn set_max_bytes_for_level_base(&mut self, size: u64) {
        self.0.set_max_bytes_for_level_base(size)
    }

    pub fn set_max_bytes_for_level_multiplier(&mut self, mul: f64) {
        self.0.set_max_bytes_for_level_multiplier(mul)
    }

    pub fn set_max_manifest_file_size(&mut self, size: usize) {
        self.0.set_max_manifest_file_size(size)
    }

    pub fn set_target_file_size_base(&mut self, size: u64) {
        self.0.set_target_file_size_base(size)
    }

    pub fn set_min_write_buffer_number_to_merge(&mut self, to_merge: c_int) {
        self.0.set_min_write_buffer_number_to_merge(to_merge)
    }

    pub fn set_level_zero_file_num_compaction_trigger(&mut self, n: c_int) {
        self.0.set_level_zero_file_num_compaction_trigger(n)
    }

    pub fn set_level_zero_slowdown_writes_trigger(&mut self, n: c_int) {
        self.0.set_level_zero_slowdown_writes_trigger(n)
    }

    pub fn set_level_zero_stop_writes_trigger(&mut self, n: c_int) {
        self.0.set_level_zero_stop_writes_trigger(n)
    }

    pub fn set_compaction_style(&mut self, style: PyRef<DBCompactionStylePy>) {
        self.0.set_compaction_style(style.0)
    }

    pub fn set_universal_compaction_options(&mut self, uco: PyRef<UniversalCompactOptionsPy>) {
        self.0.set_universal_compaction_options(&uco.deref().into())
    }

    pub fn set_fifo_compaction_options(&mut self, fco: PyRef<FifoCompactOptionsPy>) {
        self.0.set_fifo_compaction_options(&fco.deref().into())
    }

    pub fn set_unordered_write(&mut self, unordered: bool) {
        self.0.set_unordered_write(unordered)
    }

    pub fn set_max_subcompactions(&mut self, num: u32) {
        self.0.set_max_subcompactions(num)
    }

    pub fn set_max_background_jobs(&mut self, jobs: c_int) {
        self.0.set_max_background_jobs(jobs)
    }

    pub fn set_disable_auto_compactions(&mut self, disable: bool) {
        self.0.set_disable_auto_compactions(disable)
    }

    pub fn set_memtable_huge_page_size(&mut self, size: size_t) {
        self.0.set_memtable_huge_page_size(size)
    }

    pub fn set_max_successive_merges(&mut self, num: usize) {
        self.0.set_max_successive_merges(num)
    }

    pub fn set_bloom_locality(&mut self, v: u32) {
        self.0.set_bloom_locality(v)
    }

    pub fn set_inplace_update_support(&mut self, enabled: bool) {
        self.0.set_inplace_update_support(enabled)
    }

    pub fn set_inplace_update_locks(&mut self, num: usize) {
        self.0.set_inplace_update_locks(num)
    }

    pub fn set_max_bytes_for_level_multiplier_additional(&mut self, level_values: Vec<i32>) {
        self.0
            .set_max_bytes_for_level_multiplier_additional(&level_values)
    }

    pub fn set_skip_checking_sst_file_sizes_on_db_open(&mut self, value: bool) {
        self.0.set_skip_checking_sst_file_sizes_on_db_open(value)
    }

    pub fn set_max_write_buffer_size_to_maintain(&mut self, size: i64) {
        self.0.set_max_write_buffer_size_to_maintain(size)
    }

    pub fn set_enable_pipelined_write(&mut self, value: bool) {
        self.0.set_enable_pipelined_write(value)
    }

    pub fn set_memtable_factory(&mut self, factory: PyRef<MemtableFactoryPy>) {
        self.0.set_memtable_factory(match factory.0 {
            MemtableFactory::Vector => MemtableFactory::Vector,
            MemtableFactory::HashSkipList {
                bucket_count,
                height,
                branching_factor,
            } => MemtableFactory::HashSkipList {
                bucket_count,
                height,
                branching_factor,
            },
            MemtableFactory::HashLinkList { bucket_count } => {
                MemtableFactory::HashLinkList { bucket_count }
            }
        })
    }

    pub fn set_block_based_table_factory(&mut self, factory: PyRef<BlockBasedOptionsPy>) {
        self.0.set_block_based_table_factory(&factory.0)
    }

    pub fn set_cuckoo_table_factory(&mut self, factory: PyRef<CuckooTableOptionsPy>) {
        self.0.set_cuckoo_table_factory(&factory.0)
    }

    pub fn set_plain_table_factory(&mut self, options: PyRef<PlainTableFactoryOptionsPy>) {
        self.0.set_plain_table_factory(&options.deref().into())
    }

    pub fn set_min_level_to_compress(&mut self, lvl: c_int) {
        self.0.set_min_level_to_compress(lvl)
    }

    pub fn set_report_bg_io_stats(&mut self, enable: bool) {
        self.0.set_report_bg_io_stats(enable)
    }

    pub fn set_max_total_wal_size(&mut self, size: u64) {
        self.0.set_max_total_wal_size(size)
    }

    pub fn set_wal_recovery_mode(&mut self, mode: PyRef<DBRecoveryModePy>) {
        self.0.set_wal_recovery_mode(mode.0)
    }

    pub fn enable_statistics(&mut self) {
        self.0.enable_statistics()
    }

    pub fn get_statistics(&self) -> Option<String> {
        self.0.get_statistics()
    }

    pub fn set_stats_dump_period_sec(&mut self, period: c_uint) {
        self.0.set_stats_dump_period_sec(period)
    }

    pub fn set_stats_persist_period_sec(&mut self, period: c_uint) {
        self.0.set_stats_persist_period_sec(period)
    }

    pub fn set_advise_random_on_open(&mut self, advise: bool) {
        self.0.set_advise_random_on_open(advise)
    }

    // pub fn set_access_hint_on_compaction_start(&mut self, pattern: AccessHint) {
    //     self.0.set_access_hint_on_compaction_start(pattern)
    // }

    pub fn set_use_adaptive_mutex(&mut self, enabled: bool) {
        self.0.set_use_adaptive_mutex(enabled)
    }

    pub fn set_num_levels(&mut self, n: c_int) {
        self.0.set_num_levels(n)
    }

    pub fn set_memtable_prefix_bloom_ratio(&mut self, ratio: f64) {
        self.0.set_memtable_prefix_bloom_ratio(ratio)
    }

    pub fn set_max_compaction_bytes(&mut self, nbytes: u64) {
        self.0.set_max_compaction_bytes(nbytes)
    }

    pub fn set_wal_dir(&mut self, path: &str) {
        self.0.set_wal_dir(Path::new(path))
    }

    pub fn set_wal_ttl_seconds(&mut self, secs: u64) {
        self.0.set_wal_ttl_seconds(secs)
    }

    pub fn set_wal_size_limit_mb(&mut self, size: u64) {
        self.0.set_wal_size_limit_mb(size)
    }

    pub fn set_manifest_preallocation_size(&mut self, size: usize) {
        self.0.set_manifest_preallocation_size(size)
    }

    pub fn set_purge_redundant_kvs_while_flush(&mut self, enabled: bool) {
        self.0.set_purge_redundant_kvs_while_flush(enabled)
    }

    pub fn set_skip_stats_update_on_db_open(&mut self, skip: bool) {
        self.0.set_skip_stats_update_on_db_open(skip)
    }

    pub fn set_keep_log_file_num(&mut self, nfiles: usize) {
        self.0.set_keep_log_file_num(nfiles)
    }

    pub fn set_allow_mmap_writes(&mut self, is_enabled: bool) {
        self.0.set_allow_mmap_writes(is_enabled)
    }

    pub fn set_allow_mmap_reads(&mut self, is_enabled: bool) {
        self.0.set_allow_mmap_reads(is_enabled)
    }

    pub fn set_atomic_flush(&mut self, atomic_flush: bool) {
        self.0.set_atomic_flush(atomic_flush)
    }

    pub fn set_row_cache(&mut self, cache: PyRef<CachePy>) {
        self.0.set_row_cache(&cache.0)
    }

    pub fn set_ratelimiter(
        &mut self,
        rate_bytes_per_sec: i64,
        refill_period_us: i64,
        fairness: i32,
    ) {
        self.0
            .set_ratelimiter(rate_bytes_per_sec, refill_period_us, fairness)
    }

    pub fn set_max_log_file_size(&mut self, size: usize) {
        self.0.set_max_log_file_size(size)
    }

    pub fn set_log_file_time_to_roll(&mut self, secs: usize) {
        self.0.set_log_file_time_to_roll(secs)
    }

    pub fn set_recycle_log_file_num(&mut self, num: usize) {
        self.0.set_recycle_log_file_num(num)
    }

    pub fn set_soft_rate_limit(&mut self, limit: f64) {
        self.0.set_soft_rate_limit(limit)
    }

    pub fn set_hard_rate_limit(&mut self, limit: f64) {
        self.0.set_hard_rate_limit(limit)
    }

    pub fn set_soft_pending_compaction_bytes_limit(&mut self, limit: usize) {
        self.0.set_soft_pending_compaction_bytes_limit(limit)
    }

    pub fn set_hard_pending_compaction_bytes_limit(&mut self, limit: usize) {
        self.0.set_hard_pending_compaction_bytes_limit(limit)
    }

    pub fn set_rate_limit_delay_max_milliseconds(&mut self, millis: c_uint) {
        self.0.set_rate_limit_delay_max_milliseconds(millis)
    }

    pub fn set_arena_block_size(&mut self, size: usize) {
        self.0.set_arena_block_size(size)
    }

    pub fn set_dump_malloc_stats(&mut self, enabled: bool) {
        self.0.set_dump_malloc_stats(enabled)
    }

    pub fn set_memtable_whole_key_filtering(&mut self, whole_key_filter: bool) {
        self.0.set_memtable_whole_key_filtering(whole_key_filter)
    }
}

#[pymethods]
impl WriteOptionsPy {
    #[new]
    pub fn new() -> Self {
        WriteOptionsPy {
            sync: false,
            disable_wal: false,
            ignore_missing_column_families: false,
            no_slowdown: false,
            low_pri: false,
            memtable_insert_hint_per_batch: false,
        }
    }

    /// Sets the sync mode. If true, the write will be flushed
    /// from the operating system buffer cache before the write is considered complete.
    /// If this flag is true, writes will be slower.
    ///
    /// Default: false
    pub fn set_sync(&mut self, sync: bool) {
        self.sync = sync
    }

    /// Sets whether WAL should be active or not.
    /// If true, writes will not first go to the write ahead log,
    /// and the write may got lost after a crash.
    ///
    /// Default: false
    pub fn disable_wal(&mut self, disable: bool) {
        self.disable_wal = disable
    }

    /// If true and if user is trying to write to column families that don't exist (they were dropped),
    /// ignore the write (don't return an error). If there are multiple writes in a WriteBatch,
    /// other writes will succeed.
    ///
    /// Default: false
    pub fn set_ignore_missing_column_families(&mut self, ignore: bool) {
        self.ignore_missing_column_families = ignore
    }

    /// If true and we need to wait or sleep for the write request, fails
    /// immediately with Status::Incomplete().
    ///
    /// Default: false
    pub fn set_no_slowdown(&mut self, no_slowdown: bool) {
        self.no_slowdown = no_slowdown
    }

    /// If true, this write request is of lower priority if compaction is
    /// behind. In this case, no_slowdown = true, the request will be cancelled
    /// immediately with Status::Incomplete() returned. Otherwise, it will be
    /// slowed down. The slowdown value is determined by RocksDB to guarantee
    /// it introduces minimum impacts to high priority writes.
    ///
    /// Default: false
    pub fn set_low_pri(&mut self, v: bool) {
        self.low_pri = v
    }

    /// If true, writebatch will maintain the last insert positions of each
    /// memtable as hints in concurrent write. It can improve write performance
    /// in concurrent writes if keys in one writebatch are sequential. In
    /// non-concurrent writes (when concurrent_memtable_writes is false) this
    /// option will be ignored.
    ///
    /// Default: false
    pub fn set_memtable_insert_hint_per_batch(&mut self, v: bool) {
        self.memtable_insert_hint_per_batch = v
    }
}

impl From<&WriteOptionsPy> for WriteOptions {
    fn from(w_opt: &WriteOptionsPy) -> Self {
        let mut opt = WriteOptions::default();
        opt.set_sync(w_opt.sync);
        opt.disable_wal(w_opt.disable_wal);
        opt.set_ignore_missing_column_families(w_opt.ignore_missing_column_families);
        opt.set_low_pri(w_opt.low_pri);
        opt.set_memtable_insert_hint_per_batch(w_opt.memtable_insert_hint_per_batch);
        opt.set_no_slowdown(w_opt.no_slowdown);
        opt
    }
}

#[pymethods]
impl FlushOptionsPy {
    #[new]
    pub fn new() -> Self {
        FlushOptionsPy { wait: true }
    }

    pub fn set_wait(&mut self, wait: bool) {
        self.wait = wait
    }
}

impl From<&FlushOptionsPy> for FlushOptions {
    fn from(f_opt: &FlushOptionsPy) -> Self {
        let mut opt = FlushOptions::default();
        opt.set_wait(f_opt.wait);
        opt
    }
}

#[pymethods]
impl ReadOptionsPy {
    #[new]
    pub fn default() -> Self {
        ReadOptionsPy(Some(ReadOptions::default()))
    }

    /// Specify whether the "data block"/"index block"/"filter block"
    /// read for this iteration should be cached in memory?
    /// Callers may wish to set this field to false for bulk scans.
    ///
    /// Default: true
    pub fn fill_cache(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.fill_cache(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Sets the upper bound for an iterator.
    /// The upper bound itself is not included on the iteration result.
    pub fn set_iterate_upper_bound(&mut self, key: &PyAny) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_iterate_upper_bound(encode_value(key)?))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Sets the lower bound for an iterator.
    pub fn set_iterate_lower_bound(&mut self, key: &PyAny) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_iterate_lower_bound(encode_value(key)?))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Enforce that the iterator only iterates over the same
    /// prefix as the seek.
    /// This option is effective only for prefix seeks, i.e. prefix_extractor is
    /// non-null for the column family and total_order_seek is false.  Unlike
    /// iterate_upper_bound, prefix_same_as_start only works within a prefix
    /// but in both directions.
    ///
    /// Default: false
    pub fn set_prefix_same_as_start(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_prefix_same_as_start(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Enable a total order seek regardless of index format (e.g. hash index)
    /// used in the table. Some table format (e.g. plain table) may not support
    /// this option.
    ///
    /// If true when calling Get(), we also skip prefix bloom when reading from
    /// block based table. It provides a way to read existing data after
    /// changing implementation of prefix extractor.
    pub fn set_total_order_seek(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_total_order_seek(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Sets a threshold for the number of keys that can be skipped
    /// before failing an iterator seek as incomplete. The default value of 0 should be used to
    /// never fail a request as incomplete, even on skipping too many keys.
    ///
    /// Default: 0
    pub fn set_max_skippable_internal_keys(&mut self, num: u64) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_max_skippable_internal_keys(num))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// If true, when PurgeObsoleteFile is called in CleanupIteratorState, we schedule a background job
    /// in the flush job queue and delete obsolete files in background.
    ///
    /// Default: false
    pub fn set_background_purge_on_interator_cleanup(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_background_purge_on_interator_cleanup(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// If true, keys deleted using the DeleteRange() API will be visible to
    /// readers until they are naturally deleted during compaction. This improves
    /// read performance in DBs with many range deletions.
    ///
    /// Default: false
    pub fn set_ignore_range_deletions(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_ignore_range_deletions(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// If true, all data read from underlying storage will be
    /// verified against corresponding checksums.
    ///
    /// Default: true
    pub fn set_verify_checksums(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_verify_checksums(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// If non-zero, an iterator will create a new table reader which
    /// performs reads of the given size. Using a large size (> 2MB) can
    /// improve the performance of forward iteration on spinning disks.
    /// Default: 0
    ///
    /// ```
    /// use rocksdb::{ReadOptions};
    ///
    /// let mut opts = ReadOptions::default();
    /// opts.set_readahead_size(4_194_304); // 4mb
    /// ```
    pub fn set_readahead_size(&mut self, v: usize) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_readahead_size(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// If true, create a tailing iterator. Note that tailing iterators
    /// only support moving in the forward direction. Iterating in reverse
    /// or seek_to_last are not supported.
    pub fn set_tailing(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_tailing(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }

    /// Specifies the value of "pin_data". If true, it keeps the blocks
    /// loaded by the iterator pinned in memory as long as the iterator is not deleted,
    /// If used when reading from tables created with
    /// BlockBasedTableOptions::use_delta_encoding = false,
    /// Iterator's property "rocksdb.iterator.is-key-pinned" is guaranteed to
    /// return 1.
    ///
    /// Default: false
    pub fn set_pin_data(&mut self, v: bool) -> PyResult<()> {
        if let Some(opt) = &mut self.0 {
            Ok(opt.set_pin_data(v))
        } else {
            Err(PyException::new_err(
                "this `ReadOptions` instance is already consumed, create a new ReadOptions()",
            ))
        }
    }
}

#[pymethods]
impl MemtableFactoryPy {
    #[staticmethod]
    pub fn vector() -> Self {
        MemtableFactoryPy(MemtableFactory::Vector)
    }

    #[staticmethod]
    pub fn hash_skip_list(bucket_count: usize, height: i32, branching_factor: i32) -> Self {
        MemtableFactoryPy(MemtableFactory::HashSkipList {
            bucket_count,
            height,
            branching_factor,
        })
    }

    #[staticmethod]
    pub fn hash_link_list(bucket_count: usize) -> Self {
        MemtableFactoryPy(MemtableFactory::HashLinkList { bucket_count })
    }
}

#[pymethods]
impl BlockBasedOptionsPy {
    #[new]
    pub fn default() -> Self {
        BlockBasedOptionsPy(BlockBasedOptions::default())
    }

    /// Approximate size of user data packed per block. Note that the
    /// block size specified here corresponds to uncompressed data. The
    /// actual size of the unit read from disk may be smaller if
    /// compression is enabled. This parameter can be changed dynamically.
    pub fn set_block_size(&mut self, size: usize) {
        self.0.set_block_size(size)
    }

    /// Block size for partitioned metadata. Currently applied to indexes when
    /// kTwoLevelIndexSearch is used and to filters when partition_filters is used.
    /// Note: Since in the current implementation the filters and index partitions
    /// are aligned, an index/filter block is created when either index or filter
    /// block size reaches the specified limit.
    ///
    /// Note: this limit is currently applied to only index blocks; a filter
    /// partition is cut right after an index block is cut.
    pub fn set_metadata_block_size(&mut self, size: usize) {
        self.0.set_metadata_block_size(size)
    }

    /// Note: currently this option requires kTwoLevelIndexSearch to be set as
    /// well.
    ///
    /// Use partitioned full filters for each SST file. This option is
    /// incompatible with block-based filters.
    pub fn set_partition_filters(&mut self, size: bool) {
        self.0.set_partition_filters(size)
    }

    /// Sets global cache for blocks (user data is stored in a set of blocks, and
    /// a block is the unit of reading from disk). Cache must outlive DB instance which uses it.
    ///
    /// If set, use the specified cache for blocks.
    /// By default, rocksdb will automatically create and use an 8MB internal cache.
    pub fn set_block_cache(&mut self, cache: PyRef<CachePy>) {
        self.0.set_block_cache(&cache.0)
    }

    /// Sets global cache for compressed blocks. Cache must outlive DB instance which uses it.
    ///
    /// By default, rocksdb will not use a compressed block cache.
    pub fn set_block_cache_compressed(&mut self, cache: PyRef<CachePy>) {
        self.0.set_block_cache_compressed(&cache.0)
    }

    /// Disable block cache
    pub fn disable_cache(&mut self) {
        self.0.disable_cache()
    }

    /// Sets the filter policy to reduce disk read
    pub fn set_bloom_filter(&mut self, bits_per_key: c_int, block_based: bool) {
        self.0.set_bloom_filter(bits_per_key, block_based)
    }

    pub fn set_cache_index_and_filter_blocks(&mut self, v: bool) {
        self.0.set_cache_index_and_filter_blocks(v)
    }

    /// Defines the index type to be used for SS-table lookups.
    ///
    /// # Examples
    ///
    /// ```python
    /// from rocksdict import BlockBasedOptions, BlockBasedIndexType, Options
    ///
    /// opts = Options()
    /// block_opts = BlockBasedOptions()
    /// block_opts.set_index_type(BlockBasedIndexType.hash_search())
    /// opts.set_block_based_table_factory(block_opts)
    /// ```
    pub fn set_index_type(&mut self, index_type: PyRef<BlockBasedIndexTypePy>) {
        self.0.set_index_type(match index_type.0 {
            BlockBasedIndexType::BinarySearch => BlockBasedIndexType::BinarySearch,
            BlockBasedIndexType::HashSearch => BlockBasedIndexType::HashSearch,
            BlockBasedIndexType::TwoLevelIndexSearch => BlockBasedIndexType::TwoLevelIndexSearch,
        })
    }

    /// If cache_index_and_filter_blocks is true and the below is true, then
    /// filter and index blocks are stored in the cache, but a reference is
    /// held in the "table reader" object so the blocks are pinned and only
    /// evicted from cache when the table reader is freed.
    ///
    /// Default: false.
    pub fn set_pin_l0_filter_and_index_blocks_in_cache(&mut self, v: bool) {
        self.0.set_pin_l0_filter_and_index_blocks_in_cache(v)
    }

    /// If cache_index_and_filter_blocks is true and the below is true, then
    /// the top-level index of partitioned filter and index blocks are stored in
    /// the cache, but a reference is held in the "table reader" object so the
    /// blocks are pinned and only evicted from cache when the table reader is
    /// freed. This is not limited to l0 in LSM tree.
    ///
    /// Default: false.
    pub fn set_pin_top_level_index_and_filter(&mut self, v: bool) {
        self.0.set_pin_top_level_index_and_filter(v)
    }

    /// Format version, reserved for backward compatibility.
    ///
    /// See full [list](https://github.com/facebook/rocksdb/blob/f059c7d9b96300091e07429a60f4ad55dac84859/include/rocksdb/table.h#L249-L274)
    /// of the supported versions.
    ///
    /// Default: 2.
    pub fn set_format_version(&mut self, version: i32) {
        self.0.set_format_version(version)
    }

    /// Number of keys between restart points for delta encoding of keys.
    /// This parameter can be changed dynamically. Most clients should
    /// leave this parameter alone. The minimum value allowed is 1. Any smaller
    /// value will be silently overwritten with 1.
    ///
    /// Default: 16.
    pub fn set_block_restart_interval(&mut self, interval: i32) {
        self.0.set_block_restart_interval(interval)
    }

    /// Same as block_restart_interval but used for the index block.
    /// If you don't plan to run RocksDB before version 5.16 and you are
    /// using `index_block_restart_interval` > 1, you should
    /// probably set the `format_version` to >= 4 as it would reduce the index size.
    ///
    /// Default: 1.
    pub fn set_index_block_restart_interval(&mut self, interval: i32) {
        self.0.set_index_block_restart_interval(interval)
    }

    /// Set the data block index type for point lookups:
    ///  `DataBlockIndexType::BinarySearch` to use binary search within the data block.
    ///  `DataBlockIndexType::BinaryAndHash` to use the data block hash index in combination with
    ///  the normal binary search.
    ///
    /// The hash table utilization ratio is adjustable using [`set_data_block_hash_ratio`](#method.set_data_block_hash_ratio), which is
    /// valid only when using `DataBlockIndexType::BinaryAndHash`.
    ///
    /// Default: `BinarySearch`
    /// # Examples
    ///
    /// ```python
    /// from rocksdict import BlockBasedOptions, BlockBasedIndexType, Options
    ///
    /// opts = Options()
    /// block_opts = BlockBasedOptions()
    /// block_opts.set_data_block_index_type(DataBlockIndexType.binary_and_hash())
    /// block_opts.set_data_block_hash_ratio(0.85)
    /// opts.set_block_based_table_factory(block_opts)
    /// ```
    pub fn set_data_block_index_type(&mut self, index_type: PyRef<DataBlockIndexTypePy>) {
        self.0.set_data_block_index_type(match index_type.0 {
            DataBlockIndexType::BinarySearch => DataBlockIndexType::BinarySearch,
            DataBlockIndexType::BinaryAndHash => DataBlockIndexType::BinaryAndHash,
        })
    }

    /// Set the data block hash index utilization ratio.
    ///
    /// The smaller the utilization ratio, the less hash collisions happen, and so reduce the risk for a
    /// point lookup to fall back to binary search due to the collisions. A small ratio means faster
    /// lookup at the price of more space overhead.
    ///
    /// Default: 0.75
    pub fn set_data_block_hash_ratio(&mut self, ratio: f64) {
        self.0.set_data_block_hash_ratio(ratio)
    }
}

#[pymethods]
impl CuckooTableOptionsPy {
    #[new]
    pub fn default() -> Self {
        CuckooTableOptionsPy(CuckooTableOptions::default())
    }

    /// Determines the utilization of hash tables. Smaller values
    /// result in larger hash tables with fewer collisions.
    /// Default: 0.9
    pub fn set_hash_ratio(&mut self, ratio: f64) {
        self.0.set_hash_ratio(ratio)
    }

    /// A property used by builder to determine the depth to go to
    /// to search for a path to displace elements in case of
    /// collision. See Builder.MakeSpaceForKey method. Higher
    /// values result in more efficient hash tables with fewer
    /// lookups but take more time to build.
    /// Default: 100
    pub fn set_max_search_depth(&mut self, depth: u32) {
        self.0.set_max_search_depth(depth)
    }

    /// In case of collision while inserting, the builder
    /// attempts to insert in the next cuckoo_block_size
    /// locations before skipping over to the next Cuckoo hash
    /// function. This makes lookups more cache friendly in case
    /// of collisions.
    /// Default: 5
    pub fn set_cuckoo_block_size(&mut self, size: u32) {
        self.0.set_cuckoo_block_size(size)
    }

    /// If this option is enabled, user key is treated as uint64_t and its value
    /// is used as hash value directly. This option changes builder's behavior.
    /// Reader ignore this option and behave according to what specified in
    /// table property.
    /// Default: false
    pub fn set_identity_as_first_hash(&mut self, flag: bool) {
        self.0.set_identity_as_first_hash(flag)
    }

    /// If this option is set to true, module is used during hash calculation.
    /// This often yields better space efficiency at the cost of performance.
    /// If this option is set to false, # of entries in table is constrained to
    /// be power of two, and bit and is used to calculate hash, which is faster in general.
    /// Default: true
    pub fn set_use_module_hash(&mut self, flag: bool) {
        self.0.set_use_module_hash(flag)
    }
}

#[pymethods]
impl PlainTableFactoryOptionsPy {
    #[new]
    pub fn default() -> Self {
        PlainTableFactoryOptionsPy {
            user_key_length: 0,
            bloom_bits_per_key: 10,
            hash_table_ratio: 0.75,
            index_sparseness: 16,
        }
    }
}

impl From<&PlainTableFactoryOptionsPy> for PlainTableFactoryOptions {
    fn from(p_opt: &PlainTableFactoryOptionsPy) -> Self {
        PlainTableFactoryOptions {
            // One extra byte for python object type
            user_key_length: if p_opt.user_key_length > 0 {
                p_opt.user_key_length + 1
            } else {
                0
            },
            bloom_bits_per_key: p_opt.bloom_bits_per_key,
            hash_table_ratio: p_opt.hash_table_ratio,
            index_sparseness: p_opt.index_sparseness,
        }
    }
}

#[pymethods]
impl CachePy {
    /// Create a lru cache with capacity
    #[new]
    pub fn new_lru_cache(capacity: size_t) -> PyResult<CachePy> {
        match Cache::new_lru_cache(capacity) {
            Ok(cache) => Ok(CachePy(cache)),
            Err(e) => Err(PyException::new_err(e.into_string())),
        }
    }

    /// Returns the Cache memory usage
    pub fn get_usage(&self) -> usize {
        self.0.get_usage()
    }

    /// Returns pinned memory usage
    pub fn get_pinned_usage(&self) -> usize {
        self.0.get_pinned_usage()
    }

    /// Sets cache capacity
    pub fn set_capacity(&mut self, capacity: size_t) {
        self.0.set_capacity(capacity)
    }
}

#[pymethods]
impl BlockBasedIndexTypePy {
    /// A space efficient index block that is optimized for
    /// binary-search-based index.
    #[staticmethod]
    pub fn binary_search() -> Self {
        BlockBasedIndexTypePy(BlockBasedIndexType::BinarySearch)
    }

    /// The hash index, if enabled, will perform a hash lookup if
    /// a prefix extractor has been provided through Options::set_prefix_extractor.
    #[staticmethod]
    pub fn hash_search() -> Self {
        BlockBasedIndexTypePy(BlockBasedIndexType::HashSearch)
    }

    /// A two-level index implementation. Both levels are binary search indexes.
    #[staticmethod]
    pub fn two_level_index_search() -> Self {
        BlockBasedIndexTypePy(BlockBasedIndexType::TwoLevelIndexSearch)
    }
}

#[pymethods]
impl DataBlockIndexTypePy {
    /// Use binary search when performing point lookup for keys in data blocks.
    /// This is the default.
    #[staticmethod]
    pub fn binary_search() -> Self {
        DataBlockIndexTypePy(DataBlockIndexType::BinarySearch)
    }

    /// Appends a compact hash table to the end of the data block for efficient indexing. Backwards
    /// compatible with databases created without this feature. Once turned on, existing data will
    /// be gradually converted to the hash index format.
    #[staticmethod]
    pub fn binary_and_hash() -> Self {
        DataBlockIndexTypePy(DataBlockIndexType::BinaryAndHash)
    }
}

#[pymethods]
impl SliceTransformPy {
    #[staticmethod]
    pub fn create_fixed_prefix(len: size_t) -> Self {
        SliceTransformPy(SliceTransformType::Fixed(len))
    }

    ///
    /// prefix max length at `len`. If key is longer than `len`,
    /// the prefix will have length `len`, if key is shorter than `len`,
    /// the prefix will have the same length as `len`.
    ///
    #[staticmethod]
    pub fn create_max_len_prefix(len: usize) -> Self {
        SliceTransformPy(SliceTransformType::MaxLen(len))
    }

    #[staticmethod]
    pub fn create_noop() -> Self {
        SliceTransformPy(SliceTransformType::NOOP)
    }
}

#[pymethods]
impl DBPathPy {
    #[new]
    pub fn new(path: &str, target_size: u64) -> Self {
        DBPathPy {
            path: PathBuf::from(path),
            target_size,
        }
    }
}

#[pymethods]
impl DBCompressionTypePy {
    #[staticmethod]
    pub fn none() -> Self {
        DBCompressionTypePy(DBCompressionType::None)
    }

    #[staticmethod]
    pub fn snappy() -> Self {
        DBCompressionTypePy(DBCompressionType::Snappy)
    }

    #[staticmethod]
    pub fn zlib() -> Self {
        DBCompressionTypePy(DBCompressionType::Zlib)
    }

    #[staticmethod]
    pub fn bz2() -> Self {
        DBCompressionTypePy(DBCompressionType::Bz2)
    }

    #[staticmethod]
    pub fn lz4() -> Self {
        DBCompressionTypePy(DBCompressionType::Lz4)
    }

    #[staticmethod]
    pub fn lz4hc() -> Self {
        DBCompressionTypePy(DBCompressionType::Lz4hc)
    }

    #[staticmethod]
    pub fn zstd() -> Self {
        DBCompressionTypePy(DBCompressionType::Zstd)
    }
}

#[pymethods]
impl DBCompactionStylePy {
    #[staticmethod]
    pub fn level_style() -> Self {
        DBCompactionStylePy(DBCompactionStyle::Level)
    }

    #[staticmethod]
    pub fn universal_style() -> Self {
        DBCompactionStylePy(DBCompactionStyle::Universal)
    }

    #[staticmethod]
    pub fn fifo_style() -> Self {
        DBCompactionStylePy(DBCompactionStyle::Fifo)
    }
}

#[pymethods]
impl DBRecoveryModePy {
    #[staticmethod]
    pub fn tolerate_corrupted_tail_records_mode() -> Self {
        DBRecoveryModePy(DBRecoveryMode::TolerateCorruptedTailRecords)
    }

    #[staticmethod]
    pub fn absolute_consistency_mode() -> Self {
        DBRecoveryModePy(DBRecoveryMode::AbsoluteConsistency)
    }

    #[staticmethod]
    pub fn point_in_time_mode() -> Self {
        DBRecoveryModePy(DBRecoveryMode::PointInTime)
    }

    #[staticmethod]
    pub fn skip_any_corrupted_record_mode() -> Self {
        DBRecoveryModePy(DBRecoveryMode::SkipAnyCorruptedRecord)
    }
}

#[pymethods]
impl EnvPy {
    /// Returns default env
    #[new]
    pub fn default() -> PyResult<Self> {
        match Env::default() {
            Ok(env) => Ok(EnvPy(env)),
            Err(e) => Err(PyException::new_err(e.into_string())),
        }
    }

    /// Returns a new environment that stores its data in memory and delegates
    /// all non-file-storage tasks to base_env.
    #[staticmethod]
    pub fn mem_env() -> PyResult<Self> {
        match Env::mem_env() {
            Ok(env) => Ok(EnvPy(env)),
            Err(e) => Err(PyException::new_err(e.into_string())),
        }
    }

    /// Sets the number of background worker threads of a specific thread pool for this environment.
    /// `LOW` is the default pool.
    ///
    /// Default: 1
    pub fn set_background_threads(&mut self, num_threads: c_int) {
        self.0.set_background_threads(num_threads)
    }

    /// Sets the size of the high priority thread pool that can be used to
    /// prevent compactions from stalling memtable flushes.
    pub fn set_high_priority_background_threads(&mut self, n: c_int) {
        self.0.set_high_priority_background_threads(n)
    }

    /// Sets the size of the low priority thread pool that can be used to
    /// prevent compactions from stalling memtable flushes.
    pub fn set_low_priority_background_threads(&mut self, n: c_int) {
        self.0.set_low_priority_background_threads(n)
    }

    /// Sets the size of the bottom priority thread pool that can be used to
    /// prevent compactions from stalling memtable flushes.
    pub fn set_bottom_priority_background_threads(&mut self, n: c_int) {
        self.0.set_bottom_priority_background_threads(n)
    }

    /// Wait for all threads started by StartThread to terminate.
    pub fn join_all_threads(&mut self) {
        self.0.join_all_threads()
    }

    /// Lowering IO priority for threads from the specified pool.
    pub fn lower_thread_pool_io_priority(&mut self) {
        self.0.lower_thread_pool_io_priority()
    }

    /// Lowering IO priority for high priority thread pool.
    pub fn lower_high_priority_thread_pool_io_priority(&mut self) {
        self.0.lower_high_priority_thread_pool_io_priority()
    }

    /// Lowering CPU priority for threads from the specified pool.
    pub fn lower_thread_pool_cpu_priority(&mut self) {
        self.0.lower_thread_pool_cpu_priority()
    }

    /// Lowering CPU priority for high priority thread pool.
    pub fn lower_high_priority_thread_pool_cpu_priority(&mut self) {
        self.0.lower_high_priority_thread_pool_cpu_priority()
    }
}

#[pymethods]
impl UniversalCompactOptionsPy {
    #[new]
    pub fn default() -> Self {
        UniversalCompactOptionsPy {
            size_ratio: 1,
            min_merge_width: 2,
            max_merge_width: c_int::MAX,
            max_size_amplification_percent: 200,
            compression_size_percent: -1,
            stop_style: UniversalCompactionStopStylePy(UniversalCompactionStopStyle::Total),
        }
    }

    /// Sets the percentage flexibility while comparing file size.
    /// If the candidate file(s) size is 1% smaller than the next file's size,
    /// then include next file into this candidate set.
    ///
    /// Default: 1
    pub fn set_size_ratio(&mut self, ratio: c_int) {
        self.size_ratio = ratio
    }

    /// Sets the minimum number of files in a single compaction run.
    ///
    /// Default: 2
    pub fn set_min_merge_width(&mut self, num: c_int) {
        self.min_merge_width = num
    }

    /// Sets the maximum number of files in a single compaction run.
    ///
    /// Default: UINT_MAX
    pub fn set_max_merge_width(&mut self, num: c_int) {
        self.max_merge_width = num
    }

    /// sets the size amplification.
    ///
    /// It is defined as the amount (in percentage) of
    /// additional storage needed to store a single byte of data in the database.
    /// For example, a size amplification of 2% means that a database that
    /// contains 100 bytes of user-data may occupy upto 102 bytes of
    /// physical storage. By this definition, a fully compacted database has
    /// a size amplification of 0%. Rocksdb uses the following heuristic
    /// to calculate size amplification: it assumes that all files excluding
    /// the earliest file contribute to the size amplification.
    ///
    /// Default: 200, which means that a 100 byte database could require upto 300 bytes of storage.
    pub fn set_max_size_amplification_percent(&mut self, v: c_int) {
        self.max_size_amplification_percent = v
    }

    /// Sets the percentage of compression size.
    ///
    /// If this option is set to be -1, all the output files
    /// will follow compression type specified.
    ///
    /// If this option is not negative, we will try to make sure compressed
    /// size is just above this value. In normal cases, at least this percentage
    /// of data will be compressed.
    /// When we are compacting to a new file, here is the criteria whether
    /// it needs to be compressed: assuming here are the list of files sorted
    /// by generation time:
    ///    A1...An B1...Bm C1...Ct
    /// where A1 is the newest and Ct is the oldest, and we are going to compact
    /// B1...Bm, we calculate the total size of all the files as total_size, as
    /// well as  the total size of C1...Ct as total_C, the compaction output file
    /// will be compressed iff
    ///   total_C / total_size < this percentage
    ///
    /// Default: -1
    pub fn set_compression_size_percent(&mut self, v: c_int) {
        self.compression_size_percent = v
    }

    /// Sets the algorithm used to stop picking files into a single compaction run.
    ///
    /// Default: ::Total
    pub fn set_stop_style(&mut self, style: PyRef<UniversalCompactionStopStylePy>) {
        self.stop_style = *style.deref()
    }
}

impl From<&UniversalCompactOptionsPy> for UniversalCompactOptions {
    fn from(u_opt: &UniversalCompactOptionsPy) -> Self {
        let mut uni = UniversalCompactOptions::default();
        uni.set_size_ratio(u_opt.size_ratio);
        uni.set_min_merge_width(u_opt.min_merge_width);
        uni.set_max_merge_width(u_opt.max_merge_width);
        uni.set_max_size_amplification_percent(u_opt.max_size_amplification_percent);
        uni.set_compression_size_percent(u_opt.compression_size_percent);
        uni.set_stop_style(u_opt.stop_style.0);
        uni
    }
}

#[pymethods]
impl UniversalCompactionStopStylePy {
    #[staticmethod]
    pub fn similar() -> Self {
        UniversalCompactionStopStylePy(UniversalCompactionStopStyle::Similar)
    }

    #[staticmethod]
    pub fn total() -> Self {
        UniversalCompactionStopStylePy(UniversalCompactionStopStyle::Total)
    }
}

#[pymethods]
impl FifoCompactOptionsPy {
    /// Sets the max table file size.
    ///
    /// Once the total sum of table files reaches this, we will delete the oldest
    /// table file
    ///
    /// Default: 1GB
    pub fn set_max_table_files_size(&mut self, nbytes: u64) {
        self.set_max_table_files_size = nbytes
    }
}

impl From<&FifoCompactOptionsPy> for FifoCompactOptions {
    fn from(f_opt: &FifoCompactOptionsPy) -> Self {
        let mut opt = FifoCompactOptions::default();
        opt.set_max_table_files_size(f_opt.set_max_table_files_size);
        opt
    }
}

#[macro_export]
macro_rules! implement_max_len_transform {
    ($($len:literal),*) => {
        fn create_max_len_transform(len: usize) -> Result<SliceTransform, ()> {
            match len {
                $($len => Ok(SliceTransform::create(
                    "max_len",
                    |slice| {
                        if slice.len() > $len {
                            &slice[0..$len]
                        } else {
                            slice
                        }
                    },
                    None,
                ))),*,
                _ => {
                    Err(())
                }
            }
        }
    };
}

implement_max_len_transform!(
    1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24, 25, 26,
    27, 28, 29, 30, 31, 32, 33, 34, 35, 36, 37, 38, 39, 40, 41, 42, 43, 44, 45, 46, 47, 48, 49, 50,
    51, 52, 53, 54, 55, 56, 57, 58, 59, 60, 61, 62, 63, 64, 65, 66, 67, 68, 69, 70, 71, 72, 73, 74,
    75, 76, 77, 78, 79, 80, 81, 82, 83, 84, 85, 86, 87, 88, 89, 90, 91, 92, 93, 94, 95, 96, 97, 98,
    99, 100, 101, 102, 103, 104, 105, 106, 107, 108, 109, 110, 111, 112, 113, 114, 115, 116, 117,
    118, 119, 120, 121, 122, 123, 124, 125, 126, 127, 128
);
