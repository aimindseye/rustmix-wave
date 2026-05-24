use alloc::boxed::Box;
use alloc::vec::Vec;

use crate::hal::input::InputEvent;

pub enum ActivityCommand {
    None,
    Push(Box<dyn Activity>),
    Pop,
    Replace(Box<dyn Activity>),
    Home,
}

pub trait Activity {
    fn name(&self) -> &'static str;
    fn on_enter(&mut self) {}
    fn on_resume(&mut self) {}
    fn on_input(&mut self, _evt: InputEvent) -> ActivityCommand {
        ActivityCommand::None
    }
    fn on_exit(&mut self) {}
}

pub struct ActivityManager {
    stack: Vec<Box<dyn Activity>>,
}

impl Default for ActivityManager {
    fn default() -> Self {
        Self::new()
    }
}

impl ActivityManager {
    pub fn new() -> Self {
        Self { stack: Vec::new() }
    }

    pub fn push(&mut self, mut activity: Box<dyn Activity>) {
        activity.on_enter();
        self.stack.push(activity);
    }

    pub fn pop(&mut self) {
        if let Some(mut current) = self.stack.pop() {
            current.on_exit();
        }
        if let Some(current) = self.stack.last_mut() {
            current.on_resume();
        }
    }

    pub fn current_name(&self) -> Option<&'static str> {
        self.stack.last().map(|a| a.name())
    }
}
