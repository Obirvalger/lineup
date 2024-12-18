use serde_json::Value;

#[derive(Clone, Debug)]
pub enum Exception {
    BreakTaskline { taskline: Option<String>, result: Value },
}
