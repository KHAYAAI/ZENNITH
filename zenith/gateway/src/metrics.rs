use prometheus::{IntCounter, IntGauge, Registry};

pub struct Metrics {
    pub requests_total: IntCounter,
    pub errors_total: IntCounter,
    pub canisters_deployed: IntCounter,
    pub proofs_verified: IntCounter,
    pub proofs_submitted: IntCounter,
    pub provers_registered: IntCounter,
    pub active_canisters: IntGauge,
    pub pending_proofs: IntGauge,
}

impl Metrics {
    pub fn new(registry: &Registry) -> Result<Self, Box<dyn std::error::Error>> {
        let requests_total = IntCounter::new("zenith_requests_total", "Total requests")?;
        let errors_total = IntCounter::new("zenith_errors_total", "Total errors")?;
        let canisters_deployed = IntCounter::new("zenith_canisters_deployed_total", "Total canisters deployed")?;
        let proofs_verified = IntCounter::new("zenith_proofs_verified_total", "Total proofs verified")?;
        let proofs_submitted = IntCounter::new("zenith_proofs_submitted_total", "Total proofs submitted")?;
        let provers_registered = IntCounter::new("zenith_provers_registered_total", "Total provers registered")?;
        let active_canisters = IntGauge::new("zenith_active_canisters", "Active canisters")?;
        let pending_proofs = IntGauge::new("zenith_pending_proofs", "Pending proofs")?;

        registry.register(Box::new(requests_total.clone()))?;
        registry.register(Box::new(errors_total.clone()))?;
        registry.register(Box::new(canisters_deployed.clone()))?;
        registry.register(Box::new(proofs_verified.clone()))?;
        registry.register(Box::new(proofs_submitted.clone()))?;
        registry.register(Box::new(provers_registered.clone()))?;
        registry.register(Box::new(active_canisters.clone()))?;
        registry.register(Box::new(pending_proofs.clone()))?;

        Ok(Metrics {
            requests_total,
            errors_total,
            canisters_deployed,
            proofs_verified,
            proofs_submitted,
            provers_registered,
            active_canisters,
            pending_proofs,
        })
    }
}

pub fn gather_metrics() -> Result<String, Box<dyn std::error::Error>> {
    // Return simple Prometheus text format metrics
    let registry = prometheus::gather();
    let mut output = String::new();
    for family in registry {
        for metric in family.get_metric() {
            if metric.has_counter() {
                let value = metric.get_counter().get_value();
                output.push_str(&format!(
                    "{} {}\n",
                    family.get_name(),
                    value
                ));
            } else if metric.has_gauge() {
                let value = metric.get_gauge().get_value();
                output.push_str(&format!(
                    "{} {}\n",
                    family.get_name(),
                    value
                ));
            }
        }
    }
    Ok(output)
}
