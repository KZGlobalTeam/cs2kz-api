use tracing::span;
use tracing_subscriber::layer;
use tracing_subscriber::registry::LookupSpan;

use crate::logging::log::Log;

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

	fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
		self.consumer.save_log(Log::from(event));
	}

	fn on_close(&self, span_id: span::Id, ctx: layer::Context<'_, S>) {
		let span = ctx.span(&span_id).expect("invalid span id");
		let mut extensions = span.extensions_mut();

		if let Some(log) = extensions.remove::<Log>() {
			self.consumer.save_log(log);
		}
	}
}
