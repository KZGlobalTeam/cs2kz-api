use cs2kz::time::Timestamp;
use serde::de::{self, Deserialize, Deserializer};

/// Deserializes a `T` and ensures it isn't empty.
#[allow(private_bounds, reason = "implementation detail")]
pub fn deserialize_non_empty<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + IsEmpty,
{
    let value = T::deserialize(deserializer)?;

    if value.is_empty() {
        return Err(de::Error::invalid_length(0, &"1 or more"));
    }

    Ok(value)
}

/// Deserializes an <code>[Option]<T></code> and transforms empty `T`s into [`None`].
///
/// This should be used with `#[serde(default)]` if missing values should also translate to
/// [`None`].
#[allow(private_bounds, reason = "implementation detail")]
pub fn deserialize_empty_as_none<'de, D, T>(deserializer: D) -> Result<Option<T>, D::Error>
where
    D: Deserializer<'de>,
    T: Deserialize<'de> + IsEmpty,
{
    Option::<T>::deserialize(deserializer).map(|opt| opt.filter(|value| !value.is_empty()))
}

pub fn deserialize_future_timestamp_opt<'de, D>(
    deserializer: D,
) -> Result<Option<Timestamp>, D::Error>
where
    D: Deserializer<'de>,
{
    Option::<Timestamp>::deserialize(deserializer).and_then(|opt| match opt {
        None => Ok(None),
        Some(timestamp) if timestamp > Timestamp::now() => Ok(Some(timestamp)),
        Some(timestamp) => {
            Err(de::Error::custom(format_args!("timestamp {timestamp} is in the past")))
        },
    })
}

#[allow(private_bounds, reason = "implementation detail of `deserialize_non_empty`")]
trait IsEmpty {
    fn is_empty(&self) -> bool;
}

impl<T: IsEmpty> IsEmpty for Option<T> {
    fn is_empty(&self) -> bool {
        self.as_ref().is_some_and(T::is_empty)
    }
}

impl<T> IsEmpty for Vec<T> {
    fn is_empty(&self) -> bool {
        <[T]>::is_empty(&self[..])
    }
}

impl IsEmpty for String {
    fn is_empty(&self) -> bool {
        str::is_empty(&self[..])
    }
}
