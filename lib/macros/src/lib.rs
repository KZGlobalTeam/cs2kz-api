use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemEnum};

mod problem_type;

macro_rules! error {
	($span:expr, $($message:tt)*) => {{
		use syn::spanned::Spanned as _;
		return Err(syn::Error::new($span.span(), format_args!($($message)*)))
	}};
}

pub(crate) use error;

#[proc_macro_error]
#[proc_macro_derive(ProblemType, attributes(status))]
pub fn problem_type(item: TokenStream) -> TokenStream
{
	let item = parse_macro_input!(item as ItemEnum);

	match problem_type::expand(item) {
		Ok(tokens) => tokens,
		Err(error) => error.into_compile_error().into(),
	}
}
