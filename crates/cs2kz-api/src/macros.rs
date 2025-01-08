/// Conditionally includes items or statements if [taskdump] support is enabled.
///
/// [taskdump]: tokio::runtime::Handle::dump
macro_rules! cfg_taskdump {
    ($(let $var:ident $(: $ty:ty)? = $expr:expr;)*) => {
        $(
            #[cfg(all(
                target_os = "linux",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                ),
            ))]
            let $var $(: $ty)? = $expr;
        )*
    };
    ($($item:item)*) => {
        $(
            #[cfg(all(
                target_os = "linux",
                any(
                    target_arch = "x86",
                    target_arch = "x86_64",
                    target_arch = "aarch64",
                ),
            ))]
            $item
        )*
    };
}
