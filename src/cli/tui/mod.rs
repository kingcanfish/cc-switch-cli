pub mod event;
pub mod keymap;
pub mod runtime;
pub mod screen;
pub mod theme;
pub mod widgets;

pub use event::TuiEvent;
pub use runtime::{is_tui_active, set_tui_active, TerminalGuard, TuiRuntime};
pub use screen::{Screen, ScreenResult};
pub use widgets::{ConfirmScreen, ListScreen, MultiSelectScreen, TextInputScreen, TextViewScreen};
