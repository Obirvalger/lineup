use anyhow::Result;
use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::render::Render;
use crate::template::Context;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Matches {
    And(Vec<Matches>),
    Or(Vec<Matches>),
    AnyRe(String),
    ErrRe(String),
    OutRe(String),
}

impl Matches {
    pub fn is_match<O: AsRef<str>, E: AsRef<str>>(&self, out: O, err: E) -> Result<bool> {
        match self {
            Matches::And(ms) => {
                for m in ms {
                    if !m.is_match(out.as_ref(), err.as_ref())? {
                        return Ok(false);
                    }
                }

                Ok(true)
            }
            Matches::Or(ms) => {
                for m in ms {
                    if m.is_match(out.as_ref(), err.as_ref())? {
                        return Ok(true);
                    }
                }

                Ok(false)
            }
            Matches::AnyRe(re) => {
                let re = Regex::new(re)?;
                Ok(re.is_match(out.as_ref()) || re.is_match(err.as_ref()))
            }
            Matches::ErrRe(re) => {
                let re = Regex::new(re)?;
                Ok(re.is_match(err.as_ref()))
            }
            Matches::OutRe(re) => {
                let re = Regex::new(re)?;
                Ok(re.is_match(out.as_ref()))
            }
        }
    }
}

impl Render for Matches {
    fn render<S: AsRef<str>>(&self, context: &Context, place: S) -> Result<Self> {
        match self {
            Matches::And(ms) => {
                let mut new_ms: Vec<Matches> = Vec::with_capacity(ms.len());

                for m in ms {
                    new_ms.push(m.render(context, place.as_ref())?);
                }

                Ok(Matches::And(new_ms))
            }
            Matches::Or(ms) => {
                let mut new_ms: Vec<Matches> = Vec::with_capacity(ms.len());

                for m in ms {
                    new_ms.push(m.render(context, place.as_ref())?);
                }

                Ok(Matches::Or(new_ms))
            }
            Matches::AnyRe(re) => {
                Ok(Matches::AnyRe(re.render(context, format!("any-re in {}", place.as_ref()))?))
            }
            Matches::ErrRe(re) => {
                Ok(Matches::ErrRe(re.render(context, format!("err-re in {}", place.as_ref()))?))
            }
            Matches::OutRe(re) => {
                Ok(Matches::OutRe(re.render(context, format!("out-re in {}", place.as_ref()))?))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_out() -> Result<()> {
        assert!(Matches::OutRe("version".to_string()).is_match("version", "").unwrap());

        Ok(())
    }

    #[test]
    fn simple_err() -> Result<()> {
        assert!(Matches::ErrRe("version".to_string()).is_match("", "version").unwrap());

        Ok(())
    }

    #[test]
    fn simple_any_out() -> Result<()> {
        assert!(Matches::AnyRe("version".to_string()).is_match("version", "").unwrap());

        Ok(())
    }

    #[test]
    fn simple_any_err() -> Result<()> {
        assert!(Matches::AnyRe("version".to_string()).is_match("", "version")?);

        Ok(())
    }

    #[test]
    fn simple_or() -> Result<()> {
        let matches = "or = [ { err-re = 'LLM' }, { err-re = 'toml' }]";
        let matches = toml::from_str::<Matches>(matches)?;
        assert!(matches.is_match("", "toml")?);
        assert!(matches.is_match("", "LLM")?);

        Ok(())
    }

    #[test]
    fn simple_and() -> Result<()> {
        let matches = "and = [ { err-re = 'LLM' }, { err-re = 'toml' }]";
        let matches = toml::from_str::<Matches>(matches)?;
        assert!(!matches.is_match("", "toml")?);
        assert!(!matches.is_match("", "LLM")?);
        assert!(matches.is_match("", "toml LLM")?);

        Ok(())
    }

    #[test]
    fn nested() -> Result<()> {
        let matches =
            "and = [ { out-re = 'ls' }, {or = [{ err-re = 'LLM' }, { err-re = 'toml' }]}]";
        let matches = toml::from_str::<Matches>(matches)?;
        assert!(matches.is_match("ls", "toml")?);
        assert!(matches.is_match("ls", "LLM")?);
        assert!(!matches.is_match("", "toml LLM")?);
        assert!(!matches.is_match("ls", "")?);

        Ok(())
    }
}
