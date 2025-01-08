use std::num::NonZero;

#[derive(Debug, Default, serde::Deserialize)]
#[serde(default, rename_all = "kebab-case", deny_unknown_fields)]
pub struct RuntimeConfig {
    #[serde(deserialize_with = "deserialize_non_zero")]
    pub worker_threads: Option<NonZero<usize>>,

    #[serde(deserialize_with = "deserialize_non_zero")]
    pub max_blocking_threads: Option<NonZero<usize>>,
}

fn deserialize_non_zero<'de, D>(deserializer: D) -> Result<Option<NonZero<usize>>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(<Option<usize> as serde::Deserialize<'de>>::deserialize(deserializer)?
        .and_then(NonZero::new))
}
