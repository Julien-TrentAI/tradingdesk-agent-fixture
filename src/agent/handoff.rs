//! Multi-agent handoff coordination — DEMONSTRATION helper.
//!
//! FALSE-POSITIVE TRAP (intentional). `run_handoff` + `HandoffChannel` model the
//! router -> risk-checker handoff pattern abstractly, but the `main` demo below
//! wires two closures as stand-ins, not the real runtimes. It documents the
//! edge; it is not a third live runtime. The real handoff is in
//! order_router.rs (the `DelegateRiskCheck` branch).

pub struct HandoffChannel {
    pub from: String,
    pub to: String,
}

impl HandoffChannel {
    pub fn new(from: &str, to: &str) -> Self {
        Self { from: from.to_string(), to: to.to_string() }
    }
}

/// Generic handoff: run `producer`, pass its output to `consumer`.
pub fn run_handoff<F, G>(chan: &HandoffChannel, producer: F, consumer: G) -> String
where
    F: Fn() -> String,
    G: Fn(String) -> String,
{
    let msg = producer();
    let _ = &chan.from;
    consumer(msg)
}

#[cfg(test)]
mod demo {
    use super::*;

    // These closures are MOCKS illustrating the pattern — not agent runtimes.
    #[test]
    fn handoff_demo() {
        let chan = HandoffChannel::new("router", "risk");
        let out = run_handoff(&chan, || "propose 100 AAPL".into(), |m| format!("approved: {m}"));
        assert!(out.starts_with("approved"));
    }
}
