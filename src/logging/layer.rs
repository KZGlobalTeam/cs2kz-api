use tracing::span;
use tracing_subscriber::layer;
use tracing_subscriber::registry::{LookupSpan, SpanRef};
use uuid::Uuid;

use crate::logging::log::{Log, Value};

/// A generic tracing layer that will collect [`Log`]s and save them according to the
/// [`Consumer`] implementation of the logger it is wrapping.
pub struct Layer<C: 'static> {
	consumer: &'static C,
}

impl<C> Layer<C> {
	pub fn new(consumer: C) -> Self {
		Self { consumer: Box::leak(Box::new(consumer)) }
	}
}

pub trait Consumer {
	fn save_log(&'static self, log: Log);
}

impl<S, C> tracing_subscriber::Layer<S> for Layer<C>
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
	C: Consumer + 'static,
{
	fn on_new_span(
		&self,
		attributes: &span::Attributes<'_>,
		span_id: &span::Id,
		ctx: layer::Context<'_, S>,
	) {
		let log = Log::from(attributes);
		let span = ctx.span(span_id).expect("invalid span id");
		let mut extensions = span.extensions_mut();

		extensions.insert(log);
	}

	fn on_record(&self, span_id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
		let span = ctx.span(span_id).expect("invalid span id");
		let mut extensions = span.extensions_mut();

		if let Some(log) = extensions.get_mut::<Log>() {
			values.record(log);
		} else {
			let mut log = Log::from(span.metadata());
			values.record(&mut log);
			extensions.insert(log);
		}
	}

	fn on_event(&self, event: &tracing::Event<'_>, ctx: layer::Context<'_, S>) {
		let mut log = Log::from(event);

		if let Some(request_id) = event
			.parent()
			.and_then(|id| ctx.span(id))
			.or_else(|| ctx.lookup_current())
			.and_then(find_request_id)
		{
			log.fields.insert("request_id", request_id.into());
		}

		self.consumer.save_log(log);
	}

	fn on_close(&self, span_id: span::Id, ctx: layer::Context<'_, S>) {
		let span = ctx.span(&span_id).expect("invalid span id");
		let mut extensions = span.extensions_mut();

		let Some(mut log) = extensions.remove::<Log>() else {
			return;
		};

		drop(extensions);

		if let Some(request_id) = find_request_id(span) {
			log.fields.insert("request_id", request_id.into());
		}

		self.consumer.save_log(log);
	}
}

fn find_request_id<'a, R>(span: SpanRef<'a, R>) -> Option<Uuid>
where
	R: LookupSpan<'a>,
{
	span.scope().flat_map(span_to_id).next()
}

fn span_to_id<'a, R>(span: SpanRef<'a, R>) -> Option<Uuid>
where
	R: LookupSpan<'a>,
{
	span.extensions()
		.get::<Log>()
		.and_then(|log| log.field("id"))
		.and_then(value_to_uuid)
}

fn value_to_uuid(value: &Value) -> Option<Uuid> {
	if let Value::String(id) = value {
		id.parse().map(Some).expect("invalid id format")
	} else {
		None
	}
}
