use std::sync::Arc;

use datafusion::arrow::array::{Array, ArrayRef};
use datafusion::arrow::datatypes::DataType;
use datafusion::common::ScalarValue;
use datafusion::error::DataFusionError;
use datafusion::logical_expr::{ReturnTypeFunction, ScalarUDF, Signature, Volatility};
use datafusion::physical_expr::functions::make_scalar_function;
use spi::query::function::FunctionMetadataManager;
use spi::Result;

use crate::extension::expr::aggregate_function::StateAggData;

pub const DURATION_IN: &str = "duration_in";

pub fn register_udf(func_manager: &mut dyn FunctionMetadataManager) -> Result<ScalarUDF> {
    let udf = new();
    func_manager.register_udf(udf.clone())?;
    Ok(udf)
}

fn new() -> ScalarUDF {
    let return_type_fn: ReturnTypeFunction = Arc::new(|input| {
        let error = || DataFusionError::Execution("Get duration_in ReturnTypeFuction error".into());
        match &input[0] {
            DataType::Struct(f) => {
                let a = f.find("state_duration").ok_or_else(error)?.1;
                match a.data_type() {
                    DataType::List(f) => match f.data_type() {
                        DataType::Struct(f) => Ok(f
                            .find("duration")
                            .ok_or_else(error)?
                            .1
                            .data_type()
                            .clone()
                            .into()),
                        _ => Err(error()),
                    },
                    _ => Err(error()),
                }
            }
            _ => Err(error()),
        }
    });

    let duration = make_scalar_function(duration_in_implement);

    // TODO: support state_agg
    // let signature = vec![
    //     TypeSignature::Any(2),
    //     TypeSignature::Any(3),
    //     TypeSignature::Any(4),
    // ];

    ScalarUDF::new(
        DURATION_IN,
        &Signature::any(2, Volatility::Immutable),
        &return_type_fn,
        &duration,
    )
}

fn duration_in_implement(input: &[ArrayRef]) -> Result<ArrayRef, DataFusionError> {
    let array_len = input[0].len();
    let mut res = Vec::with_capacity(array_len);
    match input.len() {
        // duration_in(state_agg, state)
        2 => {
            for i in 0..array_len {
                let state_agg = ScalarValue::try_from_array(input[0].as_ref(), i)?;
                let state = ScalarValue::try_from_array(input[1].as_ref(), i)?;
                let state_agg = StateAggData::try_from(state_agg)?;
                let value = state_agg.duration_in(state, ScalarValue::Null, ScalarValue::Null)?;
                res.push(value)
            }
        }
        // duration_in(state_agg, state, start_time)
        3 => {
            return Err(DataFusionError::NotImplemented(
                "duration in only support 2 arguments".into(),
            ));
            // for i in 0..array_len {
            //     let state_agg = ScalarValue::try_from_array(input[0].as_ref(), i)?;
            //     let state = ScalarValue::try_from_array(input[1].as_ref(), i)?;
            //     let start = ScalarValue::try_from_array(input[2].as_ref(), i)?;
            //     let state_agg = StateAggData::try_from(state_agg)?;
            //     let value = state_agg.duration_in(state, start, ScalarValue::Null)?;
            //     res.push(value)
            // }
        }
        // duration_in(state_agg, state, start_time, interval)
        4 => {
            return Err(DataFusionError::NotImplemented(
                "duration in only support 2 arguments".into(),
            ));
            // for i in 0..array_len {
            //     let state_agg = ScalarValue::try_from_array(input[0].as_ref(), i)?;
            //     let state = ScalarValue::try_from_array(input[1].as_ref(), i)?;
            //     let start = ScalarValue::try_from_array(input[2].as_ref(), i)?;
            //     let interval = ScalarValue::try_from_array(input[3].as_ref(), i)?;
            //     let state_agg = StateAggData::try_from(state_agg)?;
            //     let value = state_agg.duration_in(state, start, interval)?;
            //     res.push(value)
            // }
        }
        _ => {
            return Err(DataFusionError::NotImplemented(
                "duration in only support 2 arguments".into(),
            ));
        }
    }
    let array = ScalarValue::iter_to_array(res)?;
    Ok(array)
}
