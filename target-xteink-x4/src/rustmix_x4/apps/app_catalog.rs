//! Shared app catalog for native X4 apps.
//!
//! Rustmix keeps the built-in catalog deliberately small: reader firmware,
//! storage/network utilities, and optional SD-loaded Lua apps. Spiritual,
//! calendar-specific, or personal sample apps are not compiled in.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemAppId {
    Reader,
}

impl SystemAppId {
    pub const fn stable_key(self) -> &'static str {
        match self {
            Self::Reader => "reader",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SystemAppKind {
    Reading,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SystemAppDescriptor {
    pub id: SystemAppId,
    pub name: &'static str,
    pub summary: &'static str,
    pub kind: SystemAppKind,
    pub enabled: bool,
}

impl SystemAppDescriptor {
    pub const fn is_sleep_provider(self) -> bool {
        false
    }
}

pub const READER_APP: SystemAppDescriptor = SystemAppDescriptor {
    id: SystemAppId::Reader,
    name: "Reader",
    summary: "Read books and documents",
    kind: SystemAppKind::Reading,
    enabled: true,
};

pub const SYSTEM_APP_CATALOG: [SystemAppDescriptor; 1] = [READER_APP];

pub const fn system_app_count() -> usize {
    SYSTEM_APP_CATALOG.len()
}

pub const fn enabled_system_app_count() -> usize {
    let mut count = 0usize;
    let mut index = 0usize;
    while index < SYSTEM_APP_CATALOG.len() {
        if SYSTEM_APP_CATALOG[index].enabled {
            count += 1;
        }
        index += 1;
    }
    count
}

pub const fn app_by_id(id: SystemAppId) -> SystemAppDescriptor {
    match id {
        SystemAppId::Reader => READER_APP,
    }
}

pub const fn app_by_index(index: usize) -> Option<SystemAppDescriptor> {
    match index {
        0 => Some(READER_APP),
        _ => None,
    }
}

pub fn app_count_label() -> &'static str {
    match enabled_system_app_count() {
        0 => "0 apps",
        1 => "1 app",
        _ => "apps",
    }
}

#[cfg(test)]
mod tests {
    use super::{SystemAppId, app_by_index, app_count_label, enabled_system_app_count};

    #[test]
    fn catalog_contains_reader_only() {
        assert_eq!(enabled_system_app_count(), 1);
        assert_eq!(app_by_index(0).unwrap().id, SystemAppId::Reader);
        assert!(app_by_index(1).is_none());
    }

    #[test]
    fn home_count_label_matches_enabled_apps() {
        assert_eq!(app_count_label(), "1 app");
    }
}
