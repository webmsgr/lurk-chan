use serde::{Serialize, de::DeserializeOwned};
/// stupid idiot function to convert serializable to serializable. 
/// Useful for hashmap -> object conversions (how its used in LC)
pub fn transmute_json<I: Serialize, D: DeserializeOwned>(from: I) -> Result<D, serde_json::Error> {
    Ok(serde_json::to_value(from).and_then(serde_json::from_value::<D>)?)
}