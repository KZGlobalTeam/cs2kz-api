use std::fs;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::{parse_macro_input, Block, FnArg, Ident, ItemFn, Lit, Pat, Token, Type};

static FIXTURES_PATH: &str = "./database/fixtures";

#[proc_macro_error]
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
	let TestArgs { queries } = parse_macro_input!(args as TestArgs);
	let TestFunction { name, ctx_arg, body, .. } = parse_macro_input!(item as TestFunction);

	quote! {
		#[::tokio::test]
		async fn #name() -> ::color_eyre::Result<()> {
			use ::color_eyre::eyre::Context as _;
			use ::sqlx::migrate::MigrateDatabase as _;
			use ::std::fmt::Write as _;

			// Create TCP server for the API
			let tcp_listener = ::tokio::net::TcpListener::bind("127.0.0.1:0")
				.await
				.context("failed to bind tcp listener")?;

			let addr = tcp_listener
				.local_addr()
				.context("failed to get tcp listener addr")?;

			let port = addr.port();

			// Generate "unique" database URL for this test
			let mut database_url =
				::std::env::var("TEST_DATABASE_URL").context("missing `TEST_DATABASE_URL`")?;

			write!(&mut database_url, "-test-{port}")?;

			// Drop the old one, just in case
			::sqlx::mysql::MySql::drop_database(&database_url)
				.await
				.with_context(|| format!("failed to drop test database {port}"))?;

			// Create the test database
			::sqlx::mysql::MySql::create_database(&database_url)
				.await
				.with_context(|| format!("failed to create test database {port}"))?;

			let database = ::sqlx::mysql::MySqlPoolOptions::new()
				.connect(&database_url)
				.await
				.context("failed to connect to database")?;

			// Run migrations
			crate::tests::MIGRATOR
				.run(&database)
				.await
				.with_context(|| format!("failed to run migrations for {port}"))?;

			// Run fixtures
			#(
				::sqlx::query!(#queries)
					.execute(&database)
					.await
					.context("failed to execute post-migration script")?;
			)*

			// Setup API
			let mut config = crate::Config::new()
				.await
				.context("failed to load API configuration")?;

			config.socket_addr.set_port(port);
			config
				.api_url
				.set_port(Some(port))
				.map_err(|_| ::color_eyre::eyre::eyre!("failed to set API port"))?;

			// Spawn API in a background task
			::tokio::task::spawn(async move {
				crate::API::run(config, database, tcp_listener)
					.await
					.expect("Failed to run API.");

				unreachable!("API shutdown prematurely?");
			});

			async fn test(#ctx_arg: crate::tests::Context) -> ::color_eyre::Result<()> {
				#body
				::color_eyre::Result::Ok(())
			}

			let ctx = crate::tests::Context::new(addr);

			// Run the actual test
			test(ctx).await?;

			// Drop the test DB
			::sqlx::mysql::MySql::drop_database(&database_url)
				.await
				.with_context(|| format!("failed to drop test database {port}"))?;

			::color_eyre::Result::Ok(())
		}
	}
	.into()
}

macro_rules! error {
	($item:expr, $($args:tt)+) => {
		return Err(::syn::Error::new($item.span(), format!($($args)+)));
	}
}

#[derive(Debug)]
struct TestArgs {
	queries: Vec<String>,
}

impl Parse for TestArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let args = Punctuated::<Lit, Token![,]>::parse_terminated(input)?;
		let mut queries = Vec::new();

		for arg in args {
			let Lit::Str(filename) = arg else {
				error!(arg, "Invalid attribute argument. Expected one or more file names.");
			};

			let path = PathBuf::from(FIXTURES_PATH).join(filename.value());

			if !path.extension().is_some_and(|ext| ext == "sql") {
				error!(filename, "Files are expected to end in `.sql`.");
			}

			if !path.exists() {
				error!(filename, "`{path:?}` does not exist.");
			}

			let file = fs::read_to_string(path).map_err(|err| {
				syn::Error::new(filename.span(), format!("Error reading file: {err}"))
			})?;

			let new_queries = file
				.split(';')
				.map(|query| query.trim())
				.filter(|query| !query.is_empty())
				.map(|query| query.to_owned());

			queries.extend(new_queries);
		}

		Ok(Self { queries })
	}
}

#[derive(Debug)]
struct TestFunction {
	name: Ident,
	ctx_arg: Ident,
	body: Block,
}

impl Parse for TestFunction {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let function = input.parse::<ItemFn>()?;

		if function.sig.asyncness.is_none() {
			error!(function.sig, "Test functions must be marked as `async`.");
		}

		let name = function.sig.ident;
		let inputs = function.sig.inputs;

		if inputs.len() != 1 {
			error!(
				inputs,
				"Test functions only accept a single argument of type `Context`."
			);
		}

		let argument = inputs.into_iter().next().unwrap();

		let FnArg::Typed(argument) = argument else {
			error!(argument, "Test functions cannot have a `self` parameter.");
		};

		if !argument.attrs.is_empty() {
			error!(argument.attrs[0], "Arguments cannot be annotated.");
		}

		let Pat::Ident(argument_identifier) = *argument.pat else {
			error!(argument.pat, "Argument identifiers cannot use pattern matching.");
		};

		let Type::Path(argument_type) = *argument.ty else {
			error!(argument.ty, "Argument types must be paths.");
		};

		if !argument_type
			.path
			.get_ident()
			.is_some_and(|ident| ident == "Context")
		{
			error!(argument_type.path, "Invalid argument type. Expected `Context`.");
		}

		let ctx_arg = argument_identifier.ident;
		let body = *function.block;

		Ok(Self { name, ctx_arg, body })
	}
}
