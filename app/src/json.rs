// use serde_json::{json, Map, Value};
//
// use crate::Error;
//
// use super::Result;
//
// #[derive(Debug, Clone, PartialEq)]
// pub enum JsonType {
//     Null,
//     Bool,
//     Number,
//     String,
//     Array,
//     Object,
//     // Array(Box<JsonType>),
//     // Object(Box<JsonType>, Box<JsonType>), // Key type, Value type
// }
//
// pub fn get_object(value: Value, field: &str) -> Result<Map<String, Value>> {
//     match value {
//         Value::Object(obj) => Ok(obj),
//         _ => Err(Error::TypeError {
//             field: field.to_string(),
//             expected: "object".to_string(),
//         }),
//     }
// }
//
// pub fn expect_keys(value: Value, field: &str, expected_keys: &[&str]) -> Result<Value> {
//     match value {
//         Value::Object(obj) => {
//             for key in expected_keys {
//                 if !obj.contains_key(*key) {
//                     let full_path = format!("{}.{}", field, key);
//                     return Err(Error::MissingField(full_path));
//                 }
//             }
//             Ok(obj)
//         }
//         _ => Err(Error::TypeError {
//             field: field.to_string(),
//             expected: "object".to_string(),
//         }),
//     }
// }
//
// /// Checks if a JSON value matches the expected JsonType.
// /// Returns the value if it matches, otherwise returns an error.
// pub fn expect_type(value: &Value, field: &str, expected_type: JsonType) -> Result<()> {
//     match (&expected_type, value) {
//         (JsonType::Null, Value::Null) => Ok(()),
//         (JsonType::Bool, Value::Bool(_)) => Ok(()),
//         (JsonType::Number, Value::Number(_)) => Ok(()),
//         (JsonType::String, Value::String(_)) => Ok(()),
//         (JsonType::Array, Value::Array(_)) => Ok(()),
//         (JsonType::Object, Value::Object(_)) => Ok(()),
//
//         // (JsonType::Array(elem_type), Value::Array(arr)) => {
//         //     // Check that all elements in the array match the expected element type
//         //     for elem in arr {
//         //         expect_type(elem, *elem_type.clone())?;
//         //     }
//         //     Ok(())
//         // }
//         //
//         // (JsonType::Object(key_type, val_type), Value::Object(obj)) => {
//         //     // For objects, we just check the value types against the expected value type
//         //     // (since JSON keys are always strings, we could relax this check)
//         //     for (_, val) in obj {
//         //         expect_type(val, *val_type.clone())?;
//         //     }
//         //     Ok(())
//         // }
//         _ => Err(Error::TypeError {
//             field: field.to_string(),
//             expected: format!("{:?}", expected_type),
//         }),
//     }
// }
//
// // Helper function to extract values once the type is verified
// pub fn extract_as<T>(
//     value: &Value,
//     expected_type: JsonType,
//     extractor: fn(&Value) -> T,
// ) -> Result<T> {
//     expect_type(value, expected_type)?;
//     Ok(extractor(value))
// }
