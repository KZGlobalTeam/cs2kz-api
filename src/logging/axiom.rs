use std::io;
use std::sync::Arc;

use cs2kz_api::audit_error;
use cs2kz_api::config::axiom::Config as AxiomConfig;
use serde_json::Value as JsonValue;
use tokio::task;

#[derive(Default)]
pub struct Writer {
	dataset: String,
	client: Option<Arc<axiom_rs::Client>>,
}

impl Writer {
	pub fn new(AxiomConfig { token, org_id, dataset, .. }: AxiomConfig) -> Arc<Self> {
		let client = axiom_rs::Client::builder()
			.with_token(token)
			.with_org_id(org_id)
			.build()
			.map(Arc::new)
			.map_err(|err| {
				eprintln!("Failed to connect to axiom: {err}");
				err
			})
			.ok();

		Arc::new(Self { dataset, client })
	}
}

impl io::Write for &Writer {
	fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
		let Some(client) = self.client.as_ref().map(Arc::clone) else {
			return Ok(0);
		};

		let dataset = self.dataset.clone();
		let json_data = serde_json::from_slice::<JsonValue>(buf)
			.map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;

		if json_data["fields"]["skip_axiom"] == JsonValue::Bool(true) {
			return Ok(0);
		}

		task::spawn(async move {
			if let Err(err) = client.ingest(dataset, [json_data]).await {
				audit_error!(skip_axiom = true, %err, "failed to send logs to axiom");
			}
		});

		Ok(buf.len())
	}

	fn flush(&mut self) -> io::Result<()> {
		Ok(())
	}
}