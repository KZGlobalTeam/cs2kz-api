use std::marker::PhantomData;

use tokio::sync::mpsc;
use tokio::task;
use tokio::time::{sleep, Duration};
use tracing::span;
use tracing_subscriber::layer;
use tracing_subscriber::registry::LookupSpan;

use super::Log;

pub struct Layer<C> {
	tx: mpsc::UnboundedSender<Log>,
	_consumer: PhantomData<C>,
}

impl<C> Layer<C>
where
	C: Consumer + Send + Sync + 'static,
{
	pub fn new(consumer: C) -> Self {
		let (tx, mut rx) = mpsc::unbounded_channel();

		task::spawn(async move {
			loop {
				process(&consumer, &mut rx);
				sleep(Duration::from_secs(1)).await;
			}
		});

		Self { tx, _consumer: PhantomData }
	}
}

fn process<C>(consumer: &C, rx: &mut mpsc::UnboundedReceiver<Log>)
where
	C: Consumer + Send + Sync + 'static,
{
	let mut logs = Vec::new();

	while let Ok(log) = rx.try_recv() {
		logs.push(log);
	}

	if !logs.is_empty() {
		consumer.consume(logs);
	}
}

pub trait Consumer {
	fn is_interested_in(metadata: &tracing::Metadata<'_>) -> bool;
	fn would_consume(log: &Log) -> bool;
	fn consume(&self, logs: Vec<Log>);
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
		if !C::is_interested_in(attributes.metadata()) {
			return;
		}

		let span = ctx.span(span_id).expect("valid span id");
		let event = tracing::Event::new(attributes.metadata(), attributes.values());
		let mut log = Log::from(event.metadata());

		attributes.record(&mut log);

		if C::would_consume(&log) {
			span.extensions_mut().insert(log);
		}
	}

	fn on_record(&self, span_id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
		let span = ctx.span(span_id).expect("valid span id");

		if !C::is_interested_in(span.metadata()) {
			return;
		}

		let mut extensions = span.extensions_mut();

		if let Some(log) = extensions.get_mut::<Log>() {
			values.record(log);
		} else {
			let mut log = Log::from(span.metadata());

			values.record(&mut log);

			if C::would_consume(&log) {
				extensions.insert(log);
			}
		}
	}

	fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
		if !C::is_interested_in(event.metadata()) {
			return;
		}

		let mut log = Log::from(event.metadata());

		event.record(&mut log);

		if C::would_consume(&log) {
			let _ = self.tx.send(log);
		}
	}

	fn on_close(&self, span_id: span::Id, ctx: layer::Context<'_, S>) {
		let span = ctx.span(&span_id).expect("valid span id");

		if !C::is_interested_in(span.metadata()) {
			return;
		}

		let mut extensions = span.extensions_mut();

		if let Some(log) = extensions.remove::<Log>() {
			if C::would_consume(&log) {
				let _ = self.tx.send(log);
			}
		}
	}
}
