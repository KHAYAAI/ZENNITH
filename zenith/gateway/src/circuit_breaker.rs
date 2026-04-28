use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use std::sync::Arc;
use tracing::{info, warn};

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,      // Normal operation
    Open,        // Failing, reject requests
    HalfOpen,    // Testing if recovered
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    last_failure_time: Arc<RwLock<Option<Instant>>>,
    failure_threshold: u32,
    timeout: Duration,
    half_open_max_attempts: u32,
    half_open_attempts: Arc<RwLock<u32>>,
}

#[derive(Debug)]
pub enum CircuitBreakerError {
    CircuitOpen,
    CallFailed(String),
}

impl std::fmt::Display for CircuitBreakerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CircuitBreakerError::CircuitOpen => write!(f, "Circuit breaker is open"),
            CircuitBreakerError::CallFailed(e) => write!(f, "Call failed: {}", e),
        }
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout_secs: u64) -> Self {
        CircuitBreaker {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            last_failure_time: Arc::new(RwLock::new(None)),
            failure_threshold,
            timeout: Duration::from_secs(timeout_secs),
            half_open_max_attempts: 3,
            half_open_attempts: Arc::new(RwLock::new(0)),
        }
    }

    pub async fn call<F, T>(&self, f: F) -> Result<T, CircuitBreakerError>
    where
        F: std::future::Future<Output = Result<T, String>>,
    {
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                // Check if we should move to HalfOpen
                if let Some(last_failure) = *self.last_failure_time.read().await {
                    if last_failure.elapsed() > self.timeout {
                        info!("Circuit breaker transitioning to HalfOpen");
                        *self.state.write().await = CircuitState::HalfOpen;
                        *self.half_open_attempts.write().await = 0;
                    } else {
                        return Err(CircuitBreakerError::CircuitOpen);
                    }
                } else {
                    return Err(CircuitBreakerError::CircuitOpen);
                }
            }
            CircuitState::HalfOpen => {
                // Allow limited attempts
                let attempts = *self.half_open_attempts.read().await;
                if attempts >= self.half_open_max_attempts {
                    return Err(CircuitBreakerError::CircuitOpen);
                }
            }
            CircuitState::Closed => {}
        }

        // Execute the function
        match f.await {
            Ok(result) => {
                // Success: reset failure count
                *self.failure_count.write().await = 0;
                *self.last_failure_time.write().await = None;
                *self.half_open_attempts.write().await = 0;

                if *self.state.read().await == CircuitState::HalfOpen {
                    info!("Circuit breaker transitioning back to Closed");
                    *self.state.write().await = CircuitState::Closed;
                }

                Ok(result)
            }
            Err(e) => {
                // Failure: increment counter
                let mut failures = self.failure_count.write().await;
                *failures += 1;
                *self.last_failure_time.write().await = Some(Instant::now());

                if *self.state.read().await == CircuitState::HalfOpen {
                    *self.half_open_attempts.write().await += 1;
                }

                if *failures >= self.failure_threshold {
                    warn!("Circuit breaker opening after {} failures", failures);
                    *self.state.write().await = CircuitState::Open;
                }

                Err(CircuitBreakerError::CallFailed(e))
            }
        }
    }

    pub async fn get_state(&self) -> CircuitState {
        *self.state.read().await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_circuit_breaker_transitions() {
        let cb = CircuitBreaker::new(2, 1);

        // Should start closed
        assert_eq!(cb.get_state().await, CircuitState::Closed);

        // First failure
        let result: Result<i32, CircuitBreakerError> =
            cb.call(async { Err("fail".to_string()) }).await;
        assert!(result.is_err());
        assert_eq!(cb.get_state().await, CircuitState::Closed);

        // Second failure opens circuit
        let result: Result<i32, CircuitBreakerError> =
            cb.call(async { Err("fail".to_string()) }).await;
        assert!(result.is_err());
        assert_eq!(cb.get_state().await, CircuitState::Open);

        // Attempt while open should fail immediately
        let result: Result<i32, CircuitBreakerError> =
            cb.call(async { Ok(42) }).await;
        assert!(matches!(result, Err(CircuitBreakerError::CircuitOpen)));

        // Wait for timeout
        tokio::time::sleep(Duration::from_millis(1100)).await;

        // Should transition to HalfOpen and succeed
        let result: Result<i32, CircuitBreakerError> =
            cb.call(async { Ok(42) }).await;
        assert!(result.is_ok());
        assert_eq!(cb.get_state().await, CircuitState::Closed);
    }
}
