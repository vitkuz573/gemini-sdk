//! Integration tests for the metrics facade.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::time::Duration;

use gemini_sdk::{GeminiClient, MetricsRecorder};

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
        let attrs = attributes
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.counters.lock().unwrap().push((name.to_string(), attrs));
    }

    fn record_histogram(&self, name: &str, value: Duration, attributes: &[(&str, &str)]) {
        self.histogram_calls.fetch_add(1, Ordering::SeqCst);
        let attrs = attributes
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect();
        self.histograms
            .lock()
            .unwrap()
            .push((name.to_string(), value.as_secs_f64(), attrs));
    }
}

#[tokio::test]
async fn with_metrics_stores_recorder_in_config() {
    let recorder = CountingRecorder::default();
    let client = GeminiClient::from_cookie_header("__Secure-1PSID=a; __Secure-1PSIDCC=b")
        .unwrap()
        .with_metrics(recorder)
        .await;

    let _ = client;
}

#[test]
fn no_op_metrics_recorder_is_default_behaviour() {
    let recorder = gemini_sdk::NoOpMetricsRecorder;
    recorder.increment_counter("gemini_sdk.requests", &[("status", "ok")]);
    recorder.record_histogram("gemini_sdk.latency", Duration::from_millis(1), &[]);
}
