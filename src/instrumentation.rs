//! Performance instrumentation module for latency profiling.
//!
//! This module provides zero-overhead compile-time instrumentation using the tracing crate.
//! Enable with `--features instrumentation` to collect timing statistics.

use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::Mutex;
use std::time::Instant;
use thread_local::ThreadLocal;
use tracing::span;
use tracing_subscriber::layer::{Context, SubscriberExt};
use tracing_subscriber::registry::LookupSpan;
use tracing_subscriber::Layer;

/// Thread-local timing data to minimize cross-thread contention.
/// Each thread has its own HashMap with no lock during hot path access.
static THREAD_TIMING_DATA: Lazy<ThreadLocal<Mutex<HashMap<String, (u64, u64)>>>> =
    Lazy::new(|| ThreadLocal::new());

/// Custom tracing layer that collects timing statistics for each instrumented span.
struct TimingLayer;

impl<S> Layer<S> for TimingLayer
where
    S: tracing::Subscriber + for<'a> LookupSpan<'a>,
{
    fn on_enter(&self, id: &span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            extensions.insert(Instant::now());
        }
    }

    fn on_exit(&self, id: &span::Id, ctx: Context<'_, S>) {
        if let Some(span) = ctx.span(id) {
            let mut extensions = span.extensions_mut();
            if let Some(start) = extensions.remove::<Instant>() {
                let elapsed = start.elapsed();
                let name = span.name().to_string();

                // Update thread-local data (mutex only locks within same thread)
                let cell = THREAD_TIMING_DATA.get_or(|| Mutex::new(HashMap::new()));
                let mut data = cell.lock().unwrap();
                let entry = data.entry(name).or_insert((0, 0));
                entry.0 += 1;
                entry.1 += elapsed.as_nanos() as u64;
            }
        }
    }
}

/// Initialize tracing subscriber with timing layer.
///
/// Respects RUST_LOG environment variable:
/// - "off" or unset: Silent mode, only collect timing data
/// - Any other value: Verbose mode, print span events
pub fn init_tracing() {
    use tracing_subscriber::EnvFilter;

    let timing_layer = TimingLayer;
    let env_filter = std::env::var("RUST_LOG").unwrap_or_else(|_| "off".to_string());

    if env_filter == "off" || env_filter.is_empty() {
        // Silent mode: only collect timing data
        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::new("trace")) // Enable all spans for timing
            .with(timing_layer);

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    } else {
        // Verbose mode: also print span events
        use tracing_subscriber::fmt;

        let fmt_layer = fmt::layer().with_target(false).with_level(false).compact();

        let subscriber = tracing_subscriber::registry()
            .with(EnvFilter::from_default_env())
            .with(timing_layer)
            .with(fmt_layer);

        tracing::subscriber::set_global_default(subscriber)
            .expect("Failed to set tracing subscriber");
    }
}

/// Print aggregated timing statistics from all threads.
///
/// Output format:
/// - Function name
/// - Total call count
/// - Total time in milliseconds
/// - Average time per call in microseconds
pub fn print_timing_statistics() {
    // Aggregate data from all threads
    let mut aggregated: HashMap<String, (u64, u64)> = HashMap::new();

    for thread_data in THREAD_TIMING_DATA.iter() {
        let data = thread_data.lock().unwrap();
        for (name, (count, nanos)) in data.iter() {
            let entry = aggregated.entry(name.clone()).or_insert((0, 0));
            entry.0 += count;
            entry.1 += nanos;
        }
    }

    if aggregated.is_empty() {
        eprintln!("\nNo timing data collected.");
        return;
    }

    // Collect and sort by total time
    let mut entries: Vec<_> = aggregated.into_iter().collect();
    entries.sort_by_key(|(_, (_, total))| std::cmp::Reverse(*total));

    eprintln!("\n{:=<80}", "");
    eprintln!("Latency Statistics (sorted by total time)");
    eprintln!("{:=<80}", "");
    eprintln!(
        "{:<40} {:>12} {:>12} {:>12}",
        "Function", "Calls", "Total (ms)", "Avg (µs)"
    );
    eprintln!("{:-<80}", "");

    let mut grand_total_nanos = 0u64;

    for (name, (count, total_nanos)) in &entries {
        if *count > 0 {
            let total_ms = *total_nanos as f64 / 1_000_000.0;
            let avg_micros = (*total_nanos as f64 / *count as f64) / 1_000.0;
            grand_total_nanos += total_nanos;

            eprintln!(
                "{:<40} {:>12} {:>12.2} {:>12.2}",
                name, count, total_ms, avg_micros
            );
        }
    }

    eprintln!("{:-<80}", "");
    eprintln!(
        "Total instrumented time: {:.2} ms\n",
        grand_total_nanos as f64 / 1_000_000.0
    );
}
