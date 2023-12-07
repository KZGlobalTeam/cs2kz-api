macro_rules! header {
	($name:ident wraps $type:ty as $header_name:literal) => {
		use axum::http::{HeaderName, HeaderValue};
		use axum_extra::headers::{self, Header};
		use serde::Deserialize;
		use utoipa::IntoParams;

		#[derive(Debug, Deserialize, IntoParams)]
		#[into_params(names($header_name), parameter_in = Header)]
		pub struct $name(pub $type);

		mod header_name {
			use axum::http::HeaderName;

			#[allow(non_upper_case_globals)]
			pub(super) static $name: HeaderName = HeaderName::from_static($header_name);
		}

		impl Header for $name {
			fn name() -> &'static HeaderName {
				&header_name::$name
			}

			fn decode<'i, I>(values: &mut I) -> Result<Self, headers::Error>
			where
				Self: Sized,
				I: Iterator<Item = &'i HeaderValue>,
			{
				let value = values
					.next()
					.ok_or_else(headers::Error::invalid)?
					.to_str()
					.map_err(|_| headers::Error::invalid())?
					.parse::<$type>()
					.map_err(|_| headers::Error::invalid())?;

				Ok(Self(value))
			}

			fn encode<E: Extend<HeaderValue>>(&self, values: &mut E) {
				let s = self.0.to_string();
				let value =
					HeaderValue::from_str(&s).expect(concat!(stringify!($type), " is valid ASCII"));

				values.extend(std::iter::once(value));
			}
		}
	};
}

mod api_key {
	header!(ApiKey wraps u32 as "api-key");
}

pub use api_key::ApiKey;

mod plugin_version {
	header!(PluginVersion wraps u16 as "plugin-version");
}

pub use plugin_version::PluginVersion;
