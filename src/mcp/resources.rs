use serde_json::{Value, json};

use super::live::{self, instance_events_uri};
use super::state::McpState;

const ENVIRONMENTS_URI: &str = "spacetime://mcp/environments";
const FUNCTIONS_URI: &str = "spacetime://mcp/functions";
const INSTANCES_URI: &str = "spacetime://mcp/instances";
const COMMENTS_URI_PREFIX: &str = "comments://";

pub fn templates() -> Value {
    json!({
        "resourceTemplates": [
            {
                "uriTemplate": "spacetime://mcp/environments",
                "name": "Spacetime environments",
                "title": "Spacetime Environments",
                "description": "Discovered and open Spacetime MCP environments.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "spacetime://mcp/environments/{env_id}/tabs",
                "name": "Spacetime environment tabs",
                "title": "Spacetime Environment Tabs",
                "description": "Tabs opened inside one Spacetime MCP environment.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "spacetime://mcp/functions",
                "name": "Spacetime functions",
                "title": "Spacetime Functions",
                "description": "Registered Spacetime MCP live functions.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "spacetime://mcp/instances",
                "name": "Spacetime instances",
                "title": "Spacetime Mounted Instances",
                "description": "Mounted Spacetime MCP function instances.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "spacetime://instances/{instance_id}/events",
                "name": "Spacetime instance events",
                "title": "Spacetime Instance Events",
                "description": "Event history emitted by a mounted Spacetime MCP live instance.",
                "mimeType": "application/json"
            },
            {
                "uriTemplate": "comments://{env_id}",
                "name": "Spacetime environment comments",
                "title": "Spacetime Environment Comments",
                "description": "Attach project conversations to agent context or watch them; use st_comments_list when selecting comments with filters.",
                "mimeType": "application/json"
            }
        ]
    })
}

pub fn list(state: &McpState) -> Value {
    let mut resources = vec![
        resource(
            ENVIRONMENTS_URI,
            "environments",
            "Spacetime Environments",
            "Discovered and open Spacetime environments",
        ),
        resource(
            FUNCTIONS_URI,
            "functions",
            "Spacetime Functions",
            "Registered live Spacetime functions",
        ),
        resource(
            INSTANCES_URI,
            "instances",
            "Spacetime Instances",
            "Mounted live Spacetime function instances",
        ),
    ];

    for env in state.environments() {
        resources.push(resource(
            &env_tabs_uri(&env.id),
            &format!("{} tabs", env.id),
            &format!("{} tabs", env.title),
            "Tabs opened in this Spacetime environment",
        ));
        resources.push(resource(
            &comments_uri(&env.id),
            &format!("{} comments", env.id),
            &format!("{} comments", env.title),
            "Attach project conversations to context or watch them; use st_comments_list to filter work.",
        ));
    }

    if let Some(live) = state.live_if_started() {
        for instance in live.instances() {
            resources.push(resource(
                &instance_events_uri(&instance.id),
                &format!("{} events", instance.id),
                &format!("{} events", instance.title),
                "Events emitted by this mounted function instance",
            ));
        }
    }

    json!({ "resources": resources })
}

pub fn read(uri: &str, state: &mut McpState) -> Result<Value, String> {
    let body = if uri == ENVIRONMENTS_URI {
        json!({
            "schema_version": 1,
            "discovered": state.discovered_environments_json(),
            "open": state.open_environments_json(),
        })
    } else if let Some(env_id) = parse_env_tabs_uri(uri) {
        let env = state
            .environment(env_id)
            .ok_or_else(|| format!("no such environment: {env_id}"))?;
        json!({
            "schema_version": 1,
            "env": {
                "id": env.id,
                "title": env.title,
                "root": env.root.display().to_string(),
            },
            "tabs": state.tabs_json(env_id),
        })
    } else if let Some(env_id) = parse_comments_uri(uri) {
        let root = state
            .environment(env_id)
            .map(|env| env.root)
            .ok_or_else(|| format!("no such environment: {env_id}; call st_env_open first"))?;
        super::tools::comment_index_json(&root, state)?
    } else if uri == FUNCTIONS_URI {
        let functions = state
            .live_if_started()
            .map(|live| {
                live.functions()
                    .into_iter()
                    .map(|f| f.to_json())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({ "schema_version": 1, "functions": functions })
    } else if uri == INSTANCES_URI {
        let instances = state
            .live_if_started()
            .map(|live| {
                live.instances()
                    .into_iter()
                    .map(|i| i.to_json())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        json!({ "schema_version": 1, "instances": instances })
    } else if let Some(instance_id) = parse_instance_events_uri(uri) {
        let live = state
            .live_if_started()
            .ok_or_else(|| "live server has not started".to_string())?;
        live.read_instance_events(instance_id)
            .ok_or_else(|| format!("no such instance: {instance_id}"))?
    } else {
        return Err(format!("unknown resource URI: {uri}"));
    };

    Ok(json!({
        "contents": [
            {
                "uri": uri,
                "mimeType": "application/json",
                "text": serde_json::to_string_pretty(&body).unwrap_or_else(|_| body.to_string())
            }
        ]
    }))
}

pub fn workbench_snapshot(state: &McpState) -> Value {
    let functions = state
        .live_if_started()
        .map(|live| {
            live.functions()
                .into_iter()
                .map(|f| f.to_json())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let instances = state
        .live_if_started()
        .map(|live| {
            live.instances()
                .into_iter()
                .map(|i| i.to_json())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let events = state
        .live_if_started()
        .map(|live| {
            live.instances()
                .into_iter()
                .flat_map(|instance| instance.events.into_iter().map(|event| event.to_json()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let tabs = state.all_tabs_json();

    json!({
        "schema_version": 1,
        "environments": {
            "discovered": state.discovered_environments_json(),
            "open": state.open_environments_json(),
        },
        "tabs": tabs,
        "functions": functions,
        "contract_params": super::live::flatten_contract_params(&functions),
        "instances": instances,
        "events": events,
    })
}

pub fn validate_uri(uri: &str, state: &mut McpState) -> Result<(), String> {
    read(uri, state).map(|_| ())
}

pub fn events_uri(instance_id: &str) -> String {
    live::instance_events_uri(instance_id)
}

pub fn env_tabs_uri(env_id: &str) -> String {
    format!("spacetime://mcp/environments/{env_id}/tabs")
}

pub fn comments_uri(env_id: &str) -> String {
    format!("{COMMENTS_URI_PREFIX}{env_id}")
}

fn resource(uri: &str, name: &str, title: &str, description: &str) -> Value {
    json!({
        "uri": uri,
        "name": name,
        "title": title,
        "description": description,
        "mimeType": "application/json",
    })
}

fn parse_env_tabs_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix("spacetime://mcp/environments/")?
        .strip_suffix("/tabs")
}

fn parse_comments_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix(COMMENTS_URI_PREFIX)
        .filter(|env_id| !env_id.is_empty())
}

fn parse_instance_events_uri(uri: &str) -> Option<&str> {
    uri.strip_prefix("spacetime://instances/")?
        .strip_suffix("/events")
}
