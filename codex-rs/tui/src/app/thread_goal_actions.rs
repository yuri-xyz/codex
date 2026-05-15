use super::App;
use crate::app_event::ThreadGoalSetMode;
use crate::app_server_session::AppServerSession;
use crate::goal_display::goal_status_label;
use crate::goal_display::goal_usage_summary;
use codex_app_server_protocol::ThreadGoalStatus;
use codex_protocol::ThreadId;

impl App {
    pub(super) async fn open_thread_goal_menu(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) {
        let result = app_server.thread_goal_get(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                self.chat_widget
                    .add_error_message(format!("Failed to read thread goal: {err}"));
                return;
            }
        };

        let Some(goal) = response.goal else {
            self.chat_widget.add_info_message(
                "Usage: /goal <objective>".to_string(),
                Some("No goal is currently set.".to_string()),
            );
            return;
        };

        self.chat_widget.show_goal_summary(goal);
    }

    pub(super) async fn maybe_prompt_resume_paused_goal_after_resume(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) {
        let result = app_server.thread_goal_get(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                tracing::warn!("failed to read thread goal after resume: {err}");
                return;
            }
        };

        let Some(goal) = response.goal else {
            return;
        };
        if goal.status == ThreadGoalStatus::Paused {
            self.chat_widget
                .show_resume_paused_goal_prompt(thread_id, goal.objective);
        }
    }

    pub(super) async fn open_thread_goal_editor(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: Option<ThreadId>,
    ) {
        let Some(thread_id) = thread_id else {
            self.show_no_thread_goal_to_edit();
            return;
        };

        let result = app_server.thread_goal_get(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        let response = match result {
            Ok(response) => response,
            Err(err) => {
                self.chat_widget
                    .add_error_message(format!("Failed to read thread goal: {err}"));
                return;
            }
        };

        let Some(goal) = response.goal else {
            self.show_no_thread_goal_to_edit();
            return;
        };

        self.chat_widget.show_goal_edit_prompt(thread_id, goal);
    }

    pub(super) async fn set_thread_goal_objective(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
        objective: String,
        mode: ThreadGoalSetMode,
    ) {
        let replacing_goal = matches!(mode, ThreadGoalSetMode::ReplaceExisting);
        if replacing_goal {
            let result = app_server.thread_goal_clear(thread_id).await;

            if let Err(err) = result {
                if self.current_displayed_thread_id() != Some(thread_id) {
                    return;
                }
                self.chat_widget
                    .add_error_message(format!("Failed to replace thread goal: {err}"));
                return;
            }
        }

        let (status, token_budget) = match mode {
            ThreadGoalSetMode::ReplaceExisting => (ThreadGoalStatus::Active, None),
            ThreadGoalSetMode::UpdateExisting {
                status,
                token_budget,
            } => (status, Some(token_budget)),
        };

        let result = app_server
            .thread_goal_set(thread_id, Some(objective), Some(status), token_budget)
            .await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => self.chat_widget.add_info_message(
                format!("Goal {}", goal_status_label(response.goal.status)),
                Some(goal_usage_summary(&response.goal)),
            ),
            Err(err) => {
                let action = if replacing_goal { "replace" } else { "set" };
                self.chat_widget
                    .add_error_message(format!("Failed to {action} thread goal: {err}"));
            }
        }
    }

    pub(super) async fn set_thread_goal_status(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
        status: ThreadGoalStatus,
    ) {
        let result = app_server
            .thread_goal_set(
                thread_id,
                /*objective*/ None,
                Some(status),
                /*token_budget*/ None,
            )
            .await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => self.chat_widget.add_info_message(
                format!("Goal {}", goal_status_label(response.goal.status)),
                Some(goal_usage_summary(&response.goal)),
            ),
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to update thread goal: {err}")),
        }
    }

    pub(super) async fn clear_thread_goal(
        &mut self,
        app_server: &mut AppServerSession,
        thread_id: ThreadId,
    ) {
        let result = app_server.thread_goal_clear(thread_id).await;
        if self.current_displayed_thread_id() != Some(thread_id) {
            return;
        }

        match result {
            Ok(response) => {
                if response.cleared {
                    self.chat_widget
                        .add_info_message("Goal cleared".to_string(), /*hint*/ None);
                } else {
                    self.chat_widget.add_info_message(
                        "No goal to clear".to_string(),
                        Some("This thread does not currently have a goal.".to_string()),
                    );
                }
            }
            Err(err) => self
                .chat_widget
                .add_error_message(format!("Failed to clear thread goal: {err}")),
        }
    }

    fn show_no_thread_goal_to_edit(&mut self) {
        self.chat_widget
            .add_error_message("No goal is currently set.".to_string());
        self.chat_widget.add_info_message(
            "Usage: /goal <objective>".to_string(),
            Some("Create a goal before editing it.".to_string()),
        );
    }
}
