use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::convert::{From, TryFrom};
use std::fmt;
use std::result::Result as StdResult;
use std::str::FromStr;

use anyhow::Context as AnyhowContext;
use anyhow::{bail, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use serde_json::{json, value::Value};
use tera::Context;

use crate::error::Error;
use crate::fs_var::FsVar;
use crate::render::Render;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Kind {
    Fs,
    Json,
    #[default]
    Nothing,
    Raw,
    Yaml,
}

impl Kind {
    pub fn is_nothing(&self) -> bool {
        matches!(self, Kind::Nothing)
    }

    fn process_value<N: AsRef<str>, P: AsRef<str>>(
        &self,
        value: &Value,
        args: &BTreeMap<String, String>,
        name: N,
        context: &Context,
        place: P,
    ) -> Result<Value> {
        let name = name.as_ref();
        let mut render = true;
        if let Some(render_arg) = args.get("render") {
            match render_arg.as_str() {
                "true" => {}
                "false" => render = false,
                _ => bail!(Error::BadKindArgRedner(render_arg.to_string())),
            }
        }

        let value = match self {
            Self::Fs => {
                let value = if render {
                    value.render(context, format!("variables in {}", place.as_ref()))?
                } else {
                    value.to_owned()
                };

                let fs_var = FsVar::new(name)?;
                fs_var.write(&value)?;

                Value::String(name.to_string())
            }
            Self::Json => {
                let value = if render {
                    value.render(context, format!("variables in {}", place.as_ref()))?
                } else {
                    value.to_owned()
                };
                match value {
                    Value::String(s) => serde_json::from_str(&s)
                        .with_context(|| format!("failed to parse json variable `{}`", &name))?,
                    _ => bail!(Error::WrongVarType(name.to_string(), "string".to_string())),
                }
            }
            Self::Yaml => {
                let value = if render {
                    value.render(context, format!("variables in {}", place.as_ref()))?
                } else {
                    value.to_owned()
                };
                match value {
                    Value::String(s) => serde_yaml::from_str(&s)
                        .with_context(|| format!("failed to parse yaml variable `{}`", &name))?,
                    _ => bail!(Error::WrongVarType(name.to_string(), "string".to_string())),
                }
            }
            Self::Nothing => {
                if render {
                    value.render(context, format!("variables in {}", place.as_ref()))?
                } else {
                    value.to_owned()
                }
            }
            Self::Raw => value.to_owned(),
        };

        Ok(value)
    }
}

impl FromStr for Kind {
    type Err = Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "fs" => Ok(Self::Fs),
            "json" | "j" => Ok(Self::Json),
            "raw" | "r" => Ok(Self::Raw),
            "yaml" => Ok(Self::Yaml),
            "" => Ok(Self::Nothing),
            _ => Err(Error::UnknownVarKind(s.to_string())),
        }
    }
}

impl From<Kind> for String {
    fn from(kind: Kind) -> String {
        kind.to_string()
    }
}

impl fmt::Display for Kind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Fs => write!(f, "fs"),
            Self::Json => write!(f, "json"),
            Self::Nothing => write!(f, ""),
            Self::Raw => write!(f, "raw"),
            Self::Yaml => write!(f, "yaml"),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, PartialOrd, Ord, Eq)]
#[serde(rename_all = "kebab-case")]
enum Type {
    Bool,
    Number,
    U64,
    I64,
    F64,
    String,
    Array,
    Object,
}

impl Type {
    fn is_match(&self, value: &Value) -> bool {
        match self {
            Self::Bool => value.is_boolean(),
            Self::Number => value.is_number(),
            Self::U64 => value.is_u64(),
            Self::I64 => value.is_i64(),
            Self::F64 => value.is_f64(),
            Self::String => value.is_string(),
            Self::Array => value.is_array(),
            Self::Object => value.is_object(),
        }
    }
}

impl FromStr for Type {
    type Err = Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bool" | "b" => Ok(Self::Bool),
            "number" | "n" => Ok(Self::Number),
            "u64" | "u" => Ok(Self::U64),
            "i64" | "i" => Ok(Self::I64),
            "f64" | "f" => Ok(Self::F64),
            "string" | "s" => Ok(Self::String),
            "array" | "a" => Ok(Self::Array),
            "object" | "o" => Ok(Self::Object),
            _ => Err(Error::UnknownVarType(s.to_string())),
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Bool => write!(f, "bool"),
            Self::Number => write!(f, "number"),
            Self::U64 => write!(f, "u64"),
            Self::I64 => write!(f, "i64"),
            Self::F64 => write!(f, "f64"),
            Self::String => write!(f, "string"),
            Self::Array => write!(f, "array"),
            Self::Object => write!(f, "object"),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", try_from = "String", into = "String")]
pub struct Var {
    pub name: String,
    types: BTreeSet<Type>,
    pub kind: Kind,
    kind_args: BTreeMap<String, String>,
}

impl Var {
    pub fn from_name<S: AsRef<str>>(name: S) -> Self {
        Self {
            name: name.as_ref().to_string(),
            types: Default::default(),
            kind: Default::default(),
            kind_args: Default::default(),
        }
    }

    fn parse_kind_args<S: AsRef<str>>(kind_args: S) -> StdResult<BTreeMap<String, String>, Error> {
        let mut args = BTreeMap::new();
        static RE: Lazy<Regex> = Lazy::new(|| Regex::new(r",\s*").unwrap());

        for arg in RE.split(kind_args.as_ref()) {
            if let Some((name, value)) = arg.split_once(':') {
                args.insert(name.to_string(), value.to_string());
            } else {
                return Err(Error::BadKindArg(arg.to_string()));
            }
        }

        Ok(args)
    }

    fn parse_types<S: AsRef<str>>(type_: S) -> StdResult<BTreeSet<Type>, Error> {
        let mut types = BTreeSet::new();

        for type_ in type_.as_ref().split([' ', '|']) {
            if !type_.is_empty() {
                types.insert(type_.parse()?);
            }
        }

        Ok(types)
    }

    pub fn check_type(&self, value: &Value) -> Result<()> {
        if self.types.is_empty() {
            return Ok(());
        }

        for type_ in &self.types {
            if type_.is_match(value) {
                return Ok(());
            }
        }

        bail!(Error::WrongVarType(
            self.name.to_string(),
            self.types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | ")
        ))
    }
}

impl PartialEq for Var {
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl Eq for Var {}

impl Ord for Var {
    fn cmp(&self, other: &Self) -> Ordering {
        self.name.cmp(&other.name)
    }
}

impl PartialOrd for Var {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl FromStr for Var {
    type Err = Error;

    fn from_str(s: &str) -> StdResult<Self, Self::Err> {
        Var::try_from(s.to_string())
    }
}

impl TryFrom<String> for Var {
    type Error = Error;

    fn try_from(s: String) -> StdResult<Self, Self::Error> {
        let mut name = s;
        let mut kind = "".to_string();
        let mut kind_args = BTreeMap::new();
        let mut type_ = "".to_string();

        static RE: Lazy<Regex> = Lazy::new(|| {
            Regex::new(
                r#"^(?x)
                    (?: (?<kind>\w+) (?: \( (?<kind_args>[^)]+) \) )? \s*%\s* )?
                    (?<name>[.\w]+)
                    (?:\s*:\s*(?<type>\w+(?:\s*\|\s*(?<it>\w+))*))?
                $"#,
            )
            .unwrap()
        });

        if let Some(captures) = RE.captures(&name.to_string()) {
            name = captures["name"].to_string();
            if let Some(kind_capture) = captures.name("kind") {
                kind = kind_capture.as_str().to_string();
            }
            if let Some(kind_args_capture) = captures.name("kind_args") {
                kind_args = Var::parse_kind_args(kind_args_capture.as_str())?;
            }
            if let Some(type_capture) = captures.name("type") {
                type_ = type_capture.as_str().to_string();
            }
        } else {
            return Err(Error::BadVar(name));
        }

        let kind = kind.parse()?;
        let types = Var::parse_types(type_)?;
        let var = Var { name, types, kind, kind_args };

        Ok(var)
    }
}

impl From<Var> for String {
    fn from(var: Var) -> Self {
        var.to_string()
    }
}

impl fmt::Display for Var {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut var = self.name.to_string();
        if !self.kind.is_nothing() {
            var = format!("{} % {}", self.kind, var);
        }
        if !self.types.is_empty() {
            var = format!(
                "{}: {}",
                var,
                self.types.iter().map(|t| t.to_string()).collect::<Vec<_>>().join(" | ")
            );
        }

        write!(f, "{}", var)
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub struct Vars(BTreeMap<Var, Value>);

impl Vars {
    pub fn new() -> Self {
        let map = BTreeMap::new();
        Self(map)
    }

    pub fn insert(&mut self, key: Var, value: Value) -> Option<Value> {
        self.0.insert(key, value)
    }

    pub fn extend(&mut self, other: Self) {
        self.0.extend(other.0);
    }

    pub fn into_map(self) -> BTreeMap<String, Value> {
        self.0.into_iter().map(|(k, v)| (k.to_string(), v)).collect()
    }

    pub fn context(&self) -> Result<Context> {
        let mut context = Context::new();
        for (var, value) in &self.0 {
            context.insert(var.name.to_string(), value);
        }

        Ok(context)
    }
}

impl TryFrom<BTreeMap<String, Value>> for Vars {
    type Error = Error;

    fn try_from(vars: BTreeMap<String, Value>) -> Result<Self, Self::Error> {
        let mut map = BTreeMap::new();
        for (var_str, value) in vars {
            let var = Var::try_from(var_str)?;
            map.insert(var, value);
        }

        Ok(Vars(map))
    }
}

impl From<Context> for Vars {
    fn from(context: Context) -> Self {
        let mut map = BTreeMap::new();
        let context = match context.into_json() {
            Value::Object(o) => o,
            _ => panic!("Context into_json is not object"),
        };

        for (name, value) in context {
            let var = Var::from_name(name);
            map.insert(var, value);
        }

        Vars(map)
    }
}

impl Render for Vars {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let mut new_map = BTreeMap::new();
        for (var, value) in &self.0 {
            let mut var = var.to_owned();
            let mut value = var.kind.process_value(
                value,
                &var.kind_args,
                &var.name,
                context,
                place.as_ref(),
            )?;
            var.check_type(&value)?;

            let parts = var.name.split('.').rev().collect::<Vec<_>>();
            for part in &parts[..parts.len() - 1] {
                value = json!({part.to_string(): value});
            }
            var.name = parts[parts.len() - 1].to_string();

            new_map.insert(var, value);
        }

        Ok(Self(new_map))
    }
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", from = "Vec<Vars>", into = "Vec<Vars>")]
pub struct Maps {
    maps: Vec<Vars>,
    context: Context,
    place: String,
}

impl Maps {
    pub fn vars(self) -> Result<Vars> {
        let mut new_vars = Vars::new();
        let mut context = self.context;

        for vars in self.maps {
            let vars = vars.render(&context, format!("ExtVars::vars in {}", &self.place))?;
            context.extend(vars.context()?);
            new_vars.extend(vars);
        }

        Ok(new_vars)
    }
}

impl From<Vec<Vars>> for Maps {
    fn from(maps: Vec<Vars>) -> Self {
        Self { maps, context: Context::new(), place: Default::default() }
    }
}

impl From<Maps> for Vec<Vars> {
    fn from(val: Maps) -> Self {
        val.maps
    }
}

impl Render for Maps {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        let maps = self.maps.to_owned();
        let context = context.to_owned();
        let place = place.as_ref().to_string();

        Ok(Self { maps, context, place })
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case", untagged)]
pub enum ExtVars {
    Vars(Vars),
    Maps(Maps),
}

impl ExtVars {
    pub fn vars(self) -> Result<Vars> {
        match self {
            Self::Vars(vars) => Ok(vars),
            Self::Maps(maps) => Ok(maps.vars()?),
        }
    }
}

impl Default for ExtVars {
    fn default() -> Self {
        ExtVars::Vars(Vars::new())
    }
}

impl Render for ExtVars {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        match self {
            Self::Vars(vars) => Ok(Self::Vars(vars.render(context, place)?)),
            Self::Maps(maps) => Ok(Self::Maps(maps.render(context, place)?)),
        }
    }
}
