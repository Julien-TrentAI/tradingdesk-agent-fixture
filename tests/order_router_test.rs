//! Wiring contract for the agent runtimes. These tests encode the
//! by-construction inventory truth so the code and the answer key cannot
//! silently diverge: the router's tool registry holds exactly the seven desk
//! tools, and the risk sub-agent's tool set is a read-only subset of them.

use deskrouter::agent::risk_checker::RISK_TOOLS;
use deskrouter::agent::tools::TOOL_REGISTRY;

#[test]
fn registry_holds_the_seven_router_tools() {
    let names: Vec<&str> = TOOL_REGISTRY.keys().copied().collect();
    for expected in [
        "place_order",
        "cancel_order",
        "quote",
        "lookup_position",
        "read_pnl",
        "fetch_research",
        "run_analyst_shell",
    ] {
        assert!(names.contains(&expected), "missing tool: {expected}");
    }
    assert_eq!(TOOL_REGISTRY.len(), 7);
}

#[test]
fn risk_subagent_tools_are_readonly_subset() {
    for t in RISK_TOOLS {
        assert!(TOOL_REGISTRY.contains_key(t), "risk tool not in registry: {t}");
        let cap = TOOL_REGISTRY.get(t).unwrap().capability;
        assert_eq!(cap, "read", "risk sub-agent tool must be read-only: {t}");
    }
    // strict subset: the sub-agent must not hold the write/exec/egress tools.
    assert!(!RISK_TOOLS.contains(&"place_order"));
    assert!(!RISK_TOOLS.contains(&"run_analyst_shell"));
}
