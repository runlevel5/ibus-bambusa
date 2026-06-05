//! The `org.freedesktop.IBus.Engine` D-Bus interface and its backing actor.
//!
//! Incoming IBus method calls are async and may interleave; the engine logic
//! and signal emission must stay strictly ordered (the forwarding modes depend
//! on it). So the interface object only forwards each call as a [`Cmd`] over an
//! mpsc channel to a single task that owns the handler and the signal emitter,
//! processing one command to completion before the next.

use tokio::sync::{mpsc, oneshot};
use zbus::object_server::SignalEmitter;
use zvariant::{OwnedValue, Value};

use crate::consts::PREEDIT_COMMIT;
use crate::handler::{Action, EngineHandler};
use crate::types::IBusText;

/// A forwarded IBus method call.
enum Cmd {
    ProcessKey {
        keyval: u32,
        keycode: u32,
        state: u32,
        reply: oneshot::Sender<bool>,
    },
    FocusIn,
    FocusOut,
    Reset,
    Enable,
    Disable,
    SetSurroundingText {
        text: String,
        cursor: u32,
        anchor: u32,
    },
    PropertyActivate {
        name: String,
        state: u32,
    },
    CandidateClicked {
        index: u32,
        button: u32,
        state: u32,
    },
    PageUp,
    PageDown,
    CursorUp,
    CursorDown,
    Destroy,
}

/// Owns the handler and emitter; processes commands in order.
struct Actor {
    handler: Box<dyn EngineHandler>,
    emitter: SignalEmitter<'static>,
}

impl Actor {
    async fn run(mut self, mut rx: mpsc::Receiver<Cmd>) {
        while let Some(cmd) = rx.recv().await {
            match cmd {
                Cmd::ProcessKey {
                    keyval,
                    keycode,
                    state,
                    reply,
                } => {
                    let (handled, actions) = self.handler.process_key_event(keyval, keycode, state);
                    self.emit_all(actions).await;
                    let _ = reply.send(handled);
                }
                Cmd::FocusIn => self.emit_handler(|h| h.focus_in()).await,
                Cmd::FocusOut => self.emit_handler(|h| h.focus_out()).await,
                Cmd::Reset => self.emit_handler(|h| h.reset()).await,
                Cmd::Enable => self.emit_handler(|h| h.enable()).await,
                Cmd::Disable => self.emit_handler(|h| h.disable()).await,
                Cmd::PageUp => self.emit_handler(|h| h.page_up()).await,
                Cmd::PageDown => self.emit_handler(|h| h.page_down()).await,
                Cmd::CursorUp => self.emit_handler(|h| h.cursor_up()).await,
                Cmd::CursorDown => self.emit_handler(|h| h.cursor_down()).await,
                Cmd::SetSurroundingText {
                    text,
                    cursor,
                    anchor,
                } => {
                    let actions = self.handler.set_surrounding_text(text, cursor, anchor);
                    self.emit_all(actions).await;
                }
                Cmd::PropertyActivate { name, state } => {
                    let actions = self.handler.property_activate(name, state);
                    self.emit_all(actions).await;
                }
                Cmd::CandidateClicked {
                    index,
                    button,
                    state,
                } => {
                    let actions = self.handler.candidate_clicked(index, button, state);
                    self.emit_all(actions).await;
                }
                Cmd::Destroy => break,
            }
        }
    }

    async fn emit_handler(&mut self, f: impl FnOnce(&mut dyn EngineHandler) -> Vec<Action>) {
        let actions = f(self.handler.as_mut());
        self.emit_all(actions).await;
    }

    async fn emit_all(&mut self, actions: Vec<Action>) {
        for action in actions {
            // A failed emission (e.g. client gone) shouldn't kill the engine.
            let _ = self.emit(action).await;
        }
    }

    async fn emit(&mut self, action: Action) -> zbus::Result<()> {
        let e = &self.emitter;
        match action {
            Action::CommitText(text) => {
                EngineInterface::commit_text(e, IBusText::new(text).into()).await
            }
            Action::UpdatePreedit {
                text,
                cursor_pos,
                visible,
                underline,
            } => {
                let t = if underline {
                    IBusText::with_underline(text)
                } else {
                    IBusText::new(text)
                };
                EngineInterface::update_preedit_text(
                    e,
                    t.into(),
                    cursor_pos,
                    visible,
                    PREEDIT_COMMIT,
                )
                .await
            }
            Action::HidePreedit => EngineInterface::hide_preedit_text(e).await,
            Action::ForwardKeyEvent {
                keyval,
                keycode,
                state,
            } => EngineInterface::forward_key_event(e, keyval, keycode, state).await,
            Action::DeleteSurroundingText { offset, nchars } => {
                EngineInterface::delete_surrounding_text(e, offset, nchars).await
            }
            Action::UpdateAuxiliaryText { text, visible } => {
                EngineInterface::update_auxiliary_text(e, IBusText::new(text).into(), visible).await
            }
            Action::HideAuxiliaryText => EngineInterface::hide_auxiliary_text(e).await,
            Action::UpdateLookupTable { table, visible } => {
                EngineInterface::update_lookup_table(e, (*table).into(), visible).await
            }
            Action::HideLookupTable => EngineInterface::hide_lookup_table(e).await,
            Action::RegisterProperties(props) => {
                EngineInterface::register_properties(e, (*props).into()).await
            }
            Action::UpdateProperty(prop) => {
                EngineInterface::update_property(e, (*prop).into()).await
            }
            Action::RequireSurroundingText => EngineInterface::require_surrounding_text(e).await,
        }
    }
}

/// The exported `org.freedesktop.IBus.Engine` object: a thin forwarder.
pub struct EngineInterface {
    cmds: mpsc::Sender<Cmd>,
}

impl EngineInterface {
    /// Build the interface object plus an actor driving `handler`, emitting via
    /// `emitter`. Spawn `actor.run(rx)` on the runtime; serve the returned
    /// interface at the object path `emitter` was created for.
    pub fn new(
        handler: Box<dyn EngineHandler>,
        emitter: SignalEmitter<'static>,
    ) -> (Self, impl std::future::Future<Output = ()>) {
        let (tx, rx) = mpsc::channel(64);
        let actor = Actor { handler, emitter };
        (Self { cmds: tx }, actor.run(rx))
    }

    async fn send(&self, cmd: Cmd) {
        let _ = self.cmds.send(cmd).await;
    }
}

fn variant_text(value: Value<'_>) -> String {
    OwnedValue::try_from(value)
        .ok()
        .and_then(|owned| IBusText::try_from(owned).ok())
        .map(|t| t.text)
        .unwrap_or_default()
}

#[zbus::interface(name = "org.freedesktop.IBus.Engine")]
impl EngineInterface {
    async fn process_key_event(&self, keyval: u32, keycode: u32, state: u32) -> bool {
        let (reply, rx) = oneshot::channel();
        if self
            .cmds
            .send(Cmd::ProcessKey {
                keyval,
                keycode,
                state,
                reply,
            })
            .await
            .is_err()
        {
            return false;
        }
        rx.await.unwrap_or(false)
    }

    async fn focus_in(&self) {
        self.send(Cmd::FocusIn).await;
    }

    async fn focus_out(&self) {
        self.send(Cmd::FocusOut).await;
    }

    async fn reset(&self) {
        self.send(Cmd::Reset).await;
    }

    async fn enable(&self) {
        self.send(Cmd::Enable).await;
    }

    async fn disable(&self) {
        self.send(Cmd::Disable).await;
    }

    async fn set_surrounding_text(&self, text: Value<'_>, cursor_index: u32, anchor_pos: u32) {
        self.send(Cmd::SetSurroundingText {
            text: variant_text(text),
            cursor: cursor_index,
            anchor: anchor_pos,
        })
        .await;
    }

    async fn set_cursor_location(&self, _x: i32, _y: i32, _w: i32, _h: i32) {}

    async fn set_capabilities(&self, _caps: u32) {}

    async fn set_content_type(&self, _purpose: u32, _hints: u32) {}

    async fn property_activate(&self, name: String, state: u32) {
        self.send(Cmd::PropertyActivate { name, state }).await;
    }

    async fn candidate_clicked(&self, index: u32, button: u32, state: u32) {
        self.send(Cmd::CandidateClicked {
            index,
            button,
            state,
        })
        .await;
    }

    async fn page_up(&self) {
        self.send(Cmd::PageUp).await;
    }

    async fn page_down(&self) {
        self.send(Cmd::PageDown).await;
    }

    async fn cursor_up(&self) {
        self.send(Cmd::CursorUp).await;
    }

    async fn cursor_down(&self) {
        self.send(Cmd::CursorDown).await;
    }

    async fn destroy(&self) {
        self.send(Cmd::Destroy).await;
    }

    #[zbus(signal)]
    async fn commit_text(emitter: &SignalEmitter<'_>, text: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_preedit_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        cursor_pos: u32,
        visible: bool,
        mode: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn show_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_preedit_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_auxiliary_text(
        emitter: &SignalEmitter<'_>,
        text: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_auxiliary_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_lookup_table(
        emitter: &SignalEmitter<'_>,
        table: Value<'_>,
        visible: bool,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn hide_lookup_table(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn register_properties(emitter: &SignalEmitter<'_>, props: Value<'_>)
    -> zbus::Result<()>;

    #[zbus(signal)]
    async fn update_property(emitter: &SignalEmitter<'_>, prop: Value<'_>) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn forward_key_event(
        emitter: &SignalEmitter<'_>,
        keyval: u32,
        keycode: u32,
        state: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn delete_surrounding_text(
        emitter: &SignalEmitter<'_>,
        offset: i32,
        nchars: u32,
    ) -> zbus::Result<()>;

    #[zbus(signal)]
    async fn require_surrounding_text(emitter: &SignalEmitter<'_>) -> zbus::Result<()>;
}
