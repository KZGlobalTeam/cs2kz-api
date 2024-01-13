use axum::http::Uri;
use url::{Host, Url};

pub trait UrlExt {
	/// Extracts the subdomain of this URL, if any.
	fn subdomain(&self) -> Option<&str>;

	/// This function compares the hosts of two URLs without considering subdomains.
	/// This means that `cs2.kz`, `dashboard.cs2.kz`, and `forum.cs2.kz` are all equal.
	fn host_eq_weak(&self, other: &Self) -> bool;
}

impl UrlExt for Url {
	fn subdomain(&self) -> Option<&str> {
		let Some(Host::Domain(domain)) = self.host() else {
			return None;
		};

		subdomain(domain)
	}

	fn host_eq_weak(&self, other: &Self) -> bool {
		fn inner(this: &Url, other: &Url) -> Option<bool> {
			Some(match (this.host()?, other.host()?) {
				(Host::Ipv4(this), Host::Ipv4(other)) => this == other,
				(Host::Ipv6(this), Host::Ipv6(other)) => this == other,
				(Host::Domain(this), Host::Domain(other)) => compare_domains(this, other),
				_ => false,
			})
		}

		inner(self, other).unwrap_or(false)
	}
}

impl UrlExt for Uri {
	fn subdomain(&self) -> Option<&str> {
		self.host().and_then(subdomain)
	}

	fn host_eq_weak(&self, other: &Self) -> bool {
		Option::zip(self.host(), other.host())
			.map(|(this, other)| compare_domains(this, other))
			.unwrap_or(false)
	}
}

fn subdomain(host: &str) -> Option<&str> {
	let mut segments = host.split('.');
	let subdomain = segments.next()?;
	let rest_count = segments.count();

	(rest_count == 2).then_some(subdomain)
}

fn compare_domains(a: &str, b: &str) -> bool {
	let a = a.split('.').rev().take(2);
	let b = b.split('.').rev().take(2);

	a.eq(b)
}

#[cfg(test)]
mod tests {
	use color_eyre::Result;
	use url::Url;

	use super::UrlExt;

	#[test]
	fn weak_url_cmp() -> Result<()> {
		let a = Url::parse("https://cs2.kz")?;
		let b = Url::parse("https://dashboard.cs2.kz")?;
		let c = Url::parse("https://dashboard.cs2.notkz")?;

		assert!(a.host_eq_weak(&b));
		assert!(!a.host_eq_weak(&c));
		assert!(!b.host_eq_weak(&c));

		Ok(())
	}
}
