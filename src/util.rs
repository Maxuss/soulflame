use anyhow::bail;
use regex::Regex;

#[derive(Debug, Clone, PartialEq, Hash)]
pub struct Identifier {
    namespace: String,
    path: String,
}

pub trait Identified {
    fn id(&self) -> Identifier;
}

lazy_static::lazy_static! {
    pub static ref SOULFLAME_NAMESPACE: String = "soulflame".into();
    pub static ref MINECRAFT_NAMESPACE: String = "minecraft".into();
    pub static ref NAMESPACE_RE: Regex = Regex::new(r"[a-z\d.-_]+").unwrap();
    pub static ref PATH_RE: Regex = Regex::new(r"[a-z\d.-_/]+").unwrap();
    pub static ref FULL_RE: Regex = Regex::new(r"([a-z\d.-_]+):([a-z\d.-_/]+)").unwrap();
}

impl Identifier {
    pub fn new<S: Into<String>>(namespace: S, path: S) -> anyhow::Result<Self> {
        let ns = namespace.into();
        let p = path.into();
        Ok(Self {
            namespace: if NAMESPACE_RE.is_match(&ns) {
                ns
            } else {
                bail!(
                    "Identifier namespace '{}' does not follow allowed pattern ([a-z\\d.-_]+)!",
                    ns
                )
            },
            path: if PATH_RE.is_match(&p) {
                p
            } else {
                bail!(
                    "Identifier path '{}' does not follow allowed pattern ([a-z\\d.-_/]+)!",
                    p
                )
            },
        })
    }

    pub fn soulflame<S: Into<String>>(path: S) -> anyhow::Result<Self> {
        let p = path.into();
        Ok(Self {
            namespace: SOULFLAME_NAMESPACE.clone(),
            path: if PATH_RE.is_match(&p) {
                p
            } else {
                bail!(
                    "Identifier path '{}' does not follow allowed pattern ([a-z\\d.-_/]+)!",
                    p
                )
            },
        })
    }

    pub fn minecraft<S: Into<String>>(path: S) -> anyhow::Result<Self> {
        let p = path.into();
        Ok(Self {
            namespace: MINECRAFT_NAMESPACE.clone(),
            path: if PATH_RE.is_match(&p) {
                p
            } else {
                bail!(
                    "Identifier path '{}' does not follow allowed pattern ([a-z\\d.-_/]+)!",
                    p
                )
            },
        })
    }

    pub fn path(&self) -> String {
        self.path.clone()
    }

    pub fn namespace(&self) -> String {
        self.namespace.clone()
    }

    pub fn parse<S: Into<String>>(from: S) -> anyhow::Result<Self> {
        let text = from.into();
        let matches = FULL_RE.captures(&text);
        if let Some(captures) = matches {
            let namespace = captures
                .get(0)
                .ok_or_else(|| anyhow::Error::msg("Could not match identifier namespace!"))?;
            let path = captures
                .get(1)
                .ok_or_else(|| anyhow::Error::msg("Could not match identifier path!"))?;
            Identifier::new(namespace.as_str(), path.as_str())
        } else {
            bail!("Invalid identifier provided in string '{}'! Should follow pattern '[a-z\\d.-_]+:[a-z\\d.-_/]+'!", text);
        }
    }
}

impl ToString for Identifier {
    fn to_string(&self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }
}

impl Into<String> for Identifier {
    fn into(self) -> String {
        format!("{}:{}", self.namespace, self.path)
    }
}
