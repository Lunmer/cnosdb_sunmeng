use std::sync::Arc;

use arrow::array::{Float32Array, Float64Array, StringArray, UInt64Array};
use arrow::datatypes::{DataType, Field, Schema, SchemaRef};
use arrow::record_batch::RecordBatch;
use datafusion::arrow;
use datafusion::datasource::MemTable;
use datafusion::error::Result;
use datafusion::execution::context::SessionContext;
use parking_lot::Mutex;
use query::extension::expr::func_manager::DFSessionContextFuncAdapter;
use query::extension::expr::load_all_functions;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;
use rand::{Rng, SeedableRng};
use tokio::runtime::Runtime;

pub fn query(ctx: Arc<Mutex<SessionContext>>, sql: &str) {
    let rt = Runtime::new().unwrap();
    let df = rt.block_on(ctx.lock().sql(sql)).unwrap();
    criterion::black_box(rt.block_on(df.collect()).unwrap());
}

pub fn create_context(
    partitions_len: usize,
    array_len: usize,
    batch_size: usize,
) -> Result<Arc<Mutex<SessionContext>>> {
    let mut ctx = SessionContext::new();
    // temporary(database level): wrap SessionContext into function meta manager
    let mut func_manager = DFSessionContextFuncAdapter::new(&mut ctx);
    // temporary(database level): register function to function meta manager
    load_all_functions(&mut func_manager).expect("load_all_functions");
    let provider = create_table_provider(partitions_len, array_len, batch_size)?;
    ctx.register_table("t", provider)?;
    Ok(Arc::new(Mutex::new(ctx)))
}

/// create an in-memory table given the partition len, array len, and batch size,
/// and the result table will be of array_len in total, and then partitioned, and batched.
#[allow(dead_code)]
pub fn create_table_provider(
    partitions_len: usize,
    array_len: usize,
    batch_size: usize,
) -> Result<Arc<MemTable>> {
    let schema = Arc::new(create_schema());
    let partitions = create_record_batches(schema.clone(), array_len, partitions_len, batch_size);
    // declare a table in memory. In spark API, this corresponds to createDataFrame(...).
    MemTable::try_new(schema, partitions).map(Arc::new)
}

/// create a seedable [`StdRng`](rand::StdRng)
fn seedable_rng() -> StdRng {
    StdRng::seed_from_u64(42)
}

/// Create test data schema
pub fn create_schema() -> Schema {
    Schema::new(vec![
        Field::new("utf8", DataType::Utf8, true),
        Field::new("f32", DataType::Float32, true),
        Field::new("f64", DataType::Float64, true),
        // This field will contain integers randomly selected from a large
        // range of values, i.e. [0, u64::MAX], such that there are none (or
        // very few) repeated values.
        Field::new("u64_wide", DataType::UInt64, true),
        // This field will contain integers randomly selected from a narrow
        // range of values such that there are a few distinct values, but they
        // are repeated often.
        Field::new("u64_narrow", DataType::UInt64, true),
    ])
}

fn create_data(size: usize, null_density: f64) -> Vec<Option<f64>> {
    // use random numbers to avoid spurious compiler optimizations wrt to branching
    let mut rng = seedable_rng();

    (0..size)
        .map(|_| {
            if rng.gen::<f64>() > null_density {
                None
            } else {
                Some(rng.gen::<f64>())
            }
        })
        .collect()
}

fn create_integer_data(size: usize, value_density: f64) -> Vec<Option<u64>> {
    // use random numbers to avoid spurious compiler optimizations wrt to branching
    let mut rng = seedable_rng();

    (0..size)
        .map(|_| {
            if rng.gen::<f64>() > value_density {
                None
            } else {
                Some(rng.gen::<u64>())
            }
        })
        .collect()
}

fn create_record_batch(
    schema: SchemaRef,
    rng: &mut StdRng,
    batch_size: usize,
    i: usize,
) -> RecordBatch {
    // the 4 here is the number of different keys.
    // a higher number increase sparseness
    let vs = vec![0, 1, 2, 3];
    let keys: Vec<String> = (0..batch_size)
        .map(
            // use random numbers to avoid spurious compiler optimizations wrt to branching
            |_| format!("hi{:?}", vs.choose(rng)),
        )
        .collect();
    let keys: Vec<&str> = keys.iter().map(|e| &**e).collect();
    let values = create_data(batch_size, 0.5);

    // Integer values between [0, u64::MAX].
    let integer_values_wide = create_integer_data(batch_size, 9.0);

    // Integer values between [0, 9].
    let integer_values_narrow = (0..batch_size)
        .map(|_| rng.gen_range(0_u64..10))
        .collect::<Vec<_>>();

    RecordBatch::try_new(
        schema,
        vec![
            Arc::new(StringArray::from(keys)),
            Arc::new(Float32Array::from(vec![i as f32; batch_size])),
            Arc::new(Float64Array::from(values)),
            Arc::new(UInt64Array::from(integer_values_wide)),
            Arc::new(UInt64Array::from(integer_values_narrow)),
        ],
    )
    .unwrap()
}

/// Create record batches of `partitions_len` partitions and `batch_size` for each batch,
/// with a total number of `array_len` records
pub fn create_record_batches(
    schema: SchemaRef,
    array_len: usize,
    partitions_len: usize,
    batch_size: usize,
) -> Vec<Vec<RecordBatch>> {
    let mut rng = seedable_rng();
    (0..partitions_len)
        .map(|_| {
            (0..array_len / batch_size / partitions_len)
                .map(|i| create_record_batch(schema.clone(), &mut rng, batch_size, i))
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>()
}
