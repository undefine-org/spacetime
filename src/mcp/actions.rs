//! The MCP action registry — the SINGLE source of truth for `mcp-action` routing.
//!
//! A browser click on a region-mounted page POSTs one `mcp-action` envelope to the
//! ONE signal sink (`/__mcp/signal/{instance}`). Where that event goes is decided
//! HERE, by the action's declared [`Audience`]:
//!
//! - [`Audience::Server`] — the server resolves it synchronously at the sink
//!   (open an environment, open a tab, refresh the snapshot), updates the
//!   instance's `input.json`, and repaints. The agent is never involved and the
//!   event is NOT delivered to `st_await`.
//! - [`Audience::Agent`] — the event is for the agent to read (a kit choice /
//!   confirm / submit, or a page-declared agent-control action). It is recorded
//!   and delivered through `st_await`; the server does not resolve it.
//!
//! Before this registry (PLAN-045) the routing match was DUPLICATED in two
//! places (`live::dispatch_signal_action` at the sink and a now-deleted
//! `tools::dispatch_mcp_action` on the await side) and there was no audience
//! concept at all — every event, including server-handled navigation, sat in the
//! agent's await queue (a nav click could even satisfy an unrelated armed
//! await). Now the sink is the ONE dispatcher; `st_await` delivers only
//! Agent-audience events. This module is the single enumeration both the sink
//! and the await filter consult; adding an action is a row in `REGISTRY`, never
//! a new match arm in two files (AGENTS.md: variants are DATA).

/// Who a given `mcp-action` is addressed to.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Audience {
    /// The server resolves it synchronously at the signal sink; not awaitable.
    Server,
    /// The agent reads it via `st_await`; the server does not resolve it.
    Agent,
}

impl Audience {
    /// The stable wire label (surfaced in event JSON + the comms drawer).
    pub fn label(self) -> &'static str {
        match self {
            Audience::Server => "server",
            Audience::Agent => "agent",
        }
    }
}

/// One registered action: its wire name + who it is addressed to. Server actions
/// are resolved by the caller's context-specific resolver (the sink has a
/// `WorkbenchSession`, the await side has an `McpState`) — the registry owns the
/// ROUTING decision, the call sites own the side effect.
#[derive(Clone, Copy, Debug)]
pub struct ActionSpec {
    pub action: &'static str,
    pub audience: Audience,
}

/// The built-in action registry — the ONLY place actions are enumerated.
///
/// Server actions cover everything the workbench navigation emits today; Agent
/// actions cover the PLAN-043 interaction kit (picker/confirm/form). An action
/// absent from this table is unknown: it is rejected at the sink unless the
/// mounted instance declares itself agent-control (see [`route`]).
const REGISTRY: &[ActionSpec] = &[
    // Workbench navigation / inspection — server-resolvable, never wakes the agent.
    ActionSpec {
        action: "mcp_refresh",
        audience: Audience::Server,
    },
    ActionSpec {
        action: "st_workbench_refresh",
        audience: Audience::Server,
    },
    ActionSpec {
        action: "st_env_open",
        audience: Audience::Server,
    },
    ActionSpec {
        action: "st_tab_open",
        audience: Audience::Server,
    },
    // M2 (PLAN-046): compose a registered function INTO the originating host's
    // stage region — the gallery-click → mount-on-stage path. Server-audience: the
    // sink resolves it synchronously against `event.instance_id` (the host) and
    // repaints, no agent wake. `target` carries the function id/name; optional
    // `region` defaults to "stage".
    ActionSpec {
        action: "mcp_compose",
        audience: Audience::Server,
    },
    // M4 (PLAN-046): set a param override on a composed guest. Server-audience:
    // resolved against the originating host's `region_mount_input`, repainted via
    // the bundle poll → registerBundle re-seeds the guest root. `target` = param
    // name, `value` = new value, optional `region` (default "stage").
    ActionSpec {
        action: "mcp_param",
        audience: Audience::Server,
    },
    // PLAN-051 B4: edit ONE composed-node's named arg in the guest SOURCE,
    // addressed by the node's G2 invoke_span. `target` = param name, `value` =
    // new value, `span_start`/`span_end` = the invocation byte span, optional
    // `region` (default "stage"). Patches the source + recompiles (the per-
    // instance inspector WRITE on the G2 rail).
    ActionSpec {
        action: "mcp_node_param",
        audience: Audience::Server,
    },
    // M6 (PLAN-046): save edited source for the guest composed on the host's
    // stage — re-puts the function (hot-swap shows it live). Server-audience:
    // resolved against the host's stage region_mount. `value` = new source.
    ActionSpec {
        action: "mcp_source_edit",
        audience: Audience::Server,
    },
    // PLAN-049 W1.S1: unmount a live instance (remove from store + clear any
    // host region it was composed into). Server-audience: resolved at the sink
    // against `target` (the instance id). The instance/frame cards drive it.
    ActionSpec {
        action: "mcp_unmount",
        audience: Audience::Server,
    },
    // PLAN-049 W1.S2: clear a host region (default "stage") — drop the composed
    // guest + its param overrides so the region returns to its empty-state.
    ActionSpec {
        action: "mcp_stage_clear",
        audience: Audience::Server,
    },
    // PLAN-049 W1.S3: close an opened environment (drop it + its tabs) and delete
    // a registered function. Both refuse with a clear message while a dependent
    // is still mounted (unmount first). Server-audience; driven by card buttons.
    ActionSpec {
        action: "mcp_env_close",
        audience: Audience::Server,
    },
    ActionSpec {
        action: "mcp_fn_delete",
        audience: Audience::Server,
    },
    // PLAN-049 W2 (COLLAPSE): close a tab — drop the tab record AND delete its
    // env-origin function (both halves of the collapsed object). Refuses while
    // the function is mounted (delegates to the same delete guard).
    ActionSpec {
        action: "mcp_tab_close",
        audience: Audience::Server,
    },
    // PLAN-049 W3: the "+" creation surface — register inline source as a new
    // Agent-origin function (gallery-visible, composable) + compose it onto the
    // stage. The human STARTS a frame, not just rearranges agent-seeded ones.
    // payload.values carries { create_name, create_source } (mcp-submit collect).
    ActionSpec {
        action: "mcp_create",
        audience: Audience::Server,
    },
    // PLAN-043 interaction kit — the human's answer, addressed to the agent.
    ActionSpec {
        action: "kit-choice",
        audience: Audience::Agent,
    },
    ActionSpec {
        action: "kit-confirm",
        audience: Audience::Agent,
    },
    ActionSpec {
        action: "kit-cancel",
        audience: Audience::Agent,
    },
    ActionSpec {
        action: "kit-submit",
        audience: Audience::Agent,
    },
];

/// Look up an action's spec by its wire name.
pub fn lookup(action: &str) -> Option<&'static ActionSpec> {
    REGISTRY.iter().find(|spec| spec.action == action)
}

/// The routing verdict for an `mcp-action` on a given instance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Route {
    /// A registered server action — resolve synchronously at the sink.
    Server,
    /// Addressed to the agent (registered Agent action, or a page-declared
    /// agent-control action) — record + deliver via `st_await`.
    Agent,
    /// Unknown action with no agent-control declaration — reject at the sink.
    Reject,
}

/// Decide where an `mcp-action` goes.
///
/// `agent_control` is the mounted instance's self-declaration: a page that
/// represents an agent-control interface routes its OWN (otherwise-unknown)
/// actions to the agent instead of being rejected. Built-in registry actions
/// always win (a kit's `kit-choice` is Agent regardless; `st_env_open` is
/// Server regardless), so a page cannot hijack a built-in action's routing.
pub fn route(action: &str, agent_control: bool) -> Route {
    match lookup(action) {
        Some(spec) => match spec.audience {
            Audience::Server => Route::Server,
            Audience::Agent => Route::Agent,
        },
        None => {
            if agent_control {
                Route::Agent
            } else {
                Route::Reject
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_server_actions_route_to_server() {
        for action in [
            "mcp_refresh",
            "st_workbench_refresh",
            "st_env_open",
            "st_tab_open",
            "mcp_compose",
            "mcp_param",
            "mcp_node_param",
            "mcp_source_edit",
            "mcp_unmount",
            "mcp_stage_clear",
            "mcp_env_close",
            "mcp_fn_delete",
            "mcp_tab_close",
            "mcp_create",
        ] {
            assert_eq!(
                route(action, false),
                Route::Server,
                "{action} should be Server"
            );
            // agent_control must NOT override a built-in server action.
            assert_eq!(
                route(action, true),
                Route::Server,
                "{action} stays Server even on an agent-control page"
            );
        }
    }

    #[test]
    fn builtin_kit_actions_route_to_agent() {
        for action in ["kit-choice", "kit-confirm", "kit-cancel", "kit-submit"] {
            assert_eq!(
                route(action, false),
                Route::Agent,
                "{action} should be Agent"
            );
        }
    }

    #[test]
    fn unknown_action_is_rejected_unless_agent_control() {
        assert_eq!(route("totally-custom", false), Route::Reject);
        // A page that declares itself agent-control routes its own unknown action
        // to the agent rather than having it rejected.
        assert_eq!(route("totally-custom", true), Route::Agent);
    }

    #[test]
    fn lookup_is_the_single_enumeration() {
        // Sanity: every registered action resolves through the one lookup.
        assert!(lookup("st_env_open").is_some());
        assert!(lookup("kit-choice").is_some());
        assert!(lookup("nope").is_none());
    }
}
