use gpui::{
    div, percentage, Animation, AnimationExt, DismissEvent, EventEmitter, FocusHandle, Focusable, InteractiveElement,
    ParentElement, Render, Styled, Task, ViewContext, VisualContext, WeakEntity, WindowContext
};
use std::path::PathBuf;
use ui::{prelude::*, AlertModal, Button, ButtonStyle, Icon, IconName, Label};
use crate::{ModalView, Workspace};

pub enum Decision {
    Stream,
    Normal,
}

enum LoadState {
    Decision,
    Loading {
        decision: Decision,
        progress: f32,
        _task: Option<Task<()>>,
    },
}

pub struct LargeFileModal {
    focus_handle: FocusHandle,
    workspace: WeakEntity<Workspace>,
    path: PathBuf,
    size_bytes: u64,
    state: LoadState,
}

impl Focusable for LargeFileModal {
    fn focus_handle(&self, _: &gpui::App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl EventEmitter<DismissEvent> for LargeFileModal {}

impl ModalView for LargeFileModal {
    fn on_before_dismiss(&mut self, _: &mut gpui::Window, _: &mut ViewContext<Self>) -> crate::DismissDecision {
        crate::DismissDecision::Dismiss(true)
    }
}

impl Render for LargeFileModal {
    fn render(&mut self, _window: &mut gpui::Window, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let is_loading = matches!(self.state, LoadState::Loading { .. });
        
        let mut main_content = AlertModal::new("large-file-modal")
            .width(rems(40.))
            .key_context("LargeFileModal")
            .track_focus(&self.focus_handle(cx))
            .header(
                v_flex()
                    .p_3()
                    .gap_1()
                    .rounded_t_md()
                    .bg(cx.theme().colors().editor_background.opacity(0.5))
                    .border_b_1()
                    .border_color(cx.theme().colors().border_variant)
                    .child(
                        h_flex()
                            .gap_2()
                            .child(Icon::new(IconName::Warning).color(Color::Warning))
                            .child(Label::new("Loading Large File")),
                    )
                    .child(
                        h_flex()
                            .pl(IconSize::default().rems() + rems(0.5))
                            .child(Label::new(self.path.display().to_string()).color(Color::Muted)),
                    ),
            )
            .child(
                v_flex()
                    .gap_2()
                    .child(Label::new(format!("This file is extremely large ({} MB) and may be memory-intensive relative to your current system resources.", self.size_bytes / 1024 / 1024)).color(Color::Muted))
                    .child(Label::new("Do you want to open it in normal edit mode or disk-streaming mode?").color(Color::Muted))
            );

        if !is_loading {
            main_content = main_content.footer(
                h_flex()
                    .px_3()
                    .pb_3()
                    .gap_2()
                    .justify_end()
                    .child(
                        Button::new("cancel", "Cancel")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.dismiss(cx);
                            }))
                    )
                    .child(
                        Button::new("normal", "Normal Edit Mode")
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_loading(Decision::Normal, cx);
                            }))
                    )
                    .child(
                        Button::new("stream", "Disk-Streaming Mode")
                            .style(ButtonStyle::Filled)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.start_loading(Decision::Stream, cx);
                            }))
                    )
            );
        }

        h_flex()
            .gap_4()
            .child(main_content)
            .when(is_loading, |flex| flex.child(self.render_loading_bar(cx)))
            .into_any_element()
    }
}

impl LargeFileModal {
    pub fn new(
        workspace: WeakEntity<Workspace>,
        path: PathBuf,
        size_bytes: u64,
        cx: &mut ViewContext<Self>,
    ) -> Self {
        Self {
            focus_handle: cx.focus_handle(),
            workspace,
            path,
            size_bytes,
            state: LoadState::Decision,
        }
    }

    fn dismiss(&mut self, cx: &mut ViewContext<Self>) {
        cx.emit(DismissEvent);
    }

    fn start_loading(&mut self, decision: Decision, cx: &mut ViewContext<Self>) {
        // Start loading simulation or actual task tracking
        self.state = LoadState::Loading {
            decision,
            progress: 0.0,
            _task: None, // Will be set to the actual file load task
        };
        cx.notify();
        
        // Setup a tick to animate progress
        self.animate_progress(cx);
    }

    fn animate_progress(&mut self, cx: &mut ViewContext<Self>) {
        cx.spawn(|this, mut cx| async move {
            loop {
                cx.background_executor().timer(std::time::Duration::from_millis(16)).await;
                if let Some(mut this) = this.upgrade() {
                    this.update(&mut cx, |this, cx| {
                        if let LoadState::Loading { ref mut progress, .. } = this.state {
                            *progress += 0.01;
                            if *progress >= 1.0 {
                                *progress = 0.0;
                            }
                            cx.notify();
                        }
                    }).ok();
                } else {
                    break;
                }
            }
        }).detach();
    }

    fn cancel_loading(&mut self, cx: &mut ViewContext<Self>) {
        // Drop the task, transitioning back to decision state or dismissing
        self.state = LoadState::Decision;
        cx.notify();
    }

    fn render_loading_bar(&self, cx: &mut ViewContext<Self>) -> impl IntoElement {
        let progress = match self.state {
            LoadState::Loading { progress, .. } => progress,
            _ => 0.0,
        };

        v_flex()
            .w(rems(4.0))
            .bg(cx.theme().colors().panel_background)
            .border_1()
            .border_color(cx.theme().colors().border)
            .rounded_md()
            .p_2()
            .gap_2()
            .child(
                Button::new("cancel-loading", "X")
                    .style(ButtonStyle::Subtle)
                    .on_click(cx.listener(|this, _, _, cx| {
                        this.cancel_loading(cx);
                    }))
            )
            .child(
                div()
                    .flex_1()
                    .rounded_sm()
                    // Slight glass look via opacity and blending
                    .bg(cx.theme().colors().element_background.opacity(0.3))
                    .relative()
                    .overflow_hidden()
                    .child(
                        div()
                            .absolute()
                            .bottom_0()
                            .left_0()
                            .right_0()
                            .bg(cx.theme().colors().element_active.opacity(0.6))
                            // height based on progress filling up cup
                            .h(percentage(progress))
                    )
            )
    }
}
