use std::fs;
use std::path::{Path, PathBuf};

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::spanned::Spanned;
use syn::{parse_macro_input, AttributeArgs, FnArg, ItemFn, Lit, NestedMeta, Pat, Type};

static FIXTURES_PATH: &str = "./database/fixtures";

macro_rules! error {
	($item:expr, $($rest:tt)+) => {
		return ::syn::Error::new($item.span(), format!($($rest)+))
			.into_compile_error()
			.into();
	}
}

#[proc_macro_error]
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
	let args = parse_macro_input!(args as AttributeArgs);
	let function = parse_macro_input!(item as ItemFn);

	let mut post_migration_queries = Vec::new();

	for arg in args.into_iter() {
		let NestedMeta::Lit(Lit::Str(literal)) = arg else {
			error!(arg, "Invalid attribute argument. Expected list of string literals.");
		};

		let path = Path::new(FIXTURES_PATH).join(PathBuf::from(literal.value()));

		if !path.extension().is_some_and(|ext| ext == "sql") {
			error!(literal, "Files are expected to end in `.sql`.");
		}

		if !path.exists() {
			error!(literal, "`{path:?}` does not exist.");
		}

		let queries = fs::read_to_string(path)
			.unwrap()
			.split(';')
			.map(|query| query.trim().to_owned())
			.filter(|query| !query.is_empty())
			.collect::<Vec<String>>();

		post_migration_queries.extend(queries);
	}

	if function.sig.asyncness.is_none() {
		error!(function.sig, "Test functions must be marked as `async`.");
	}

	if function.sig.inputs.len() != 1 {
		error! {
			function.sig.inputs,
			"Test functions only accept a single argument of type `Context`."
		};
	}

	let argument = function.sig.inputs.first().unwrap();

	let FnArg::Typed(argument) = argument else {
		error!(argument, "Test functions cannot have a `self` parameter.");
	};

	if !argument.attrs.is_empty() {
		error!(argument.attrs[0], "Arguments cannot be annotated.");
	}

	let Pat::Ident(_argument_identifier) = argument.pat.as_ref() else {
		error!(argument.pat, "Argument identifiers cannot use pattern matching.");
	};

	let Type::Path(argument_type) = argument.ty.as_ref() else {
		error!(argument.ty, "Argument types must be paths.");
	};

	if !argument_type
		.path
		.get_ident()
		.is_some_and(|ident| ident == "Context")
	{
		error!(argument_type.path, "Invalid argument type. Expected `Context`.");
	}

	let test_name = &function.sig.ident;
	let inner_fn = &function.block;

	quote! {
		#[test]
		fn #test_name() -> ::color_eyre::Result<()> {
			use ::color_eyre::eyre::Context as _;
			use ::sqlx::migrate::MigrateDatabase as _;
			use ::std::fmt::Write as _;

			::tokio::runtime::Runtime::new()
				.context("failed to construct tokio runtime")?
				.block_on(async move {
					let tcp_listener = ::tokio::net::TcpListener::bind("127.0.0.1:0")
						.await
						.context("failed to bind tcp listener")?;

					let addr = tcp_listener
						.local_addr()
						.context("failed to get tcp listener addr")?;

					let port = addr.port();

					let mut database_url =
						::std::env::var("TEST_DATABASE_URL").context("missing `TEST_DATABASE_URL`")?;

					write!(&mut database_url, "-test-{port}")?;

					::sqlx::mysql::MySql::create_database(&database_url)
						.await
						.with_context(|| format!("failed to create test database {port}"))?;

					let database = ::sqlx::mysql::MySqlPoolOptions::new()
						.connect(&database_url)
						.await
						.context("failed to connect to database")?;

					crate::tests::MIGRATOR
						.run(&database)
						.await
						.with_context(|| format!("failed to run migrations for {port}"))?;

					#(
						::sqlx::query!(#post_migration_queries)
							.execute(&database)
							.await
							.context("failed to execute post-migration script")?;
					)*

					let mut config = crate::Config::new()
						.await
						.context("failed to load API configuration")?;

					config.socket_addr.set_port(port);
					config
						.api_url
						.set_port(Some(port))
						.map_err(|_| ::color_eyre::eyre::eyre!("failed to set API port"))?;

					::tokio::task::spawn(async move {
						crate::API::run(config, database, tcp_listener)
							.await
							.expect("Failed to run API.");

						unreachable!("API shutdown prematurely.");
					});

					let ctx = crate::tests::Context {
						client: ::reqwest::Client::new(),
						addr,
					};

					if let err @ ::color_eyre::Result::Err(_) = { #inner_fn ::color_eyre::Result::Ok(()) } {
						return err;
					}

					::sqlx::mysql::MySql::drop_database(&database_url)
						.await
						.with_context(|| format!("failed to drop test database {port}"))?;

					::color_eyre::Result::Ok(())
				})
		}
	}
	.into()
}
