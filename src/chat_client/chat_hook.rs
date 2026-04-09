// Copyright (C) 2026 The pgmoneta community
//
// This program is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.
//
// This program is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License
// along with this program. If not, see <https://www.gnu.org/licenses/>.

use console::style;
use indicatif::{ProgressBar, ProgressStyle};
use rig::agent::{HookAction, PromptHook, ToolCallHookAction};
use rig::completion::{CompletionModel, CompletionResponse, Message};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// PromptHook implementation providing a professional chain-style UI
/// with animated spinners, streaming output, and rich tool call display.
#[derive(Clone)]
pub struct ChatHook {
    state: Arc<Mutex<ChainState>>,
}

struct ChainState {
    provider_label: String,
    round: usize,
    tool_start: Option<Instant>,
    input_tokens: u64,
    output_tokens: u64,
    tool_count: usize,
    last_error: Option<String>,
    prompt_start: Instant,
    spinner: Option<ProgressBar>,
}

impl ChainState {
    fn start_spinner(&mut self, msg: &str) {
        // Finish any existing spinner first
        self.finish_spinner();
        let pb = ProgressBar::new_spinner();
        pb.set_style(
            ProgressStyle::with_template("  {spinner:.cyan} {msg}")
                .unwrap()
                .tick_strings(&["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏", "✔"]),
        );
        pb.set_message(msg.to_string());
        pb.enable_steady_tick(Duration::from_millis(80));
        self.spinner = Some(pb);
    }

    fn finish_spinner(&mut self) {
        if let Some(pb) = self.spinner.take() {
            pb.finish_and_clear();
        }
    }
}

impl ChatHook {
    pub fn new(provider: &str, model: &str, endpoint: &str) -> Self {
        let label = format!("{model} ({provider} @ {endpoint})");
        Self {
            state: Arc::new(Mutex::new(ChainState {
                provider_label: label,
                round: 0,
                tool_start: None,
                input_tokens: 0,
                output_tokens: 0,
                tool_count: 0,
                last_error: None,
                prompt_start: Instant::now(),
                spinner: None,
            })),
        }
    }

    pub fn reset(&self) {
        if let Ok(mut state) = self.state.try_lock() {
            state.finish_spinner();
            state.round = 0;
            state.input_tokens = 0;
            state.output_tokens = 0;
            state.tool_count = 0;
            state.tool_start = None;
            state.last_error = None;
            state.prompt_start = Instant::now();
        }
    }

    pub async fn clear_chain(&self) {
        let mut state = self.state.lock().await;
        state.finish_spinner();
    }

    pub async fn print_final_summary(&self) {
        let mut state = self.state.lock().await;
        state.finish_spinner();

        let elapsed = state.prompt_start.elapsed().as_secs_f32();
        let tools = state.tool_count;
        let t_in = state.input_tokens;
        let t_out = state.output_tokens;

        let mut parts: Vec<String> = Vec::new();
        if tools > 0 {
            parts.push(format!(
                "{} tool{}",
                tools,
                if tools > 1 { "s" } else { "" }
            ));
        }
        parts.push(format!("{}→{} tokens", t_in, t_out));
        parts.push(format!("{elapsed:.1}s"));

        eprintln!(
            "  {} {}",
            style("✔").green().bold(),
            style(format!("Done ({})", parts.join(" · "))).dim()
        );

        if let Some(err) = &state.last_error {
            eprintln!("  {} {}", style("⚠").yellow(), style(err).dim());
        }
    }

    pub async fn print_error_summary(&self, error: &str) {
        let mut state = self.state.lock().await;
        state.finish_spinner();
        let elapsed = state.prompt_start.elapsed().as_secs_f32();
        eprintln!(
            "  {} {} · {elapsed:.1}s",
            style("✖").red().bold(),
            style(error).red()
        );
    }

    fn format_args_inline(args: &str) -> String {
        if let Ok(serde_json::Value::Object(obj)) = serde_json::from_str::<serde_json::Value>(args)
        {
            let formatted: Vec<String> = obj
                .iter()
                .map(|(k, v)| {
                    let val_str = match v {
                        serde_json::Value::String(s) => {
                            if s.len() > 30 {
                                format!("\"{}...\"", &s[..27])
                            } else {
                                format!("\"{s}\"")
                            }
                        }
                        other => {
                            let s = other.to_string();
                            if s.len() > 30 {
                                format!("{}...", &s[..27])
                            } else {
                                s
                            }
                        }
                    };
                    format!("{k}={val_str}")
                })
                .collect();
            return formatted.join(", ");
        }
        String::new()
    }
}

impl<M: CompletionModel> PromptHook<M> for ChatHook {
    async fn on_completion_call(&self, _prompt: &Message, _history: &[Message]) -> HookAction {
        let mut state = self.state.lock().await;
        state.round += 1;
        let r = state.round;

        let label = state.provider_label.clone();
        if r == 1 {
            state.start_spinner(&format!(
                "Thinking... {}",
                style(format!("({label})")).dim()
            ));
        } else {
            state.start_spinner(&format!(
                "Processing results... {}",
                style(format!("(round {r})")).dim()
            ));
        }
        HookAction::cont()
    }

    async fn on_completion_response(
        &self,
        _prompt: &Message,
        response: &CompletionResponse<M::Response>,
    ) -> HookAction {
        let mut state = self.state.lock().await;
        state.input_tokens = response.usage.input_tokens;
        state.output_tokens = response.usage.output_tokens;
        state.finish_spinner();
        HookAction::cont()
    }

    async fn on_tool_call(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        args: &str,
    ) -> ToolCallHookAction {
        let mut state = self.state.lock().await;
        state.tool_count += 1;
        state.tool_start = Some(Instant::now());

        let args_preview = Self::format_args_inline(args);
        let tool_display = if args_preview.is_empty() {
            tool_name.to_string()
        } else {
            format!("{tool_name}({args_preview})")
        };

        state.start_spinner(&format!("Calling {}...", style(&tool_display).cyan()));
        ToolCallHookAction::cont()
    }

    async fn on_tool_result(
        &self,
        tool_name: &str,
        _tool_call_id: Option<String>,
        _internal_call_id: &str,
        _args: &str,
        result: &str,
    ) -> HookAction {
        let mut state = self.state.lock().await;
        let ms = state
            .tool_start
            .map(|t| t.elapsed().as_millis())
            .unwrap_or(0);

        state.finish_spinner();

        let is_error =
            result.to_lowercase().contains("error") || result.to_lowercase().contains("failed");

        if is_error {
            let preview = if result.len() > 60 {
                format!("{}...", &result[..60])
            } else {
                result.to_string()
            };
            state.last_error = Some(format!("{tool_name}: {preview}"));
            eprintln!(
                "  {} {} {}",
                style("✖").red(),
                style(tool_name).red(),
                style(format!("{ms}ms")).dim()
            );
        } else {
            eprintln!(
                "  {} {} {}",
                style("✔").green(),
                style(tool_name).cyan(),
                style(format!("{ms}ms")).dim()
            );
        }
        HookAction::cont()
    }
}
