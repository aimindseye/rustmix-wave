//! Reusable short-lived worker boundary.
//!
//! Heavy operations receive named stack budgets and return compact heap-owned
//! results to the main hardware-orchestration loop. Panel SPI ownership stays
//! on the main task.

use core::fmt::{self, Display};

#[derive(Debug)]
pub enum NamedWorkerError<E> {
    Start(std::io::Error),
    Panicked,
    Operation(E),
}

impl<E: Display> Display for NamedWorkerError<E> {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Start(error) => write!(formatter, "worker start failed: {error}"),
            Self::Panicked => formatter.write_str("worker panicked"),
            Self::Operation(error) => Display::fmt(error, formatter),
        }
    }
}

impl<E: Display + fmt::Debug> std::error::Error for NamedWorkerError<E> {}

pub fn run_named_worker<T, E, F>(
    name: &'static str,
    stack_bytes: usize,
    task: F,
) -> Result<T, NamedWorkerError<E>>
where
    T: Send + 'static,
    E: Display + Send + 'static,
    F: FnOnce() -> Result<T, E> + Send + 'static,
{
    log::info!(
        "rustmix-wave=worker-boundary name={name} status=starting stack-bytes={stack_bytes}"
    );
    crate::runtime_memory::log_runtime_memory(&format!("before-worker-{name}"));
    let worker = std::thread::Builder::new()
        .name(name.into())
        .stack_size(stack_bytes)
        .spawn(task)
        .map_err(|error| {
            log::warn!(
                "rustmix-wave=worker-boundary name={name} status=start-failed error={error}"
            );
            NamedWorkerError::Start(error)
        })?;
    let result = worker.join().map_err(|_| {
        log::warn!("rustmix-wave=worker-boundary name={name} status=panicked");
        NamedWorkerError::Panicked
    })?;
    crate::runtime_memory::log_runtime_memory(&format!("after-worker-{name}"));
    match result {
        Ok(value) => {
            log::info!("rustmix-wave=worker-boundary name={name} status=completed");
            Ok(value)
        }
        Err(error) => {
            log::warn!("rustmix-wave=worker-boundary name={name} status=failed error={error}");
            Err(NamedWorkerError::Operation(error))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::run_named_worker;

    #[test]
    fn returns_compact_result_from_named_short_lived_worker() {
        let result = run_named_worker("unit-worker", 16 * 1024, || Ok::<_, String>(42)).unwrap();
        assert_eq!(result, 42);
    }
}
