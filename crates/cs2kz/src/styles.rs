use std::str::FromStr;
use std::{fmt, ops};

use futures_util::{Stream, TryStreamExt as _};
use serde::de::{self, Deserialize, Deserializer};
use serde::ser::{Serialize, SerializeSeq, Serializer};

use crate::checksum::Checksum;
use crate::plugin::PluginVersionId;
use crate::{Context, database};

const AUTO_BHOP: u32 = 0b_0001;

#[repr(u32)]
#[derive(Debug, Display, Clone, Copy, sqlx::Type)]
pub enum Style {
    #[display("auto-bhop")]
    AutoBhop = AUTO_BHOP,
}

#[derive(Debug, Default, Clone, Copy, sqlx::Type)]
#[sqlx(transparent)]
pub struct Styles(u32);

#[derive(Debug, Clone, Copy, serde::Deserialize)]
pub struct ClientStyleInfo {
    pub style: Style,
    pub checksum: Checksum,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
pub struct StyleInfo {
    pub style: Style,
    pub linux_checksum: Checksum,
    pub windows_checksum: Checksum,
}

#[derive(Debug, Clone)]
pub struct StyleIter {
    bits: u32,
}

#[derive(Debug, Display, Error)]
#[display("unknown style")]
pub struct UnknownStyle {
    _priv: (),
}

#[tracing::instrument(skip(cx))]
pub fn get_for_plugin_version(
    cx: &Context,
    plugin_version_id: PluginVersionId,
) -> impl Stream<Item = database::Result<StyleInfo>> {
    sqlx::query_as!(
        StyleInfo,
        "SELECT
           id AS `style: Style`,
           linux_checksum AS `linux_checksum: Checksum`,
           windows_checksum AS `windows_checksum: Checksum`
         FROM StyleChecksums
         WHERE plugin_version_id = ?",
        plugin_version_id,
    )
    .fetch(cx.database().as_ref())
    .map_err(database::Error::from)
}

impl Style {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AutoBhop => "auto-bhop",
        }
    }
}

impl FromStr for Style {
    type Err = UnknownStyle;

    fn from_str(value: &str) -> Result<Self, Self::Err> {
        match value {
            "autobhop" | "auto-bhop" => Ok(Self::AutoBhop),
            _ => Err(UnknownStyle { _priv: () }),
        }
    }
}

impl TryFrom<u32> for Style {
    type Error = UnknownStyle;

    fn try_from(value: u32) -> Result<Self, Self::Error> {
        match value {
            AUTO_BHOP => Ok(Self::AutoBhop),
            _ => Err(UnknownStyle { _priv: () }),
        }
    }
}

impl ops::BitAnd for Style {
    type Output = Styles;

    fn bitand(self, rhs: Style) -> Self::Output {
        Styles(self as u32 & rhs as u32)
    }
}

impl ops::BitAnd<Styles> for Style {
    type Output = Styles;

    fn bitand(self, rhs: Styles) -> Self::Output {
        Styles(self as u32 & rhs.0)
    }
}

impl ops::BitOr for Style {
    type Output = Styles;

    fn bitor(self, rhs: Style) -> Self::Output {
        Styles(self as u32 | rhs as u32)
    }
}

impl ops::BitOr<Styles> for Style {
    type Output = Styles;

    fn bitor(self, rhs: Styles) -> Self::Output {
        Styles(self as u32 | rhs.0)
    }
}

impl ops::BitXor for Style {
    type Output = Styles;

    fn bitxor(self, rhs: Style) -> Self::Output {
        Styles(self as u32 ^ rhs as u32)
    }
}

impl ops::BitXor<Styles> for Style {
    type Output = Styles;

    fn bitxor(self, rhs: Styles) -> Self::Output {
        Styles(self as u32 ^ rhs.0)
    }
}

impl Styles {
    pub const fn none() -> Self {
        Self(0)
    }

    pub fn contains(self, other: impl Into<Styles>) -> bool {
        let other = Into::<Styles>::into(other);
        (self.0 & other.0) == other.0
    }

    pub fn count(&self) -> usize {
        self.0.count_ones() as usize
    }

    pub fn is_empty(&self) -> bool {
        self.count() == 0
    }

    pub fn iter(&self) -> StyleIter {
        StyleIter { bits: self.0 }
    }
}

impl Serialize for Style {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        self.as_str().serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for Style {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StyleVisitor;

        impl de::Visitor<'_> for StyleVisitor {
            type Value = Style;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a style")
            }

            fn visit_str<E>(self, value: &str) -> Result<Self::Value, E>
            where
                E: de::Error,
            {
                value
                    .parse()
                    .map_err(|_| E::invalid_value(de::Unexpected::Str(value), &self))
            }
        }

        deserializer.deserialize_str(StyleVisitor)
    }
}

impl From<Style> for Styles {
    fn from(style: Style) -> Self {
        Self(style as u32)
    }
}

impl IntoIterator for Styles {
    type Item = Style;
    type IntoIter = StyleIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl IntoIterator for &Styles {
    type Item = Style;
    type IntoIter = StyleIter;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl FromIterator<Style> for Styles {
    fn from_iter<I>(iter: I) -> Self
    where
        I: IntoIterator<Item = Style>,
    {
        iter.into_iter().fold(Self::none(), ops::BitOr::bitor)
    }
}

impl Serialize for Styles {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut serializer = serializer.serialize_seq(Some(self.count()))?;

        for style in self {
            serializer.serialize_element(&style)?;
        }

        serializer.end()
    }
}

impl<'de> Deserialize<'de> for Styles {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        struct StylesVisitor;

        impl<'de> de::Visitor<'de> for StylesVisitor {
            type Value = Styles;

            fn expecting(&self, fmt: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(fmt, "a list of styles")
            }

            fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
            where
                A: de::SeqAccess<'de>,
            {
                let mut styles = Styles::none();

                while let Some(style) = seq.next_element::<Style>()? {
                    styles |= style;
                }

                Ok(styles)
            }
        }

        deserializer.deserialize_seq(StylesVisitor)
    }
}

impl ops::BitAnd for Styles {
    type Output = Styles;

    fn bitand(self, rhs: Styles) -> Self::Output {
        Styles(self.0 & rhs.0)
    }
}

impl ops::BitAndAssign for Styles {
    fn bitand_assign(&mut self, rhs: Styles) {
        self.0 &= rhs.0;
    }
}

impl ops::BitAnd<Style> for Styles {
    type Output = Styles;

    fn bitand(self, rhs: Style) -> Self::Output {
        Styles(self.0 & rhs as u32)
    }
}

impl ops::BitAndAssign<Style> for Styles {
    fn bitand_assign(&mut self, rhs: Style) {
        self.0 &= rhs as u32;
    }
}

impl ops::BitOr for Styles {
    type Output = Styles;

    fn bitor(self, rhs: Styles) -> Self::Output {
        Styles(self.0 | rhs.0)
    }
}

impl ops::BitOrAssign for Styles {
    fn bitor_assign(&mut self, rhs: Styles) {
        self.0 |= rhs.0;
    }
}

impl ops::BitOr<Style> for Styles {
    type Output = Styles;

    fn bitor(self, rhs: Style) -> Self::Output {
        Styles(self.0 | rhs as u32)
    }
}

impl ops::BitOrAssign<Style> for Styles {
    fn bitor_assign(&mut self, rhs: Style) {
        self.0 |= rhs as u32;
    }
}

impl ops::BitXor for Styles {
    type Output = Styles;

    fn bitxor(self, rhs: Styles) -> Self::Output {
        Styles(self.0 ^ rhs.0)
    }
}

impl ops::BitXorAssign for Styles {
    fn bitxor_assign(&mut self, rhs: Styles) {
        self.0 ^= rhs.0;
    }
}

impl ops::BitXor<Style> for Styles {
    type Output = Styles;

    fn bitxor(self, rhs: Style) -> Self::Output {
        Styles(self.0 ^ rhs as u32)
    }
}

impl ops::BitXorAssign<Style> for Styles {
    fn bitxor_assign(&mut self, rhs: Style) {
        self.0 ^= rhs as u32;
    }
}

impl Iterator for StyleIter {
    type Item = Style;

    fn next(&mut self) -> Option<Self::Item> {
        if self.bits == 0 {
            return None;
        }

        let next_bit = 1 << self.bits.trailing_zeros();
        self.bits &= !next_bit;

        Some(Style::try_from(next_bit).expect("invalid style bit in `StyleIter`"))
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        let count = self.bits.count_ones() as usize;
        (count, Some(count))
    }
}

impl ExactSizeIterator for StyleIter {}
