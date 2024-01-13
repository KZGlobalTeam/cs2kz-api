use std::borrow::Cow;

use serde::Deserialize;

#[derive(Deserialize)]
#[serde(untagged)]
pub enum IntOrStr<'a, Int> {
	Int(Int),
	Str(Cow<'a, str>),
}
