//! Metrics and observability module

use metrics::{counter, gauge, histogram};
use std::time::Instant;

/// Metrics recorder for the processor
#[derive(Debug, Clone)]
pub struct MetricsRecorder {
    enabled: bool,
}

impl MetricsRecorder {
    /// Create a new metrics recorder
    pub fn new(enabled: bool) -> Self {
        Self { enabled }
    }

    /// Record a processed record
    pub fn record_processed(&self, duration_ms: u64, success: bool) {
        if !self.enabled {
            return;
        }

        counter!("records_processed_total").increment(1);
        
        if success {
            counter!("records_processed_success").increment(1);
        } else {
            counter!("records_processed_failed").increment(1);
        }

        histogram!("record_processing_duration_ms").record(duration_ms as f64);
    }

    /// Record batch processing
    pub fn record_batch_processed(&self, count: usize, duration_ms: u64) {
        if !self.enabled {
            return;
        }

        counter!("batches_processed_total").increment(1);
        histogram!("batch_size").record(count as f64);
        histogram!("batch_processing_duration_ms").record(duration_ms as f64);
    }

    /// Update active tasks gauge
    pub fn update_active_tasks(&self, count: u64) {
        if !self.enabled {
            return;
        }

        gauge!("active_tasks").set(count as f64);
    }

    /// Record storage operation
    pub fn record_storage_operation(&self, operation: &str, duration_ms: u64, success: bool) {
        if !self.enabled {
            return;
        }

        counter!("storage_operations_total", "operation" => operation.to_string())
            .increment(1);
        
        if success {
            counter!("storage_operations_success", "operation" => operation.to_string())
                .increment(1);
        } else {
            counter!("storage_operations_failed", "operation" => operation.to_string())
                .increment(1);
        }

        histogram!(
            "storage_operation_duration_ms",
            "operation" => operation.to_string()
        )
        .record(duration_ms as f64);
    }

    /// Record validation
    pub fn record_validation(&self, rule: &str, success: bool) {
        if !self.enabled {
            return;
        }

        counter!("validations_total", "rule" => rule.to_string()).increment(1);
        
        if success {
            counter!("validations_success", "rule" => rule.to_string()).increment(1);
        } else {
            counter!("validations_failed", "rule" => rule.to_string()).increment(1);
        }
    }

    /// Record transform application
    pub fn record_transform(&self, name: &str, duration_ms: u64, success: bool) {
        if !self.enabled {
            return;
        }

        counter!("transforms_applied_total", "transform" => name.to_string())
            .increment(1);
        
        if success {
            counter!("transforms_applied_success", "transform" => name.to_string())
                .increment(1);
        } else {
            counter!("transforms_applied_failed", "transform" => name.to_string())
                .increment(1);
        }

        histogram!(
            "transform_duration_ms",
            "transform" => name.to_string()
        )
        .record(duration_ms as f64);
    }

    /// Record error
    pub fn record_error(&self, error_type: &str) {
        if !self.enabled {
            return;
        }

        counter!("errors_total", "type" => error_type.to_string()).increment(1);
    }
}

/// Timer for measuring operation duration
pub struct Timer {
    start: Instant,
    name: String,
    recorder: MetricsRecorder,
}

impl Timer {
    /// Create a new timer
    pub fn new(name: impl Into<String>, recorder: MetricsRecorder) -> Self {
        Self {
            start: Instant::now(),
            name: name.into(),
            recorder,
        }
    }

    /// Stop the timer and record the duration
    pub fn stop(self) -> u64 {
        let duration_ms = self.start.elapsed().as_millis() as u64;
        histogram!("operation_duration_ms", "operation" => self.name.clone())
            .record(duration_ms as f64);
        duration_ms
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_recorder() {
        let recorder = MetricsRecorder::new(true);
        recorder.record_processed(100, true);
        recorder.record_batch_processed(10, 500);
        recorder.update_active_tasks(5);
    }

    #[test]
    fn test_timer() {
        let recorder = MetricsRecorder::new(true);
        let timer = Timer::new("test_operation", recorder);
        std::thread::sleep(std::time::Duration::from_millis(10));
        let duration = timer.stop();
        assert!(duration >= 10);
    }
}
