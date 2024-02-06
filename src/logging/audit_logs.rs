use sqlx::types::Json as SqlJson;
use sqlx::MySqlPool;
use tokio::task;
use tracing::{error, span};
use tracing_subscriber::filter::FilterFn;
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::{layer, Layer};

use crate::logging::log::Log;

pub struct AuditLogs {
	database: MySqlPool,
}

impl AuditLogs {
	pub fn layer<S>(database: MySqlPool) -> impl tracing_subscriber::Layer<S>
	where
		S: tracing::Subscriber + for<'a> LookupSpan<'a>,
	{
		Self { database }.with_filter(FilterFn::new(|metadata| metadata.target() == "audit_log"))
	}

	fn save_log(&self, Log { level, source, message, fields, .. }: Log) {
		let database = self.database.clone();
		let query = sqlx::query! {
			r#"
			INSERT INTO
			  AuditLogs (level, source, message, fields)
			VALUES
			  (?, ?, ?, ?)
			"#,
			level.as_str(),
			source,
			message,
			SqlJson(fields),
		};

		task::spawn(async move {
			if let Err(error) = query.execute(&database).await {
				error!(%error, "failed to save audit log");
			}
		});
	}
}

impl<S> tracing_subscriber::Layer<S> for AuditLogs
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
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
		self.save_log(Log::from(event));
	}

	fn on_close(&self, span_id: span::Id, ctx: layer::Context<'_, S>) {
		let span = ctx.span(&span_id).expect("invalid span id");
		let mut extensions = span.extensions_mut();

		if let Some(log) = extensions.remove::<Log>() {
			self.save_log(log);
		}
	}
}
