use std::sync::Arc;
use std::time::Duration;

use axiom_rs::datasets::{ContentEncoding, ContentType};
use cs2kz_api::config::axiom::Config as AxiomConfig;
use cs2kz_api::{audit, Result};
use serde_json::Value as JsonValue;
use sqlx::mysql::MySqlPoolOptions;
use sqlx::MySqlPool;
use tokio::task;
use tokio::time::timeout;
use tracing::{error, span, trace};
use tracing_subscriber::layer;
use tracing_subscriber::registry::LookupSpan;

use crate::logging::visitor::Visitor;

#[derive(Clone)]
pub struct Layer {
	database: MySqlPool,
	axiom: Option<(Arc<axiom_rs::Client>, String)>,
}

impl Layer {
	pub async fn new(database_url: &str, axiom_config: Option<AxiomConfig>) -> Result<Self> {
		let database = MySqlPoolOptions::new().connect(database_url).await?;
		let axiom = axiom_config.and_then(|config| {
			let client = axiom_rs::Client::builder()
				.with_token(config.token)
				.with_org_id(config.org_id)
				.build()
				.map(Arc::new)
				.map_err(|err| {
					eprintln!("Failed to connect to axiom: {err}");
					err
				})
				.ok()?;

			Some((client, config.dataset))
		});

		Ok(Self { database, axiom })
	}

	fn save_log(self, visitor: Visitor) {
		let Self { database, axiom } = self;

		if let Some((client, dataset)) = axiom {
			if visitor.is_axiom {
				let bytes = serde_json::to_vec(&[&visitor]).expect("invalid log values?");
				let task = Self::save_log_to_axiom(client, dataset, bytes);

				task::spawn(timeout(Duration::from_secs(5), task));
			}
		}

		if visitor.is_audit {
			task::spawn(Self::save_log_to_database(database, visitor));
		}
	}

	async fn save_log_to_database(database: MySqlPool, visitor: Visitor) {
		let query = sqlx::query! {
			r#"
			INSERT INTO
			  AuditLogs (level, source, message, fields)
			VALUES
			  (?, ?, ?, ?)
			"#,
			visitor.level,
			visitor.source,
			&visitor.message,
			JsonValue::Object(visitor.fields),
		};

		if let Err(err) = query.execute(&database).await {
			error!(%err, "failed to save audit logs");
		}
	}

	async fn save_log_to_axiom(client: Arc<axiom_rs::Client>, dataset: String, bytes: Vec<u8>) {
		match client
			.ingest_bytes(dataset, bytes, ContentType::Json, ContentEncoding::Identity)
			.await
		{
			Ok(status) => {
				trace!(?status, skip_axiom = true, "sent data to axiom");
			}
			Err(error) => {
				audit!(error, "failed sending logs to axiom", %error, skip_axiom = true);
			}
		}
	}
}

// Implementation details:
//
// 1. Normal events are captured in `on_event` and recorded by a `Visitor`.
// 2. Spans are initially captured in `on_new_span` and recorded by a `Visitor`. They are then
//    stored in the span's extensions for later retrieval.
// 3. If a span has its fields changed later, those changes will be captured in `on_record`.
// 4. The last time we get to access any span is `on_close`, which saves the current visitor state.
impl<S> tracing_subscriber::Layer<S> for Layer
where
	S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
	fn on_event(&self, event: &tracing::Event<'_>, _ctx: layer::Context<'_, S>) {
		let mut visitor = Visitor::new(event.metadata());
		event.record(&mut visitor);

		self.clone().save_log(visitor);
	}

	fn on_new_span(
		&self,
		attributes: &span::Attributes<'_>,
		span_id: &span::Id,
		ctx: layer::Context<'_, S>,
	) {
		let span = ctx.span(span_id).unwrap();
		let event = tracing::Event::new(attributes.metadata(), attributes.values());
		let mut visitor = Visitor::new(event.metadata());

		event.record(&mut visitor);
		span.extensions_mut().insert(visitor);
	}

	fn on_record(&self, span_id: &span::Id, values: &span::Record<'_>, ctx: layer::Context<'_, S>) {
		let span = ctx.span(span_id).unwrap();
		let mut extensions = span.extensions_mut();

		if let Some(visitor) = extensions.get_mut::<Visitor>() {
			values.record(visitor);
			return;
		}

		let mut visitor = Visitor::new(span.metadata());
		values.record(&mut visitor);
		extensions.insert(visitor);
	}

	fn on_close(&self, span_id: span::Id, ctx: layer::Context<'_, S>) {
		let span = ctx.span(&span_id).unwrap();
		let mut extensions = span.extensions_mut();

		if let Some(visitor) = extensions.remove::<Visitor>() {
			self.clone().save_log(visitor);
		}
	}
}
