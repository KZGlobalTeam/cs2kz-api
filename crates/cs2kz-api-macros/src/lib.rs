use std::fs;
use std::path::PathBuf;

use proc_macro::TokenStream;
use proc_macro_error::proc_macro_error;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::punctuated::Punctuated;
use syn::spanned::Spanned;
use syn::token::Comma;
use syn::{parse_macro_input, Block, FnArg, Ident, ItemFn, Lit, Pat, Type};

static FIXTURES_PATH: &str = "./database/fixtures";

#[proc_macro_error]
#[proc_macro_attribute]
pub fn test(args: TokenStream, item: TokenStream) -> TokenStream {
	let Args { queries } = parse_macro_input!(args as Args);
	let Test { name, ctx_arg, body } = parse_macro_input!(item as Test);

	quote! {
		#[::tokio::test]
		async fn #name() -> ::color_eyre::Result<()> {
			use ::std::fmt::Write as _;
			use ::color_eyre::eyre::Context as _;
			use ::sqlx::migrate::MigrateDatabase as _;
			use ::rand::Rng as _;

			let mut config = crate::Config::new().context("failed to read config")?;
			let port = ::rand::thread_rng().gen_range(3000..=u16::MAX);

			config.socket_addr.set_port(port);

			let mut database_url = ::std::env::var("TEST_DATABASE_URL")
				.context("missing `TEST_DATABASE_URL` environment variable")?;

			write!(&mut database_url, "-test-{port}")?;

			config.database.url = database_url
				.parse()
				.context("invalid `TEST_DATABASE_URL`")?;

			::sqlx::mysql::MySql::drop_database(&database_url)
				.await
				.with_context(|| format!("failed to drop database {port}"))?;

			::sqlx::mysql::MySql::create_database(&database_url)
				.await
				.with_context(|| format!("failed to create database {port}"))?;

			let connection_pool = ::sqlx::mysql::MySqlPool::connect(&database_url)
				.await
				.context("failed to connect to test database")?;

			crate::tests::MIGRATOR
				.run(&connection_pool)
				.await
				.with_context(|| format!("failed to run migrations for database {port}"))?;

			#(
				::sqlx::query!(#queries)
					.execute(&connection_pool)
					.await
					.with_context(|| format!("failed to run migration for database {port}"))?;
			)*

			let addr = config.socket_addr;
			let ctx = crate::tests::Context::new(addr, connection_pool);

			::tokio::task::spawn(async move {
				crate::API::new(config)
					.await
					.expect("failed to create api")
					.run()
					.await;

				unreachable!("api shutdown?");
			});

			async fn test(#ctx_arg: crate::tests::Context) -> ::color_eyre::Result<()> {
				::tokio::task::yield_now().await;

				#body

				::color_eyre::Result::Ok(())
			}

			test(ctx).await.with_context(|| format!("test {port} failed"))?;

			::sqlx::mysql::MySql::drop_database(&database_url)
				.await
				.with_context(|| format!("failed to drop database {port}"))?;

			::color_eyre::Result::Ok(())
		}
	}
	.into()
}

#[derive(Debug)]
struct Args {
	queries: Vec<String>,
}

#[derive(Debug)]
struct Test {
	name: Ident,
	ctx_arg: Ident,
	body: Block,
}

macro_rules! error {
	( $item:expr, $( $arg:tt )+ ) => {
		return Err(::syn::Error::new($item.span(), format!( $( $arg )+ )));
	};
}

impl Parse for Args {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		Punctuated::<Lit, Comma>::parse_terminated(input)?
			.into_iter()
			.try_fold(Vec::new(), |mut queries, argument| {
				let Lit::Str(filename) = argument else {
					error!(
						argument,
						"Invalid attribute argument. Expected one or more filenames."
					);
				};

				let path = PathBuf::from(FIXTURES_PATH).join(filename.value());

				if !path.extension().is_some_and(|ext| ext == "sql") {
					error!(filename, "Files must end in `.sql`.");
				}

				if !path.exists() {
					error!(filename, "`{path:?} does not exist.");
				}

				let file = fs::read_to_string(&path).map_err(|err| {
					syn::Error::new(filename.span(), format!("Error reading `{path:?}`: {err}"))
				})?;

				let new_queries = file
					.split(';')
					.map(|query| query.trim())
					.filter(|query| !query.is_empty())
					.map(|query| query.to_owned());

				queries.extend(new_queries);

				Ok(queries)
			})
			.map(|queries| Self { queries })
	}
}

impl Parse for Test {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let function = ItemFn::parse(input)?;

		if function.sig.asyncness.is_none() {
			error!(function.sig, "Tests must be marked as `async`.");
		}

		if function.sig.inputs.len() > 1 {
			error!(
				function.sig.inputs,
				"Tests can only take a single argument of type `Context`."
			);
		}

		let Some(ctx_arg) = function.sig.inputs.iter().next() else {
			error!(function.sig, "Tests have to take one argument of type `Context`.",);
		};

		let FnArg::Typed(ctx_arg) = ctx_arg else {
			error!(ctx_arg, "Tests cannot take a `self` parameter.");
		};

		if !ctx_arg.attrs.is_empty() {
			error!(ctx_arg, "Test arguments cannot be annotated.");
		}

		let Pat::Ident(ctx_arg_ident) = ctx_arg.pat.as_ref() else {
			error!(
				ctx_arg.pat,
				"Test arguemnt identifiers cannot be used with pattern matching."
			);
		};

		let Type::Path(ctx_arg_type) = ctx_arg.ty.as_ref() else {
			error!(ctx_arg.ty, "Tests take a single argument of type `Context`.");
		};

		if !ctx_arg_type
			.path
			.get_ident()
			.is_some_and(|ident| ident == "Context")
		{
			error!(ctx_arg_type, "Tests take a single argument of type `Context`.");
		}

		let body = *function.block;

		Ok(Self { name: function.sig.ident, ctx_arg: ctx_arg_ident.ident.clone(), body })
	}
}
