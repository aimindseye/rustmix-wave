#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SleepScreenMode {
    Dark,
    Light,
    CustomImage,
    RecentBook,
    None,
}

impl SleepScreenMode {
    pub const fn is_image_backed(self) -> bool {
        match self {
            Self::CustomImage | Self::RecentBook => true,
            Self::Dark | Self::Light | Self::None => false,
        }
    }

    pub const fn display_name(self) -> &'static str {
        match self {
            Self::Dark => "Dark",
            Self::Light => "Light",
            Self::CustomImage => "Custom image",
            Self::RecentBook => "Recent book",
            Self::None => "None",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SleepScreenSettings {
    pub mode: SleepScreenMode,
    pub fallback_mode: SleepScreenMode,
}

impl SleepScreenSettings {
    pub const fn default_static() -> Self {
        Self {
            mode: SleepScreenMode::CustomImage,
            fallback_mode: SleepScreenMode::Light,
        }
    }

    pub const fn default_disabled() -> Self {
        Self {
            mode: SleepScreenMode::Light,
            fallback_mode: SleepScreenMode::Light,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{SleepScreenMode, SleepScreenSettings};

    #[test]
    fn custom_image_is_image_backed() {
        assert!(SleepScreenMode::CustomImage.is_image_backed());
        assert!(!SleepScreenMode::Light.is_image_backed());
    }

    #[test]
    fn static_settings_fall_back_to_light_screen() {
        let settings = SleepScreenSettings::default_static();
        assert_eq!(settings.mode, SleepScreenMode::CustomImage);
        assert_eq!(settings.fallback_mode, SleepScreenMode::Light);
    }
}
