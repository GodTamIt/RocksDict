use crate::encoder::{decode_value, encode_key, encode_value};
use crate::iter::{RdictItems, RdictKeys, RdictValues};
use crate::{
    FlushOptionsPy, IngestExternalFileOptionsPy, OptionsPy, RdictIter, ReadOptionsPy,
    WriteOptionsPy,
};
use pyo3::exceptions::PyException;
use pyo3::prelude::*;
use pyo3::types::PyList;
use rocksdb::{
    ColumnFamily, ColumnFamilyDescriptor, FlushOptions, ReadOptions, WriteOptions, DB,
};
use std::cell::RefCell;
use std::collections::HashMap;
use std::fs::create_dir_all;
use std::ops::Deref;
use std::path::Path;
use std::rc::Rc;
use std::time::Duration;

///
/// A persistent on-disk dictionary. Supports string, int, float, bytes as key, values.
///
/// Example:
///     ::
///
///         from rocksdict import Rdict
///
///         db = Rdict("./test_dir")
///         db[0] = 1
///
///         db = None
///         db = Rdict("./test_dir")
///         assert(db[0] == 1)
///
#[pyclass(name = "Rdict")]
#[pyo3(text_signature = "(path, options, read_only, ttl)")]
pub(crate) struct Rdict {
    db: Option<Rc<RefCell<DB>>>,
    write_opt: WriteOptions,
    flush_opt: FlushOptionsPy,
    read_opt: ReadOptions,
    pickle_loads: PyObject,
    pickle_dumps: PyObject,
    write_opt_py: WriteOptionsPy,
    read_opt_py: ReadOptionsPy,
    column_family: Option<Rc<ColumnFamily>>,
}

#[pymethods]
impl Rdict {
    /// Create a new database or open an existing one.
    ///
    /// Args:
    ///     path (str): path to the database
    ///     options (Options): Options object
    ///     column_families (dict): (name, options) pairs
    ///     read_only (bool): whether to open read_only
    ///     error_if_log_file_exist (bool): this option is useful only when
    ///         read_only is set to `true`
    ///     ttl (int): TTL option in seconds.
    #[new]
    #[args(
        options = "Py::new(_py, OptionsPy::new())?",
        column_families = "None",
        read_only = "false",
        error_if_log_file_exist = "true",
        ttl = "0"
    )]
    fn new(
        path: &str,
        options: Py<OptionsPy>,
        column_families: Option<HashMap<String, OptionsPy>>,
        read_only: bool,
        error_if_log_file_exist: bool,
        ttl: u64,
        py: Python,
    ) -> PyResult<Self> {
        let path = Path::new(path);
        let pickle = PyModule::import(py, "pickle")?.to_object(py);
        let options = &options.borrow(py).0;
        match create_dir_all(path) {
            Ok(_) => match {
                match (read_only, ttl, column_families.is_some()) {
                    (false, 0, true) => DB::open_cf_descriptors(
                        options,
                        path,
                        column_families
                            .unwrap()
                            .into_iter()
                            .map(|(name, opt)| ColumnFamilyDescriptor::new(name, opt.0)),
                    ),
                    (false, ttl, true) => DB::open_cf_descriptors_with_ttl(
                        options,
                        path,
                        column_families
                            .unwrap()
                            .into_iter()
                            .map(|(name, opt)| ColumnFamilyDescriptor::new(name, opt.0)),
                        Duration::from_secs(ttl),
                    ),
                    (true, _, true) => DB::open_cf_descriptors_for_read_only(
                        options,
                        path,
                        column_families
                            .unwrap()
                            .into_iter()
                            .map(|(name, opt)| ColumnFamilyDescriptor::new(name, opt.0)),
                        error_if_log_file_exist,
                    ),
                    (false, 0, _) => DB::open(options, &path),
                    (false, ttl, _) => DB::open_with_ttl(options, path, Duration::from_secs(ttl)),
                    (true, _, _) => DB::open_for_read_only(options, &path, error_if_log_file_exist),
                }
            } {
                Ok(db) => {
                    let r_opt = ReadOptionsPy::default(py)?;
                    let w_opt = WriteOptionsPy::new();
                    Ok(Rdict {
                        db: Some(Rc::new(RefCell::new(db))),
                        write_opt: (&w_opt).into(),
                        flush_opt: FlushOptionsPy::new(),
                        read_opt: (&r_opt).into(),
                        pickle_loads: pickle.getattr(py, "loads")?,
                        pickle_dumps: pickle.getattr(py, "dumps")?,
                        write_opt_py: w_opt,
                        read_opt_py: r_opt,
                        column_family: None,
                    })
                }
                Err(e) => Err(PyException::new_err(e.to_string())),
            },
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    /// Optionally disable WAL or sync for this write.
    ///
    /// Example:
    ///     ::
    ///
    ///         from rocksdict import Rdict, Options, WriteBatch, WriteOptions
    ///
    ///         path = "_path_for_rocksdb_storageY1"
    ///         db = Rdict(path)
    ///
    ///         # set write options
    ///         write_options = WriteOptions()
    ///         write_options.set_sync(False)
    ///         write_options.disable_wal(True)
    ///         db.set_write_options(write_options)
    ///
    ///         # write to db
    ///         db["my key"] = "my value"
    ///         db["key2"] = "value2"
    ///         db["key3"] = "value3"
    ///
    ///         # remove db
    ///         del db
    ///         Rdict.destroy(path)
    #[pyo3(text_signature = "($self, write_opt)")]
    fn set_write_options(&mut self, write_opt: &WriteOptionsPy) {
        self.write_opt = write_opt.into();
        self.write_opt_py = write_opt.clone();
    }

    /// Configure Read Options for all the get operations.
    #[pyo3(text_signature = "($self, read_opt)")]
    fn set_read_options(&mut self, read_opt: &ReadOptionsPy) {
        self.read_opt = read_opt.into();
        self.read_opt_py = read_opt.clone();
    }

    /// Parse list for batch get.
    fn __getitem__(&self, key: &PyAny, py: Python) -> PyResult<PyObject> {
        if let Some(db) = &self.db {
            // batch_get
            if let Ok(keys) = PyTryFrom::try_from(key) {
                return Ok(
                    get_batch_inner(db, keys, py, &self.read_opt, &self.pickle_loads, &self.column_family)?
                        .to_object(py),
                );
            }
            let key = encode_key(key)?;
            let db = db.borrow();
            let value_result = if let Some(cf) = &self.column_family {
                db.get_pinned_cf_opt(cf.deref(), &key[..], &self.read_opt)
            } else {
                db.get_pinned_opt(&key[..], &self.read_opt)
            };
            match value_result {
                Ok(value) => match value {
                    None => Err(PyException::new_err("key not found")),
                    Some(slice) => decode_value(py, slice.as_ref(), &self.pickle_loads),
                },
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    fn __setitem__(&self, key: &PyAny, value: &PyAny, py: Python) -> PyResult<()> {
        if let Some(db) = &self.db {
            let key = encode_key(key)?;
            let value = encode_value(value, &self.pickle_dumps, py)?;
            let db = db.borrow();
            let put_result = if let Some(cf) = &self.column_family {
                db.put_cf_opt(cf.deref(), &key[..], value, &self.write_opt)
            } else {
                db.put_opt(&key[..], value, &self.write_opt)
            };
            match put_result {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    fn __contains__(&self, key: &PyAny) -> PyResult<bool> {
        if let Some(db) = &self.db {
            let key = encode_key(key)?;
            let db = db.borrow();
            let may_exist = if let Some(cf) = &self.column_family {
                db.key_may_exist_cf_opt(cf.deref(), &key[..], &self.read_opt)
            } else {
                db.key_may_exist_opt(&key[..], &self.read_opt)
            };
            if may_exist {
                let value_result = if let Some(cf) = &self.column_family {
                    db.get_pinned_cf_opt(cf.deref(), &key[..], &self.read_opt)
                } else {
                    db.get_pinned_opt(&key[..], &self.read_opt)
                };
                match value_result {
                    Ok(value) => match value {
                        None => Ok(false),
                        Some(_) => Ok(true),
                    },
                    Err(e) => Err(PyException::new_err(e.to_string())),
                }
            } else {
                Ok(false)
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    fn __delitem__(&self, key: &PyAny) -> PyResult<()> {
        if let Some(db) = &self.db {
            let key = encode_key(key)?;
            let db = db.borrow();
            let del_result = if let Some(cf) = &self.column_family {
                db.delete_cf_opt(cf.deref(), &key[..], &self.write_opt)
            } else {
                db.delete_opt(&key[..], &self.write_opt)
            };
            match del_result {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Reversible for iterating over keys and values.
    ///
    /// Examples:
    ///     ::
    ///
    ///         from rocksdict import Rdict, Options, ReadOptions
    ///
    ///         path = "_path_for_rocksdb_storage5"
    ///         db = Rdict(path)
    ///
    ///         for i in range(50):
    ///             db[i] = i ** 2
    ///
    ///         iter = db.iter()
    ///
    ///         iter.seek_to_first()
    ///
    ///         j = 0
    ///         while iter.valid():
    ///             assert iter.key() == j
    ///             assert iter.value() == j ** 2
    ///             print(f"{iter.key()} {iter.value()}")
    ///             iter.next()
    ///             j += 1
    ///
    ///         iter.seek_to_first();
    ///         assert iter.key() == 0
    ///         assert iter.value() == 0
    ///         print(f"{iter.key()} {iter.value()}")
    ///
    ///         iter.seek(25)
    ///         assert iter.key() == 25
    ///         assert iter.value() == 625
    ///         print(f"{iter.key()} {iter.value()}")
    ///
    ///         del iter, db
    ///         Rdict.destroy(path)
    ///
    /// Args:
    ///     read_opt: ReadOptions
    ///
    /// Returns: Reversible
    #[pyo3(text_signature = "($self, read_opt)")]
    #[args(read_opt = "Py::new(_py, ReadOptionsPy::default(_py)?)?")]
    fn iter(&self, read_opt: Py<ReadOptionsPy>, py: Python) -> PyResult<RdictIter> {
        if let Some(db) = &self.db {
            Ok(RdictIter::new(
                db,
                &self.column_family,
                read_opt.borrow(py).deref().into(),
                &self.pickle_loads,
            ))
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Iterate through all keys and values pairs.
    ///
    /// Examples:
    ///     ::
    ///
    ///         for k, v in db.items():
    ///             print(f"{k} -> {v}")
    ///
    /// Args:
    ///     inner: the inner Rdict
    ///     backwards: iteration direction, forward if `False`.
    ///     from_key: iterate from key, first seek to this key
    ///         or the nearest next key for iteration
    ///         (depending on iteration direction).
    #[pyo3(text_signature = "($self, backwards, from_key, read_opt)")]
    #[args(
        backwards = "false",
        from_key = "_py.None().into_ref(_py)",
        read_opt = "Py::new(_py, ReadOptionsPy::default(_py)?)?"
    )]
    fn items(
        &self,
        backwards: bool,
        from_key: &PyAny,
        read_opt: Py<ReadOptionsPy>,
        py: Python,
    ) -> PyResult<RdictItems> {
        Ok(RdictItems::new(
            self.iter(read_opt, py)?,
            backwards,
            from_key,
        )?)
    }

    /// Iterate through all keys.
    ///
    /// Examples:
    ///     ::
    ///
    ///         all_keys = [k for k in db.keys()]
    ///
    /// Args:
    ///     inner: the inner Rdict
    ///     backwards: iteration direction, forward if `False`.
    ///     from_key: iterate from key, first seek to this key
    ///         or the nearest next key for iteration
    ///         (depending on iteration direction).
    #[pyo3(text_signature = "($self, backwards, from_key, read_opt)")]
    #[args(
        backwards = "false",
        from_key = "_py.None().into_ref(_py)",
        read_opt = "Py::new(_py, ReadOptionsPy::default(_py)?)?"
    )]
    fn keys(
        &self,
        backwards: bool,
        from_key: &PyAny,
        read_opt: Py<ReadOptionsPy>,
        py: Python,
    ) -> PyResult<RdictKeys> {
        Ok(RdictKeys::new(
            self.iter(read_opt, py)?,
            backwards,
            from_key,
        )?)
    }

    /// Iterate through all values.
    ///
    /// Examples:
    ///     ::
    ///
    ///         all_keys = [v for v in db.values()]
    ///
    /// Args:
    ///     inner: the inner Rdict
    ///     backwards: iteration direction, forward if `False`.
    ///     from_key: iterate from key, first seek to this key
    ///         or the nearest next key for iteration
    ///         (depending on iteration direction).
    #[pyo3(text_signature = "($self, backwards, from_key, read_opt)")]
    #[args(
        backwards = "false",
        from_key = "_py.None().into_ref(_py)",
        read_opt = "Py::new(_py, ReadOptionsPy::default(_py)?)?"
    )]
    fn values(
        &self,
        backwards: bool,
        from_key: &PyAny,
        read_opt: Py<ReadOptionsPy>,
        py: Python,
    ) -> PyResult<RdictValues> {
        Ok(RdictValues::new(
            self.iter(read_opt, py)?,
            backwards,
            from_key,
        )?)
    }

    /// Manually flush all memory to disk.
    ///
    /// Notes:
    ///     Manually call mem-table flush.
    ///     It is recommended to call flush() or close() before
    ///     stopping the python program, to ensure that all written
    ///     key-value pairs have been flushed to the disk.
    ///
    /// Args:
    ///     wait (bool): whether to wait for the flush to finish.
    #[pyo3(text_signature = "($self, wait)")]
    #[args(wait = "true")]
    fn flush(&self, wait: bool) -> PyResult<()> {
        if let Some(db) = &self.db {
            let mut f_opt = FlushOptions::new();
            f_opt.set_wait(wait);
            let db = db.borrow();
            let flush_result = if let Some(cf) = &self.column_family {
                db.flush_cf_opt(cf.deref(), &f_opt)
            } else {
                db.flush_opt(&f_opt)
            };
            match flush_result {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.into_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Creates column family with given name and options.
    ///
    /// Args:
    ///     name: name of this column family
    ///     options: Rdict Options for this column family
    ///
    /// Return:
    ///     the newly created column family
    #[pyo3(text_signature = "($self, name, options)")]
    #[args(options = "Py::new(_py, OptionsPy::new())?")]
    fn create_column_family(
        &self,
        name: &str,
        options: Py<OptionsPy>,
        py: Python,
    ) -> PyResult<Rdict> {
        if let Some(db) = &self.db {
            let create_result = db.borrow_mut().create_cf(name, &options.borrow(py).0);
            match create_result {
                Ok(_) => Ok(self.get_column_family(name)?),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Drops the column family with the given name
    #[pyo3(text_signature = "($self, name)")]
    fn drop_column_family(&self, name: &str) -> PyResult<()> {
        if let Some(db) = &self.db {
            match db.borrow_mut().drop_cf(name) {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Get a column family Rdict
    ///
    /// Args:
    ///     name: name of this column family
    ///     options: Rdict Options for this column family
    ///
    /// Return:
    ///     the column family Rdict of this name
    #[pyo3(text_signature = "($self, name)")]
    pub fn get_column_family(&self, name: &str) -> PyResult<Self> {
        if let Some(db) = &self.db {
            match db.borrow().cf_handle(name) {
                None => Err(PyException::new_err(format!(
                    "column name `{}` does not exist, use `create_cf` to creat it",
                    name
                ))),
                Some(cf) => Ok(Self {
                    db: Some(db.clone()),
                    write_opt: (&self.write_opt_py).into(),
                    flush_opt: self.flush_opt.clone(),
                    read_opt: (&self.read_opt_py).into(),
                    pickle_loads: self.pickle_loads.clone(),
                    pickle_dumps: self.pickle_dumps.clone(),
                    column_family: Some(cf.clone()),
                    write_opt_py: self.write_opt_py.clone(),
                    read_opt_py: self.read_opt_py.clone(),
                }),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Loads a list of external SST files created with SstFileWriter into the DB
    ///
    /// Args:
    ///     paths: a list a paths
    ///     opts: IngestExternalFileOptionsPy instance
    #[pyo3(text_signature = "($self, paths, opts)")]
    #[args(opts = "Py::new(_py, IngestExternalFileOptionsPy::new())?")]
    fn ingest_external_file(
        &self,
        paths: Vec<String>,
        opts: Py<IngestExternalFileOptionsPy>,
        py: Python,
    ) -> PyResult<()> {
        if let Some(db) = &self.db {
            let db = db.borrow();
            let ingest_result = if let Some(cf) = &self.column_family {
                db.ingest_external_file_cf_opts(cf.deref(), &opts.borrow(py).0, paths)
            } else {
                db.ingest_external_file_opts(&opts.borrow(py).0, paths)
            };
            match ingest_result {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Flush memory to disk, and drop the database.
    ///
    /// Notes:
    ///     Setting Rdict to `None` does not always immediately close
    ///     the database depending on the garbage collector of python.
    ///     Calling `close()` is a more reliable method to ensure
    ///     that the database is correctly closed.
    ///
    ///     The database would not be usable after `close()` is called.
    ///     Calling method after `close()` will throw exception.
    #[pyo3(text_signature = "($self)")]
    fn close(&mut self) -> PyResult<()> {
        if let Some(db) = &self.db {
            let f_opt = &self.flush_opt;
            let db = db.borrow();
            let flush_result = if let Some(cf) = &self.column_family {
                db.flush_cf_opt(cf.deref(), &f_opt.into())
            } else {
                db.flush_opt(&f_opt.into())
            };
            drop(db);
            drop(self.column_family.take());
            drop(self.db.take());
            match flush_result {
                Ok(_) => Ok(()),
                Err(e) => Err(PyException::new_err(e.to_string())),
            }
        } else {
            Err(PyException::new_err("DB already closed"))
        }
    }

    /// Delete the database.
    ///
    /// Args:
    ///     path (str): path to this database
    ///     options (rocksdict.Options): Rocksdb options object
    #[staticmethod]
    #[pyo3(text_signature = "(path, options)")]
    #[args(options = "Py::new(_py, OptionsPy::new())?")]
    fn destroy(path: &str, options: Py<OptionsPy>, py: Python) -> PyResult<()> {
        match DB::destroy(&options.borrow(py).0, path) {
            Ok(_) => Ok(()),
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }
}

#[inline(always)]
fn get_batch_inner<'a>(
    db: &RefCell<DB>,
    keys: &'a PyList,
    py: Python<'a>,
    read_opt: &ReadOptions,
    pickle_loads: &PyObject,
    column_family: &Option<Rc<ColumnFamily>>,
) -> PyResult<&'a PyList> {
    let mut keys_batch = Vec::new();
    for key in keys {
        keys_batch.push(encode_key(key)?);
    }
    let db = db.borrow();
    let values = if let Some(cf) = column_family {
        let keys_cols: Vec<(&ColumnFamily, Box<[u8]>)> =
            keys_batch.into_iter().map(|k| (cf.deref(), k)).collect();
        db.multi_get_cf_opt(keys_cols, read_opt)
    } else {
        db.multi_get_opt(keys_batch, read_opt)
    };
    let result = PyList::empty(py);
    for v in values {
        match v {
            Ok(value) => match value {
                None => result.append(py.None())?,
                Some(slice) => result.append(decode_value(py, slice.as_ref(), pickle_loads)?)?,
            },
            Err(e) => return Err(PyException::new_err(e.to_string())),
        }
    }
    Ok(result)
}

impl Drop for Rdict {
    // flush
    fn drop(&mut self) {
        if let Some(db) = &self.db {
            let f_opt = &self.flush_opt;
            let db = db.borrow();
            let _ = if let Some(cf) = &self.column_family {
                db.flush_cf_opt(cf.deref(), &f_opt.into())
            } else {
                db.flush_opt(&f_opt.into())
            };
        }
        // important, always drop column families first
        // to ensure that CF handles have shorter life than DB.
        drop(self.column_family.take());
        drop(self.db.take());
    }
}

unsafe impl Send for Rdict {}
