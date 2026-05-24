use super::model::{AppAction, AppScreen, AppShell, ReaderSession};

#[derive(Debug, Clone, Default)]
pub struct ReaderState;

impl ReaderState {
    pub const fn new() -> Self {
        Self
    }

    pub fn ensure_placeholder_session(&mut self, shell: &mut AppShell) {
        if shell.reader_session().is_none() {
            shell.set_reader_session(ReaderSession::pending("/example.epub", true));
        }
    }

    pub fn handle_action(&mut self, shell: &mut AppShell, action: AppAction) -> Option<AppScreen> {
        match action {
            AppAction::Left | AppAction::Up => {
                if let Some(existing) = shell.reader_session().cloned() {
                    let mut next = existing;
                    next.current_page = next.current_page.saturating_sub(1);
                    next.handoff_pending = false;
                    shell.set_reader_session(next);
                }
                None
            }
            AppAction::Right | AppAction::Down => {
                if let Some(existing) = shell.reader_session().cloned() {
                    let mut next = existing;
                    next.current_page = next.current_page.saturating_add(1);
                    next.handoff_pending = false;
                    shell.set_reader_session(next);
                }
                None
            }
            AppAction::Back => {
                shell.set_screen(AppScreen::Browser);
                Some(AppScreen::Browser)
            }
            AppAction::Select | AppAction::None => None,
        }
    }
}
