use crate::net_io::{PacketRead, PacketWrite, VarInt};
use anyhow::bail;
use log::error;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use std::io::Cursor;
use tokio::io::AsyncReadExt;

use crate::world::block::Location;

pub trait AsComponent {
    fn as_component(&self) -> Component;
}

impl<S> AsComponent for S
where
    S: Into<String> + Clone,
{
    fn as_component(&self) -> Component {
        let str: String = Clone::clone(self).into();
        Component::text(str)
    }
}

impl From<&str> for Component {
    fn from(str: &str) -> Self {
        str.as_component()
    }
}

#[skip_serializing_none]
#[derive(Default, Debug, Clone, Serialize, Deserialize)]
pub struct Component {
    extra: Option<Vec<Component>>,
    bold: Option<bool>,
    italic: Option<bool>,
    obfuscated: Option<bool>,
    strikethrough: Option<bool>,
    underlined: Option<bool>,
    reset: Option<bool>,
    color: Option<TextColor>,
    #[serde(flatten)]
    contents: MessageContents,
}

impl ToString for Component {
    fn to_string(&self) -> String {
        serde_json::to_string(self).unwrap()
    }
}

macro_rules! _fmt_impl {
    ($($n:ident),* $(,)*) => {
        $(
        pub fn $n(&mut self, $n: bool) -> Self {
            self.$n = Some($n);
            self.clone()
        }
        )*
    }
}

impl Component {
    pub fn text<S>(msg: S) -> Self
    where
        S: Into<String>,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Plain { text: msg.into() };
        df.clone()
    }

    pub fn translate<S, C>(msg: S, placeholders: Option<Vec<C>>) -> Self
    where
        S: Into<String>,
        C: AsComponent,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Translate(TranslatedMessage {
            translate: msg.into(),
            with: placeholders.map(|it| {
                it.iter()
                    .map(|e| e.as_component())
                    .collect::<Vec<Component>>()
            }),
        });
        df.clone()
    }

    pub fn score<S>(name: S, objective: S, placeholder: Option<S>) -> Self
    where
        S: Into<String>,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Score {
            score: ScoreboardMessage {
                name: name.into(),
                objective: objective.into(),
                value: placeholder.map(|it| it.into()),
            },
        };
        df.clone()
    }

    pub fn entity<S, C>(selector: S, separator: Option<C>) -> Self
    where
        S: Into<String>,
        C: AsComponent,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Entity(Box::from(EntityMessage {
            selector: selector.into(),
            separator: separator.map(|it| it.as_component()),
        }));
        df.clone()
    }

    pub fn keybind<S: Into<String>>(key: S) -> Self {
        let mut df = Self::default();
        df.contents = MessageContents::Keybind(KeyMessage {
            keybind: key.into(),
        });
        df.clone()
    }

    pub fn block_nbt<S, C>(
        path: S,
        position: Location,
        interpret: Option<bool>,
        separator: Option<C>,
    ) -> Self
    where
        S: Into<String>,
        C: AsComponent,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Nbt(Box::from(NbtMessage {
            nbt: path.into(),
            interpret,
            separator: separator.map(|it| it.as_component()),
            block: Some(position.into()),
            entity: None,
            storage: None,
        }));
        df.clone()
    }

    pub fn entity_nbt<S, C>(
        path: S,
        selector: S,
        interpret: Option<bool>,
        separator: Option<C>,
    ) -> Self
    where
        S: Into<String>,
        C: AsComponent,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Nbt(Box::from(NbtMessage {
            nbt: path.into(),
            interpret,
            separator: separator.map(|it| it.as_component()),
            block: None,
            entity: Some(selector.into()),
            storage: None,
        }));
        df.clone()
    }

    pub fn storage_nbt<S, C>(
        path: S,
        storage: S,
        interpret: Option<bool>,
        separator: Option<C>,
    ) -> Self
    where
        S: Into<String>,
        C: AsComponent,
    {
        let mut df = Self::default();
        df.contents = MessageContents::Nbt(Box::from(NbtMessage {
            nbt: path.into(),
            interpret,
            separator: separator.map(|it| it.as_component()),
            block: None,
            entity: None,
            storage: Some(storage.into()),
        }));
        df.clone()
    }

    pub fn append<C>(&mut self, comp: C) -> Self
    where
        C: Into<Component>,
    {
        if let Some(vec) = &mut self.extra {
            vec.push(comp.into());
            self.extra = Some(vec.to_owned())
        } else {
            self.extra = Some(vec![comp.into()])
        }
        self.clone()
    }

    pub fn hex_color(&mut self, color: u64) -> Self {
        let str = format!("#{:2X}", color);
        self.color = Some(TextColor::Hex(str));
        self.clone()
    }

    pub fn color(&mut self, color: NamedColor) -> Self {
        self.color = Some(TextColor::Named(color));
        self.clone()
    }

    _fmt_impl! {
        bold, italic, obfuscated, strikethrough, underlined, reset,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum MessageContents {
    Plain { text: String },
    Translate(TranslatedMessage),
    Score { score: ScoreboardMessage },
    Entity(Box<EntityMessage>),
    Keybind(KeyMessage),
    Nbt(Box<NbtMessage>),
}

impl Default for MessageContents {
    fn default() -> Self {
        MessageContents::Plain {
            text: "".to_string(),
        }
    }
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NbtMessage {
    nbt: String,
    interpret: Option<bool>,
    separator: Option<Component>,
    block: Option<String>,
    entity: Option<String>,
    storage: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KeyMessage {
    keybind: String,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntityMessage {
    selector: String,
    separator: Option<Component>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslatedMessage {
    translate: String,
    with: Option<Vec<Component>>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScoreboardMessage {
    name: String,
    objective: String,
    value: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum TextColor {
    Named(NamedColor),
    Hex(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum NamedColor {
    DarkRed,
    Red,
    Gold,
    Yellow,
    DarkGreen,
    Green,
    Aqua,
    DarkAqua,
    DarkBlue,
    Blue,
    LightPurple,
    DarkPurple,
    White,
    Gray,
    DarkGray,
    Black,
}

pub const MAX_COMPONENT_JSON_SIZE: usize = 262144;

#[async_trait::async_trait]
impl PacketWrite for Component {
    async fn pack_write(&self, buffer: &mut Vec<u8>, target_version: u32) -> anyhow::Result<()> {
        let str = serde_json::ser::to_string(self)?;

        let bytes = str.as_bytes();
        let size = bytes.len();

        if size > MAX_COMPONENT_JSON_SIZE {
            log::error!(
                "Write Component too long (max size: {}, json size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
            bail!(
                "Write Component too long (max size: {}, json size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
        }

        VarInt(size as i32)
            .pack_write(buffer, target_version)
            .await?;

        buffer.extend_from_slice(bytes);

        Ok(())
    }
}

#[async_trait::async_trait]
impl PacketRead for Component {
    async fn pack_read(buffer: &mut Cursor<&[u8]>, target_version: u32) -> anyhow::Result<Self> {
        let size = VarInt::pack_read(buffer, target_version).await?.0 as usize;

        if size > MAX_COMPONENT_JSON_SIZE {
            error!(
                "Read Component too long (max size: {}, received size: {})",
                MAX_COMPONENT_JSON_SIZE, size
            );
            bail!(
                "Read Component too long (max size: {}, received size: {})",
                MAX_COMPONENT_JSON_SIZE,
                size
            );
        }

        let mut buf = vec![0u8; size];
        AsyncReadExt::read_exact(buffer, &mut buf).await?;

        serde_json::de::from_str(&String::from_utf8(buf)?).map_err(anyhow::Error::from)
    }
}

#[macro_export]
macro_rules! component {
    // raw literals
    ($obj:literal $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.append($crate::component!($($recurse)*)))?
    };
    // normal literals
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* $obj:literal $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color($crate::chat::NamedColor::$named_color))* $(.hex_color($hex_color))* $(.append($crate::component!($($recurse)*)))?
    };
    // variables
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* # $obj:ident $(& $($recurse:tt)*)?) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color(NamedColor::$named_color))* $(.hex_color($hex_color))* $(.append($crate::component!($($recurse)*)))?
    };
    // expressions (no recursion here)
    ( $(@ $hex_color:literal)* $(@ $named_color:ident)* $($attr:ident)* $(! $not_attr:ident)* # $obj:expr) => {
        Component::text($obj.to_string()) $(.$attr(true))* $(.$not_attr(false))* $(.color(NamedColor::$named_color))* $(.hex_color($hex_color))*
    };
}
