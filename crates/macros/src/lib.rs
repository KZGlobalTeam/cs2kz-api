use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use syn::{parse_macro_input, ItemFn};

use self::integration_test::TestArgs;

mod integration_test;

macro_rules! error {
	($span:expr, $($message:tt)*) => {
		return Err(syn::Error::new($span.span(), format_args!($($message)*)));
	};
}

pub(crate) use error;

/// Run an integration test.
///
/// # Test Setup
///
/// This macro will generate the boilerplate necessary for running **integration tests**.
/// This means that every test will get its own API instance and database! Your test function
/// should have the following signature:
///
/// ```rust,ignore
/// async fn my_test(ctx: &Context);
/// ```
///
/// Every test implicitly has a return type of `Result<()>` and returns `Ok(())` as the default
/// case, which means you don't need to specify either of them.
///
/// The `Context` parameter can be used to make requests to the API, using the `http_client`
/// field and `Context::url()` method. A connection pool to the API's database is also
/// provided, as well as a shutdown signal if the API needs to be shut down prematurely.
///
/// # Fixtures
///
/// You can specify a list of "fixtures" to be ran after the standard migrations by specifying
/// `fixtures = […]` as part of the macro arguments, like so:
///
/// ```rust,ignore
/// #[crate::integration_test(fixtures = ["my-fixture"])]
/// async fn my_test(ctx: &Context) {
///     // ...
/// }
/// ```
///
/// The names have to correspond to `.sql` files in `./database/fixtures`. In this example, you
/// would need to create a file called `./database/fixtures/my-fixture.sql` (from the repo root).
#[proc_macro_error]
#[proc_macro_attribute]
pub fn integration_test(args: TokenStream, test_function: TokenStream) -> TokenStream {
	let test_args = parse_macro_input!(args as TestArgs);
	let test_function = parse_macro_input!(test_function as ItemFn);

	match integration_test::expand(test_args, test_function) {
		Ok(tokens) => tokens,
		Err(error) => error.into_compile_error().into(),
	}
}
