use serde_json::Value;

use crate::exception::Exception;

#[derive(Clone, Debug)]
enum Either {
    Value(Value),
    Exception(Exception),
}

#[derive(Clone, Debug)]
pub struct TaskResult {
    either: Either,
}

impl TaskResult {
    pub fn as_value(&self) -> Option<&Value> {
        match &self.either {
            Either::Value(value) => Some(value),
            _ => None,
        }
    }

    pub fn as_exception(&self) -> Option<&Exception> {
        match &self.either {
            Either::Exception(exception) => Some(exception),
            _ => None,
        }
    }

    pub fn fold_vec(values: &[Self]) -> Self {
        let mut array = vec![];
        for value in values {
            match &value.either {
                Either::Value(v) => array.push(v.to_owned()),
                Either::Exception(_) => return value.to_owned(),
            }
        }

        Value::Array(array).into()
    }

    pub fn fold_pairs(values: &[(String, Self)]) -> Self {
        let mut object = serde_json::Map::new();
        for (key, value) in values {
            match &value.either {
                Either::Value(v) => {
                    object.insert(key.to_string(), v.to_owned());
                }
                Either::Exception(_) => return value.to_owned(),
            }
        }

        Value::Object(object).into()
    }
}

impl From<Value> for TaskResult {
    fn from(value: Value) -> TaskResult {
        TaskResult { either: Either::Value(value) }
    }
}

impl From<Exception> for TaskResult {
    fn from(exception: Exception) -> TaskResult {
        TaskResult { either: Either::Exception(exception) }
    }
}
