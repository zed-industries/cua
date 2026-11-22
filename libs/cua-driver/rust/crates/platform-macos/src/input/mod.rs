// SPDX-License-Identifier: MIT
// Copyright (c) 2026 Cua AI, Inc.

//! Background input synthesis for macOS.
//!
//! Two strategies:
//! 1. **AX action** (`element_index` path): `AXUIElementPerformAction` — pure
//!    RPC, works on hidden/backgrounded windows, no cursor move, no focus steal.
//! 2. **CGEvent / SkyLight** (`x, y` path): synthesize CGEvents and post them
//!    to the target pid. Prefers `SLEventPostToPid` (SkyLight SPI) over the
//!    public `CGEventPostToPid` to reach Catalyst/Chromium apps and trigger
//!    the activity-monitor tickle required for live-input detection.

pub mod appkit_activation;
pub mod ax_actions;
pub mod electron_drag;
pub mod hover;
pub mod input_source;
pub mod interactive;
pub mod keyboard;
pub mod mouse;
pub mod pointer_toolkit;
pub mod skylight;

thread_local! {
    /// A blocking worker is not cancelled when its Tokio join handle drops.
    static INPUT_CALL: std::cell::RefCell<Option<std::sync::Weak<()>>> = const { std::cell::RefCell::new(None) };
}

pub(crate) async fn blocking_input<T: Send + 'static>(
    action: impl FnOnce() -> T + Send + 'static,
) -> Result<T, tokio::task::JoinError> {
    let owner = std::sync::Arc::new(());
    let permission = std::sync::Arc::downgrade(&owner);
    let result = tokio::task::spawn_blocking(move || {
        struct Clear;
        impl Drop for Clear {
            fn drop(&mut self) {
                INPUT_CALL.set(None);
            }
        }
        let _clear = Clear;
        INPUT_CALL.set(Some(permission));
        action()
    })
    .await;
    drop(owner);
    result
}

pub(super) fn input_cancelled() -> bool {
    INPUT_CALL.with_borrow(|call| call.as_ref().is_some_and(|owner| owner.strong_count() == 0))
}

pub use ax_actions::perform_ax_action;
pub use interactive::{
    GesturePhase, InteractiveDeliveryMode, InteractiveInputBatch, InteractiveInputConfig,
    InteractiveInputError, InteractiveInputEvent, InteractiveInputReceipt, InteractiveInputSession,
    KeyState, Modifier, PointerButton, PointerPhase,
};
pub use keyboard::{hotkey, press_key, type_text};
pub use mouse::click_at_xy;
