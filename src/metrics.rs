//! Lightweight metrics facade for the Gemini SDK.
//!
//! The SDK exposes a [`MetricsRecorder`] trait so callers can observe request,
//! retry, parse, and attestation events without depending directly on
//! OpenTelemetry types. By default no recorder is configured and the overhead
//! is zero. The `metrics` feature enables an OpenTelemetry-backed
//! implementation.

use std::fmt::Debug;
use std::sync::Arc;
use std::time::Duration;

/// A low-cardinality, object-safe metrics sink.
///
/// Implementors receive counters and histograms with string-valued attributes.
/// Attribute values must be low-cardinality; the SDK never emits user content,
/// prompts, or raw tool arguments.
pub trait MetricsRecorder: Send + Sync {
    /// Increments a counter by one.
    fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]);

    /// Records a histogram observation.
    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]);
}

impl<T: MetricsRecorder + ?Sized> MetricsRecorder for Arc<T> {
    fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]) {
        (**self).increment_counter(name, attributes);
    }

    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]) {
        (**self).record_histogram(name, value, attributes);
    }
}

/// A no-op recorder used when no metrics are configured.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoOpMetricsRecorder;

impl MetricsRecorder for NoOpMetricsRecorder {
    fn increment_counter(&self, _name: &str, _attributes: &[(&str, &str)]) {}

    fn record_histogram(&self, _name: &str, _value: Duration, _attributes: &[(&str, &str)]) {}
}

/// OpenTelemetry-backed metrics recorder.
///
/// Available only when the `metrics` feature is enabled. Instruments are
/// created lazily and cached by name.
#[cfg(feature = "metrics")]
pub struct OpenTelemetryRecorder {
    meter: opentelemetry::metrics::Meter,
}

#[cfg(feature = "metrics")]
impl OpenTelemetryRecorder {
    /// Creates a recorder from an OpenTelemetry `Meter`.
    pub fn new(meter: opentelemetry::metrics::Meter) -> Self {
        Self { meter }
    }
}

#[cfg(feature = "metrics")]
impl MetricsRecorder for OpenTelemetryRecorder {
    fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]) {
        let kv: Vec<opentelemetry::KeyValue> = attributes
            .iter()
            .map(|(k, v)| opentelemetry::KeyValue::new(k.to_string(), v.to_string()))
            .collect();
        self.meter.u64_counter(name.to_string()).build().add(1, &kv);
    }

    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]) {
        let kv: Vec<opentelemetry::KeyValue> = attributes
            .iter()
            .map(|(k, v)| opentelemetry::KeyValue::new(k.to_string(), v.to_string()))
            .collect();
        self.meter
            .f64_histogram(name.to_string())
            .build()
            .record(value.as_secs_f64(), &kv);
    }
}

#[cfg(feature = "metrics")]
impl Debug for OpenTelemetryRecorder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("OpenTelemetryRecorder").finish_non_exhaustive()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Mutex;

    type CounterRecord = (String, Vec<(String, String)>);
    type HistogramRecord = (String, f64, Vec<(String, String)>);

    #[derive(Debug, Default)]
    struct CountingRecorder {
        counters: Mutex<Vec<CounterRecord>>,
        histograms: Mutex<Vec<HistogramRecord>>,
        counter_calls: AtomicUsize,
        histogram_calls: AtomicUsize,
    }

    impl MetricsRecorder for CountingRecorder {
        fn increment_counter(&self, name: &str, attributes: &[(&str, &str)]) {
            self.counter_calls.fetch_add(1, Ordering::SeqCst);
            let attrs = attributes.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            self.counters.lock().unwrap().push((name.to_string(), attrs));
        }

        fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]) {
            self.histogram_calls.fetch_add(1, Ordering::SeqCst);
            let attrs = attributes.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
            self.histograms
                .lock()
                .unwrap()
                .push((name.to_string(), value.as_secs_f64(), attrs));
        }
    }

    #[test]
    fn no_op_recorder_does_nothing() {
        use crate::constants::tracing_names::{METRIC_REQUESTS, METRIC_REQUEST_LATENCY, STATUS};

        let recorder = NoOpMetricsRecorder;
        recorder.increment_counter(METRIC_REQUESTS, &[(STATUS, "ok")]);
        recorder.record_histogram(METRIC_REQUEST_LATENCY, Duration::from_millis(10), &[]);
    }

    #[test]
    fn counting_recorder_records_counters_and_histograms() {
        use crate::constants::tracing_names::{
            METRIC_REQUESTS, METRIC_REQUEST_LATENCY, OPERATION, STATUS,
        };

        let recorder = CountingRecorder::default();
        recorder.increment_counter(METRIC_REQUESTS, &[(STATUS, "ok")]);
        recorder.record_histogram(
            METRIC_REQUEST_LATENCY,
            Duration::from_secs_f64(0.05),
            &[(OPERATION, "generate")],
        );

        assert_eq!(recorder.counter_calls.load(Ordering::SeqCst), 1);
        assert_eq!(recorder.histogram_calls.load(Ordering::SeqCst), 1);

        let counters = recorder.counters.lock().unwrap();
        assert_eq!(counters[0].0, METRIC_REQUESTS);
        assert_eq!(counters[0].1, vec![(STATUS.to_string(), "ok".to_string())]);

        let histograms = recorder.histograms.lock().unwrap();
        assert_eq!(histograms[0].0, METRIC_REQUEST_LATENCY);
        assert!((histograms[0].1 - 0.05).abs() < f64::EPSILON);
    }
}
