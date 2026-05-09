use std::sync::Arc;

use super::{make_exec, ToolDef, ToolResult};

// ─── unity_execute ───────────────────────────────────────────────────────────

pub(super) fn unity_execute() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::UNITY_EXECUTE);
    ToolDef {
        name: "unity_execute".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: make_exec(|args, ctx| {
            Box::pin(async move {
                let code = match args.get("code").and_then(|v| v.as_str()) {
                    Some(c) => c.to_string(),
                    None => {
                        return ToolResult {
                            output: "Missing required parameter: code".to_string(),
                            is_error: true,
                        }
                    }
                };

                let requested_status = match args
                    .get("request_editor_status")
                    .and_then(|v| v.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    Some(status) => status,
                    None => {
                        return ToolResult {
                            output: "Missing required parameter: request_editor_status".to_string(),
                            is_error: true,
                        }
                    }
                };

                if requested_status == crate::unity_bridge::UNITY_EDITOR_STATUS_DISCONNECTED
                    || !crate::unity_bridge::is_known_editor_status(requested_status)
                {
                    return ToolResult {
                        output: format!(
                            "Invalid request_editor_status: '{}'. Allowed values: editing, playing, playing_paused.",
                            requested_status
                        ),
                        is_error: true,
                    };
                }

                let project_path = match ctx.working_dir {
                    Some(path) if !path.trim().is_empty() => path.trim().to_string(),
                    _ => {
                        return ToolResult {
                            output: "Tool 'unity_execute' requires a selected Unity project working directory.".to_string(),
                            is_error: true,
                        }
                    }
                };

                let (connected, actual_status, _scene) =
                    crate::unity_bridge::query_unity_status(&project_path).await;
                if !connected {
                    return ToolResult {
                        output: "Unity Editor not connected".to_string(),
                        is_error: true,
                    };
                }

                if actual_status != requested_status {
                    return ToolResult {
                        output: format!(
                            "Unity Editor status is \"{}\". `unity_execute` requires \"{}\".",
                            actual_status, requested_status
                        ),
                        is_error: true,
                    };
                }

                match crate::unity_bridge::unity_execute_code(&project_path, &code).await {
                    Ok(output) => {
                        let trimmed = output.trim();
                        ToolResult {
                            output: if trimmed.is_empty() {
                                "Code executed successfully (no output).".to_string()
                            } else {
                                trimmed.to_string()
                            },
                            is_error: false,
                        }
                    }
                    Err(e) => ToolResult {
                        output: e,
                        is_error: true,
                    },
                }
            })
        }),
    }
}

// ─── unity_run_states ───────────────────────────────────────────────────────

pub(super) fn unity_run_states() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::UNITY_RUN_STATES);
    ToolDef {
        name: "unity_run_states".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: make_exec(|args, ctx| {
            Box::pin(async move {
                let project_path = match ctx.working_dir {
                    Some(path) if !path.trim().is_empty() => path,
                    _ => {
                        return ToolResult {
                            output: "Tool 'unity_run_states' requires a selected Unity project working directory.".to_string(),
                            is_error: true,
                        };
                    }
                };

                let requested_status = match args
                    .get("request_editor_status")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .filter(|value| !value.is_empty())
                {
                    Some(status) => status,
                    None => {
                        return ToolResult {
                            output: "Missing required parameter: request_editor_status".to_string(),
                            is_error: true,
                        };
                    }
                };

                let (connected, _actual_status, _) =
                    crate::unity_bridge::query_unity_status(&project_path).await;
                if !connected {
                    return ToolResult {
                        output: "Unity Editor not connected".to_string(),
                        is_error: true,
                    };
                }

                if let Err(error) =
                    crate::unity_bridge::compile_run_states(&project_path, &args).await
                {
                    return ToolResult {
                        output: error,
                        is_error: true,
                    };
                }

                let (connected, actual_status, _) =
                    crate::unity_bridge::query_unity_status(&project_path).await;
                if !connected {
                    return ToolResult {
                        output: "Unity Editor not connected".to_string(),
                        is_error: true,
                    };
                }

                if actual_status != requested_status {
                    return ToolResult {
                        output: format!(
                            "Unity Editor status is \"{}\". `unity_run_states` requires \"{}\".",
                            actual_status, requested_status
                        ),
                        is_error: true,
                    };
                }

                match crate::unity_bridge::unity_run_states(&project_path, &args).await {
                    Ok(output) => ToolResult {
                        output: output.trim().to_string(),
                        is_error: false,
                    },
                    Err(e) => ToolResult {
                        output: e,
                        is_error: true,
                    },
                }
            })
        }),
    }
}

// ─── unity_ref_search ──────────────────────────────────────────────────────

pub(super) fn unity_ref_search() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::UNITY_REF_SEARCH);
    ToolDef {
        name: "unity_ref_search".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: Arc::new(|_args, _ctx| {
            Box::pin(async {
                ToolResult {
                    output: "Error: unity_ref_search should be intercepted by agent loop, not executed directly".to_string(),
                    is_error: true,
                }
            })
        }),
    }
}

// ─── unity_asset_search ─────────────────────────────────────────────────────

pub(super) fn unity_asset_search() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::UNITY_ASSET_SEARCH);
    ToolDef {
        name: "unity_asset_search".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: Arc::new(|_args, _ctx| {
            Box::pin(async {
                ToolResult {
                    output: "Error: unity_asset_search should be intercepted by agent loop, not executed directly".to_string(),
                    is_error: true,
                }
            })
        }),
    }
}

// ─── Unity YAML tools ────────────────────────────────────────────────────────

fn intercepted_unity_yaml_tool(name: &str, prompt_json: &str) -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(prompt_json);
    let tool_name = name.to_string();
    ToolDef {
        name: tool_name.clone(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: Arc::new(move |_args, _ctx| {
            let tool_name = tool_name.clone();
            Box::pin(async move {
                ToolResult {
                    output: format!(
                        "Error: {} should be intercepted by agent loop, not executed directly",
                        tool_name
                    ),
                    is_error: true,
                }
            })
        }),
    }
}

pub(super) fn unity_yaml_list() -> ToolDef {
    intercepted_unity_yaml_tool("unity_yaml_list", crate::prompt::tools::UNITY_YAML_LIST)
}

pub(super) fn unity_yaml_search() -> ToolDef {
    intercepted_unity_yaml_tool("unity_yaml_search", crate::prompt::tools::UNITY_YAML_SEARCH)
}

pub(super) fn unity_yaml_read() -> ToolDef {
    intercepted_unity_yaml_tool("unity_yaml_read", crate::prompt::tools::UNITY_YAML_READ)
}

// ─── unity_recompile ─────────────────────────────────────────────────────────

pub(super) fn unity_recompile() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::UNITY_RECOMPILE);
    ToolDef {
        name: "unity_recompile".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: make_exec(|args, _ctx| {
            Box::pin(async move {
                let claimed_status = match args.get("editor_status").and_then(|v| v.as_str()) {
                    Some(s) => s.to_string(),
                    None => {
                        return ToolResult {
                            output: format!(
                                "Missing required parameter: editor_status. You must pass the current Unity Editor status ({}) exactly as shown in the Environment section.",
                                crate::unity_bridge::UNITY_EDITOR_STATUS_SCHEMA
                            ),
                            is_error: true,
                        }
                    }
                };

                if !crate::unity_bridge::is_known_editor_status(&claimed_status) {
                    return ToolResult {
                        output: format!(
                            "Invalid editor_status: \"{}\". Allowed values: {}.",
                            claimed_status,
                            crate::unity_bridge::UNITY_EDITOR_STATUS_SCHEMA
                        ),
                        is_error: true,
                    };
                }

                let project_path = match args.get("project_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim().to_string(),
                    _ => {
                        return ToolResult {
                            output: "Missing required parameter: project_path".to_string(),
                            is_error: true,
                        }
                    }
                };

                // Verify editor_status matches actual Unity state
                let (_connected, actual_status, _scene) =
                    crate::unity_bridge::query_unity_status(&project_path).await;
                if claimed_status != actual_status {
                    return ToolResult {
                        output: format!(
                            "editor_status mismatch: you claimed \"{}\", but the actual editor status is \"{}\". Re-read the current editor state and try again.",
                            claimed_status, actual_status
                        ),
                        is_error: true,
                    };
                }

                if actual_status == crate::unity_bridge::UNITY_EDITOR_STATUS_DISCONNECTED {
                    return ToolResult {
                        output: "Unity Editor status is \"disconnected\". `unity_recompile` is unavailable until the Editor reconnects.".to_string(),
                        is_error: true,
                    };
                }

                if crate::unity_bridge::is_play_mode_status(actual_status) {
                    return ToolResult {
                        output: format!(
                            "Unity Editor status is \"{}\". Exit Play Mode before calling `unity_recompile`.",
                            actual_status
                        ),
                        is_error: true,
                    };
                }

                match crate::unity_bridge::recompile_and_wait(&project_path).await {
                    Ok(msg) => ToolResult {
                        output: msg,
                        is_error: false,
                    },
                    Err(e) => ToolResult {
                        output: format!("Compilation failed:\n{}", e),
                        is_error: true,
                    },
                }
            })
        }),
    }
}

pub(super) fn run_playmode_tests() -> ToolDef {
    let prompt = crate::prompt::parse_tool_prompt(crate::prompt::tools::RUN_PLAYMODE_TESTS);
    ToolDef {
        name: "run_playmode_tests".to_string(),
        description: prompt.description,
        parameters: prompt.parameters,
        execute: make_exec(|args, ctx| {
            Box::pin(async move {
                let action = args
                    .get("action")
                    .and_then(|value| value.as_str())
                    .map(str::trim)
                    .unwrap_or("");
                if action.is_empty() {
                    return ToolResult {
                        output: "Missing required parameter: action".to_string(),
                        is_error: true,
                    };
                }

                let project_path = match args.get("project_path").and_then(|v| v.as_str()) {
                    Some(path) if !path.trim().is_empty() => path.trim().to_string(),
                    _ => match ctx.working_dir {
                        Some(path) if !path.trim().is_empty() => path.trim().to_string(),
                        _ => {
                            return ToolResult {
                                output: "Missing required parameter: project_path".to_string(),
                                is_error: true,
                            }
                        }
                    },
                };

                match action {
                    "start" => match crate::unity_flow::start_run_playmode_tests(&project_path).await
                    {
                        Ok(result) => ToolResult {
                            output: serde_json::json!({
                                "task_id": result.task_id,
                                "state": result.state,
                                "message": "playmode test flow task created"
                            })
                            .to_string(),
                            is_error: false,
                        },
                        Err(error) => ToolResult {
                            output: error,
                            is_error: true,
                        },
                    },
                    "status" => {
                        let Some(task_id) = args.get("task_id").and_then(|v| v.as_str()) else {
                            return ToolResult {
                                output: "Missing required parameter: task_id (for action=status)"
                                    .to_string(),
                                is_error: true,
                            };
                        };
                        match crate::unity_flow::get_task(&project_path, task_id.trim()).await {
                            Ok(task) => match serde_json::to_string(&task) {
                                Ok(json) => ToolResult {
                                    output: json,
                                    is_error: false,
                                },
                                Err(error) => ToolResult {
                                    output: format!("Failed to serialize task status: {}", error),
                                    is_error: true,
                                },
                            },
                            Err(error) => ToolResult {
                                output: error,
                                is_error: true,
                            },
                        }
                    }
                    "wait" => {
                        let Some(task_id) = args.get("task_id").and_then(|v| v.as_str()) else {
                            return ToolResult {
                                output: "Missing required parameter: task_id (for action=wait)"
                                    .to_string(),
                                is_error: true,
                            };
                        };
                        let timeout_sec = args
                            .get("timeout_sec")
                            .and_then(|v| v.as_u64())
                            .unwrap_or(120);
                        match crate::unity_flow::wait_task(
                            &project_path,
                            task_id.trim(),
                            std::time::Duration::from_secs(timeout_sec),
                        )
                        .await
                        {
                            Ok(task) => match serde_json::to_string(&task) {
                                Ok(json) => ToolResult {
                                    output: json,
                                    is_error: false,
                                },
                                Err(error) => ToolResult {
                                    output: format!("Failed to serialize waited task: {}", error),
                                    is_error: true,
                                },
                            },
                            Err(error) => ToolResult {
                                output: error,
                                is_error: true,
                            },
                        }
                    }
                    "cancel" => {
                        let Some(task_id) = args.get("task_id").and_then(|v| v.as_str()) else {
                            return ToolResult {
                                output: "Missing required parameter: task_id (for action=cancel)"
                                    .to_string(),
                                is_error: true,
                            };
                        };
                        match crate::unity_flow::cancel_task(&project_path, task_id.trim()).await {
                            Ok(task) => match serde_json::to_string(&task) {
                                Ok(json) => ToolResult {
                                    output: json,
                                    is_error: false,
                                },
                                Err(error) => ToolResult {
                                    output: format!("Failed to serialize cancelled task: {}", error),
                                    is_error: true,
                                },
                            },
                            Err(error) => ToolResult {
                                output: error,
                                is_error: true,
                            },
                        }
                    }
                    _ => ToolResult {
                        output: "Invalid action. Allowed values: start, status, wait, cancel"
                            .to_string(),
                        is_error: true,
                    },
                }
            })
        }),
    }
}
