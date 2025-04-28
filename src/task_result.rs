use std::collections::BTreeMap;

use serde_json::{to_value, Value};

use crate::exception::Exception;
use crate::template::Context;

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
    pub fn add_vars(&mut self, vars: Value) {
        if let Either::Value(v) = &mut self.either {
            v.as_object_mut().unwrap().insert("vars".to_string(), vars);
        }
    }

    pub fn as_value(&self) -> Option<&Value> {
        match &self.either {
            Either::Value(value) => Some(
                value
                    .as_object()
                    .expect("internal value should be an object")
                    .get("value")
                    .expect("internal value should contain `value` key"),
            ),
            _ => None,
        }
    }

    pub fn as_exception(&self) -> Option<&Exception> {
        match &self.either {
            Either::Exception(exception) => Some(exception),
            _ => None,
        }
    }

    pub fn as_context(&self) -> Option<Context> {
        match &self.either {
            Either::Value(value) => {
                if let Some(vars) =
                    value.as_object().expect("internal value should be an object").get("vars")
                {
                    Context::from_value(vars.to_owned()).ok()
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn fold_vec(results: &[Self]) -> Self {
        let mut value_array = vec![];
        let mut vars_array = vec![];
        for result in results {
            match &result.either {
                Either::Value(v) => {
                    let whole_value = v.as_object().expect("internal value should be an object");

                    value_array.push(
                        whole_value
                            .get("value")
                            .expect("internal value should contain `value` key")
                            .to_owned(),
                    );

                    if let Some(vars) = whole_value.get("vars") {
                        vars_array.push(vars.to_owned());
                    }
                }
                Either::Exception(_) => return result.to_owned(),
            }
        }

        let mut result: Self = Value::Array(value_array).into();

        let mut vars_object: BTreeMap<String, Vec<Value>> = BTreeMap::new();
        for vars in vars_array {
            let vars = vars.as_object().expect("internal value vars should be an object");
            for (name, value) in vars {
                vars_object
                    .entry(name.to_owned())
                    .and_modify(|e| {
                        e.push(value.to_owned());
                    })
                    .or_insert_with(|| vec![value.to_owned()]);
            }
        }
        result.add_vars(to_value(vars_object).expect("BTreeMap with string keys is valid value"));

        result
    }

    pub fn fold_items(results: &[(String, Self)]) -> Self {
        let mut value_object = serde_json::Map::new();
        let mut items_vars_object = serde_json::Map::new();
        for (item, result) in results {
            match &result.either {
                Either::Value(whole_value) => {
                    let whole_value =
                        whole_value.as_object().expect("internal value should be an object");

                    value_object.insert(
                        item.to_string(),
                        whole_value
                            .get("value")
                            .expect("internal value should contain `value` key")
                            .to_owned(),
                    );

                    if let Some(vars) = whole_value.get("vars") {
                        items_vars_object.insert(item.to_string(), vars.to_owned());
                    }
                }
                Either::Exception(_) => return result.to_owned(),
            }
        }

        let mut result: Self = Value::Object(value_object).into();

        let mut vars_object: BTreeMap<String, BTreeMap<String, Value>> = BTreeMap::new();
        for (item, vars) in items_vars_object {
            let vars = vars.as_object().expect("internal value vars should be an object");
            for (name, value) in vars {
                vars_object
                    .entry(name.to_owned())
                    .and_modify(|e| {
                        e.insert(item.to_string(), value.to_owned());
                    })
                    .or_insert_with(|| BTreeMap::from([(item.to_string(), value.to_owned())]));
            }
        }
        result.add_vars(to_value(vars_object).expect("BTreeMap with string keys is valid value"));

        result
    }
}

impl From<Value> for TaskResult {
    fn from(value: Value) -> TaskResult {
        let mut object = serde_json::Map::new();
        object.insert("value".to_string(), value);
        TaskResult { either: Either::Value(Value::Object(object)) }
    }
}

impl From<Exception> for TaskResult {
    fn from(exception: Exception) -> TaskResult {
        TaskResult { either: Either::Exception(exception) }
    }
}
