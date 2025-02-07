mod gauge_agg;

use std::sync::Arc;

use datafusion::arrow::datatypes::{DataType, Field, Fields};
use datafusion::common::Result as DFResult;
use datafusion::error::DataFusionError;
use datafusion::scalar::ScalarValue;
use spi::query::function::FunctionMetadataManager;
use spi::QueryError;

use super::{AggResult, TSPoint};

pub fn register_udafs(func_manager: &mut dyn FunctionMetadataManager) -> Result<(), QueryError> {
    gauge_agg::register_udaf(func_manager)?;
    Ok(())
}

#[derive(Debug, PartialEq)]
pub struct GaugeData {
    first: TSPoint,
    second: TSPoint,
    penultimate: TSPoint,
    last: TSPoint,
    num_elements: u64,
}

impl GaugeData {
    fn try_new_null(time_data_type: DataType, value_data_type: DataType) -> DFResult<Self> {
        let null = TSPoint::try_new_null(time_data_type, value_data_type)?;
        Ok(Self {
            first: null.clone(),
            second: null.clone(),
            penultimate: null.clone(),
            last: null,
            num_elements: 0,
        })
    }

    pub fn delta(&self) -> DFResult<ScalarValue> {
        match self.last.val().sub_checked(self.first.val()) {
            Ok(value) => Ok(value),
            Err(_) => {
                // null if overflow
                ScalarValue::try_from(self.last.val().get_datatype())
            }
        }
    }

    pub fn time_delta(&self) -> DFResult<ScalarValue> {
        match self.last.ts().sub_checked(self.first.ts()) {
            Ok(value) => Ok(value),
            Err(_) => {
                // null if overflow
                let zero = ScalarValue::new_zero(&self.last.ts().get_datatype())?;
                let interval_datatype = zero.sub(&zero)?.get_datatype();
                ScalarValue::try_from(interval_datatype)
            }
        }
    }
}

impl AggResult for GaugeData {
    fn to_scalar(self) -> DFResult<ScalarValue> {
        let Self {
            first,
            second,
            penultimate,
            last,
            num_elements,
            ..
        } = self;

        let first = first.to_scalar()?;
        let second = second.to_scalar()?;
        let penultimate = penultimate.to_scalar()?;
        let last = last.to_scalar()?;
        let num_elements = ScalarValue::from(num_elements);

        let first_data_type = first.get_datatype();
        let second_data_type = second.get_datatype();
        let penultimate_data_type = penultimate.get_datatype();
        let last_data_type = last.get_datatype();
        let num_elements_data_type = num_elements.get_datatype();

        Ok(ScalarValue::Struct(
            Some(vec![first, second, penultimate, last, num_elements]),
            Fields::from([
                Arc::new(Field::new("first", first_data_type, true)),
                Arc::new(Field::new("second", second_data_type, true)),
                Arc::new(Field::new("penultimate", penultimate_data_type, true)),
                Arc::new(Field::new("last", last_data_type, true)),
                Arc::new(Field::new("num_elements", num_elements_data_type, true)),
            ]),
        ))
    }
}

impl GaugeData {
    pub fn try_from_scalar(scalar: ScalarValue) -> DFResult<Self> {
        let valid_func = |fields: &Fields| {
            let field_names = ["first", "second", "penultimate", "last", "num_elements"];
            let input_fields = fields.iter().map(|f| f.name().as_str()).collect::<Vec<_>>();
            if !input_fields.eq(&field_names) {
                return Err(DataFusionError::External(Box::new(QueryError::Analyzer {
                    err: format!("Expected GaugeData, got {:?}", fields),
                })));
            }

            Ok(())
        };

        match scalar {
            ScalarValue::Struct(Some(values), fields) => {
                valid_func(&fields)?;

                let first = TSPoint::try_from_scalar(values[0].clone())?;
                let second = TSPoint::try_from_scalar(values[1].clone())?;
                let penultimate = TSPoint::try_from_scalar(values[2].clone())?;
                let last = TSPoint::try_from_scalar(values[3].clone())?;
                let num_elements: u64 = values[4].clone().try_into()?;

                Ok(Self {
                    first,
                    second,
                    penultimate,
                    last,
                    num_elements,
                })
            }
            ScalarValue::Struct(None, fields) => {
                valid_func(&fields)?;

                let first =
                    TSPoint::try_from_scalar(ScalarValue::try_from(fields[0].data_type())?)?;
                let second =
                    TSPoint::try_from_scalar(ScalarValue::try_from(fields[1].data_type())?)?;
                let penultimate =
                    TSPoint::try_from_scalar(ScalarValue::try_from(fields[2].data_type())?)?;
                let last = TSPoint::try_from_scalar(ScalarValue::try_from(fields[3].data_type())?)?;
                let num_elements: u64 = 0;

                Ok(Self {
                    first,
                    second,
                    penultimate,
                    last,
                    num_elements,
                })
            }
            _ => Err(DataFusionError::External(Box::new(QueryError::Analyzer {
                err: format!("Expected GaugeData, got {:?}", scalar),
            }))),
        }
    }
}
