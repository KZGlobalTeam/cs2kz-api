use std::fs;
use std::path::Path;

use proc_macro::TokenStream;
use quote::quote;
use syn::parse::{Parse, ParseStream};
use syn::spanned::Spanned;
use syn::{
	Expr, ExprArray, ExprAssign, ExprLit, ExprPath, FnArg, Ident, ItemFn, Lit, Pat, PatIdent,
	PatType, PathArguments, PathSegment, ReturnType, Signature, Type, TypePath, TypeReference,
	Visibility,
};

use crate::error;

pub fn expand(TestArgs { queries }: TestArgs, test_function: ItemFn) -> syn::Result<TokenStream> {
	let Visibility::Inherited = &test_function.vis else {
		error!(
			test_function.vis,
			"test functions do not have to be marked `pub`"
		);
	};

	let signature = &test_function.sig;
	let test_function_ident = &test_function.sig.ident;
	let test_function_body = *test_function.block;
	let ctx_param = validate_signature(signature)?;
	let fixtures = if queries.is_empty() {
		quote! {}
	} else {
		quote! {
			::tokio::try_join!(#(::sqlx::query!(#queries).execute(&ctx.database)),*)?;
		}
	};

	let output = quote! {
		#[tokio::test]
		async fn #test_function_ident() -> ::anyhow::Result<()> {
			use crate::test::Context;
			use ::anyhow::Context as _;

			#signature -> ::anyhow::Result<()> {
				use ::anyhow::{Context as _, bail, anyhow};
				use crate::test::{assert, assert_eq, assert_ne};

				#test_function_body

				Ok(())
			}

			let #ctx_param = Context::new()
				.await
				.context("create test context")?;

			#fixtures

			let result = #test_function_ident(&#ctx_param).await;
			let test_id = #ctx_param.test_id;

			#ctx_param
				.cleanup()
				.await
				.context("test cleanup")?;

			if let Err(e) = result {
				::anyhow::bail!("test {test_id} failed: {e:#?}");
			}

			Ok(())
		}
	};

	Ok(output.into())
}

pub struct TestArgs {
	queries: Vec<String>,
}

impl Parse for TestArgs {
	fn parse(input: ParseStream) -> syn::Result<Self> {
		let mut args = Self {
			queries: Vec::new(),
		};

		if input.is_empty() {
			return Ok(args);
		}

		let fixtures = ExprAssign::parse(input)?;

		let Expr::Path(ExprPath {
			qself: None,
			path: syn::Path {
				segments: fixtures_ident_path,
				..
			},
			..
		}) = fixtures.left.as_ref()
		else {
			error!(
				fixtures.left.as_ref(),
				"arguments must be `<ident> = <value>`"
			);
		};

		let Some(PathSegment {
			ident: fixtures_ident,
			arguments: PathArguments::None,
		}) = fixtures_ident_path.first()
		else {
			error!(
				fixtures.left.as_ref(),
				"arguments must be `<ident> = <value>`"
			);
		};

		if fixtures_ident != "fixtures" {
			error!(fixtures_ident, "unknown argument; try `fixtures`");
		}

		let Expr::Array(ExprArray {
			elems: fixtures, ..
		}) = fixtures.right.as_ref()
		else {
			error!(
				fixtures.right,
				"`fixtures` must be a list of file names: `[\"my-fixture\"]`"
			);
		};

		for filename in fixtures {
			let Expr::Lit(ExprLit { lit: filename, .. }) = filename else {
				error!(filename, "fixtures must be string literals");
			};

			let Lit::Str(filename) = filename else {
				error!(filename, "invalid argument; expected filenames");
			};

			let path = Path::new("./database/fixtures").join(filename.value() + ".sql");
			let contents = match fs::read_to_string(&path) {
				Ok(contents) => contents,
				Err(err) => {
					error!(filename, "failed to read {path:?}: {err}");
				}
			};

			let new_queries = contents
				.split(';')
				.map(|query| query.trim())
				.filter(|query| !query.is_empty())
				.map(|query| query.to_owned());

			args.queries.extend(new_queries);
		}

		Ok(args)
	}
}

fn validate_signature(signature: &Signature) -> syn::Result<&Ident> {
	if let Some(constness) = &signature.constness {
		error!(constness, "test functions cannot be marked `const`");
	}

	if signature.asyncness.is_none() {
		error!(&signature.fn_token, "test functions must be marked `async`");
	}

	if let Some(unsafety) = &signature.unsafety {
		error!(unsafety, "test functions cannot be marked `unsafe`");
	}

	if let Some(abi) = &signature.abi {
		error!(abi, "test functions cannot have a custom ABI");
	}

	if !signature.generics.params.is_empty() {
		error!(
			signature.generics,
			"test functions cannot take generic parameters"
		);
	}

	if signature.inputs.len() != 1 {
		error!(
			signature.inputs,
			"test functions must take a single parameter of type `&Context`"
		);
	}

	let Some(FnArg::Typed(PatType {
		pat: ctx_param_ident,
		ty: ctx_param_ty,
		..
	})) = signature.inputs.first()
	else {
		error!(
			signature.inputs,
			"test functions do not take a `self` parameter"
		);
	};

	let Type::Reference(TypeReference {
		lifetime: None,
		mutability: None,
		elem: ctx_ty_path,
		..
	}) = ctx_param_ty.as_ref()
	else {
		error!(
			ctx_param_ty,
			"test functions must take a single parameter of type `&Context`"
		);
	};

	let Type::Path(TypePath {
		path: ctx_ty_path, ..
	}) = ctx_ty_path.as_ref()
	else {
		error!(
			ctx_ty_path,
			"test functions must take a single parameter of type `&Context`"
		);
	};

	let ctx_ty_ident = ctx_ty_path.require_ident()?;

	if ctx_ty_ident != "Context" {
		error!(
			ctx_ty_path,
			"test functions must take a single parameter of type `Context`"
		);
	}

	if let Some(variadic) = &signature.variadic {
		error!(variadic, "test functions do not take variadic parameters");
	}

	if let ReturnType::Type(_, return_ty) = &signature.output {
		error!(
			return_ty,
			"all test functions implicitly return `Result<()>`; remove the return type"
		);
	}

	let Pat::Ident(PatIdent {
		ident: ctx_param_ident,
		..
	}) = ctx_param_ident.as_ref()
	else {
		error!(ctx_param_ident, "`ctx` parameter cannot be destructured");
	};

	Ok(ctx_param_ident)
}
