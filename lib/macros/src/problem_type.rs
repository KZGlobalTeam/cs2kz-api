use proc_macro::TokenStream;
use quote::quote;
use syn::spanned::Spanned as _;
use syn::{Attribute, Expr, ExprLit, Ident, ItemEnum, Lit, Meta, MetaNameValue};

use crate::error;

const DOCS: &str = include_str!("../../../static/problem-types.html");

pub fn expand(item: ItemEnum) -> syn::Result<TokenStream>
{
	let name = &item.ident;

	let all = item
		.variants
		.iter()
		.map(|variant| {
			let name = &variant.ident;
			quote!(Self::#name)
		})
		.collect::<Vec<_>>();

	let problems = item
		.variants
		.iter()
		.map(|variant| {
			let name = variant.ident.to_string();
			let slug = heck::AsKebabCase(&name).to_string();
			let docs = doc_attrs(&variant.attrs)?;
			let (status, span) = variant
				.attrs
				.iter()
				.map(|attr| &attr.meta)
				.find_map(|meta| match *meta {
					Meta::NameValue(MetaNameValue { ref path, ref value, .. }) => path
						.get_ident()
						.filter(|ident| *ident == "status")
						.map(|_| value),

					_ => None,
				})
				.ok_or_else(|| syn::Error::new(variant.ident.span(), "missing `status` attribute"))
				.and_then(|status| match status {
					Expr::Lit(ExprLit { lit: Lit::Int(int), .. }) => {
						Ok((int.base10_parse::<u16>()?, status.span()))
					}
					_ => error!(status, "`status` attribute must be an integer literal"),
				})?;

			Ok((Problem { name, slug, docs, status }, span))
		})
		.collect::<syn::Result<Vec<_>>>()?;

	let docs = problems
		.iter()
		.map(|(Problem { name, slug, docs, .. }, _)| {
			let docs = docs.join("<br>");

			format! { r##"
			  <br>
			  <div>
			    <h1 id="{slug}"><a href="#{slug}">{name}</a></h1>
			    <span>{docs}</span>
			  </div>
			"## }
			.replace("\t", "  ")
		})
		.collect::<Vec<_>>()
		.join("\n");

	let docs = DOCS.replace("__CONTENT__", &docs).replace("\t", "  ");

	let name_match_arms = problems
		.iter()
		.map(|(Problem { name, .. }, _)| {
			let variant = syn::parse_str::<Ident>(name)?;
			let name = heck::AsTitleCase(name).to_string();

			Ok(quote!(Self::#variant => #name))
		})
		.collect::<syn::Result<Vec<_>>>()?;

	let title_match_arms = problems
		.iter()
		.map(|(Problem { name, slug, .. }, _)| {
			let variant = syn::parse_str::<Ident>(name)?;

			Ok(quote!(Self::#variant => #slug))
		})
		.collect::<syn::Result<Vec<_>>>()?;

	let status_match_arms = problems
		.iter()
		.map(|(Problem { name, status, .. }, status_span)| {
			let variant = syn::parse_str::<Ident>(name)?;
			let code = match *status {
				100 => quote!(CONTINUE),
				101 => quote!(SWITCHING_PROTOCOLS),
				102 => quote!(PROCESSING),
				200 => quote!(OK),
				201 => quote!(CREATED),
				202 => quote!(ACCEPTED),
				203 => quote!(NON_AUTHORITATIVE_INFORMATION),
				204 => quote!(NO_CONTENT),
				205 => quote!(RESET_CONTENT),
				206 => quote!(PARTIAL_CONTENT),
				207 => quote!(MULTI_STATUS),
				208 => quote!(ALREADY_REPORTED),
				226 => quote!(IM_USED),
				300 => quote!(MULTIPLE_CHOICES),
				301 => quote!(MOVED_PERMANENTLY),
				302 => quote!(FOUND),
				303 => quote!(SEE_OTHER),
				304 => quote!(NOT_MODIFIED),
				305 => quote!(USE_PROXY),
				307 => quote!(TEMPORARY_REDIRECT),
				308 => quote!(PERMANENT_REDIRECT),
				400 => quote!(BAD_REQUEST),
				401 => quote!(UNAUTHORIZED),
				402 => quote!(PAYMENT_REQUIRED),
				403 => quote!(FORBIDDEN),
				404 => quote!(NOT_FOUND),
				405 => quote!(METHOD_NOT_ALLOWED),
				406 => quote!(NOT_ACCEPTABLE),
				407 => quote!(PROXY_AUTHENTICATION_REQUIRED),
				408 => quote!(REQUEST_TIMEOUT),
				409 => quote!(CONFLICT),
				410 => quote!(GONE),
				411 => quote!(LENGTH_REQUIRED),
				412 => quote!(PRECONDITION_FAILED),
				413 => quote!(PAYLOAD_TOO_LARGE),
				414 => quote!(URI_TOO_LONG),
				415 => quote!(UNSUPPORTED_MEDIA_TYPE),
				416 => quote!(RANGE_NOT_SATISFIABLE),
				417 => quote!(EXPECTATION_FAILED),
				418 => quote!(IM_A_TEAPOT),
				421 => quote!(MISDIRECTED_REQUEST),
				422 => quote!(UNPROCESSABLE_ENTITY),
				423 => quote!(LOCKED),
				424 => quote!(FAILED_DEPENDENCY),
				426 => quote!(UPGRADE_REQUIRED),
				428 => quote!(PRECONDITION_REQUIRED),
				429 => quote!(TOO_MANY_REQUESTS),
				431 => quote!(REQUEST_HEADER_FIELDS_TOO_LARGE),
				451 => quote!(UNAVAILABLE_FOR_LEGAL_REASONS),
				500 => quote!(INTERNAL_SERVER_ERROR),
				501 => quote!(NOT_IMPLEMENTED),
				502 => quote!(BAD_GATEWAY),
				503 => quote!(SERVICE_UNAVAILABLE),
				504 => quote!(GATEWAY_TIMEOUT),
				505 => quote!(HTTP_VERSION_NOT_SUPPORTED),
				506 => quote!(VARIANT_ALSO_NEGOTIATES),
				507 => quote!(INSUFFICIENT_STORAGE),
				508 => quote!(LOOP_DETECTED),
				510 => quote!(NOT_EXTENDED),
				511 => quote!(NETWORK_AUTHENTICATION_REQUIRED),
				_ => return Err(syn::Error::new(*status_span, "not a valid status code")),
			};

			Ok(quote!(Self::#variant => ::http::StatusCode::#code))
		})
		.collect::<syn::Result<Vec<_>>>()?;

	Ok(quote! {
		impl #name
		{
			/// HTML documentation for the problem types.
			pub const DOCS: &'static str = #docs;

			/// Returns a list of all variants of this enum.
			pub const fn all() -> &'static [Self]
			{
				&[#(#all),*]
			}

			/// Returns the title of a given problem type.
			pub const fn title(&self) -> &'static str
			{
				match self {
					#(#name_match_arms),*
				}
			}

			/// Returns the URI slug of a given problem type.
			pub const fn slug(&self) -> &'static str
			{
				match self {
					#(#title_match_arms),*
				}
			}

			/// Returns the HTTP status code that the resulting response should have.
			pub const fn status(&self) -> ::http::StatusCode
			{
				match self {
					#(#status_match_arms),*
				}
			}
		}
	}
	.into())
}

#[derive(Debug)]
struct Problem
{
	name: String,
	slug: String,
	docs: Vec<String>,
	status: u16,
}

fn doc_attrs(attrs: &[Attribute]) -> syn::Result<Vec<String>>
{
	attrs
		.iter()
		.map(|attr| &attr.meta)
		.filter_map(|meta| match *meta {
			Meta::NameValue(MetaNameValue { ref path, ref value, .. }) => path
				.get_ident()
				.filter(|ident| *ident == "doc")
				.map(|_| value),

			_ => None,
		})
		.map(|doc| match doc {
			Expr::Lit(ExprLit { lit: Lit::Str(text), .. }) => Ok(text.value()),
			_ => error!(doc, "doc comments should be strings"),
		})
		.collect()
}
