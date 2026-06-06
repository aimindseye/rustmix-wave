//! Optional Wi-Fi and SNTP runtime with hardware-independent snapshots.

use crate::{
    network_config::{NetworkConfig, WIFI_CONFIG_PATH},
    rtc::RtcDateTime,
};

/// Product-facing Wi-Fi state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum WifiConnectionState {
    Disabled,
    #[default]
    ConfigurationMissing,
    Connecting,
    Connected,
    Failed,
}

impl WifiConnectionState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::ConfigurationMissing => "NO CONFIG",
            Self::Connecting => "CONNECTING",
            Self::Connected => "CONNECTED",
            Self::Failed => "FAILED",
        }
    }
}

/// Product-facing SNTP synchronization state.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum NtpSyncState {
    Disabled,
    #[default]
    WaitingForWifi,
    Synchronizing,
    Synchronized,
    Failed,
}

impl NtpSyncState {
    #[must_use]
    pub const fn label(self) -> &'static str {
        match self {
            Self::Disabled => "DISABLED",
            Self::WaitingForWifi => "WAIT WIFI",
            Self::Synchronizing => "SYNCING",
            Self::Synchronized => "SYNCED",
            Self::Failed => "FAILED",
        }
    }
}

/// Serial-log fingerprint. RSSI is intentionally excluded so signal-strength
/// churn is reported only by the bounded heartbeat marker.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkLogFingerprint {
    pub wifi_state: WifiConnectionState,
    pub ntp_state: NtpSyncState,
    pub ssid: Option<String>,
    pub ipv4_address: Option<String>,
    pub last_sync_utc: Option<RtcDateTime>,
    pub error: Option<String>,
}

/// Rendering snapshot that never contains the Wi-Fi password.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NetworkSnapshot {
    pub wifi_state: WifiConnectionState,
    pub ntp_state: NtpSyncState,
    pub ssid: Option<String>,
    pub ipv4_address: Option<String>,
    pub rssi_dbm: Option<i32>,
    pub timezone_name: String,
    pub ntp_server: String,
    pub last_sync_utc: Option<RtcDateTime>,
    pub error: Option<String>,
}

impl Default for NetworkSnapshot {
    fn default() -> Self {
        Self {
            wifi_state: WifiConnectionState::ConfigurationMissing,
            ntp_state: NtpSyncState::WaitingForWifi,
            ssid: None,
            ipv4_address: None,
            rssi_dbm: None,
            timezone_name: "America/New_York".into(),
            ntp_server: "pool.ntp.org".into(),
            last_sync_utc: None,
            error: None,
        }
    }
}

impl NetworkSnapshot {
    /// Render a provisioned-but-not-yet-connected boot state before Wi-Fi is
    /// started after the first e-paper frame.
    #[must_use]
    pub fn provisioned(config: &NetworkConfig) -> Self {
        Self {
            wifi_state: WifiConnectionState::Connecting,
            ntp_state: NtpSyncState::WaitingForWifi,
            ssid: Some(config.ssid.clone()),
            timezone_name: config.timezone.clone(),
            ntp_server: config.ntp_server.clone(),
            ..Self::default()
        }
    }

    #[must_use]
    pub const fn home_badge(&self) -> &'static str {
        match (self.wifi_state, self.ntp_state) {
            (WifiConnectionState::Connected, NtpSyncState::Synchronized) => "NTP OK",
            (WifiConnectionState::Connected, _) => "WIFI OK",
            (WifiConnectionState::ConfigurationMissing, _) => "NO CFG",
            (WifiConnectionState::Connecting, _) => "WAIT",
            (WifiConnectionState::Disabled, _) => "OFF",
            (WifiConnectionState::Failed, _) => "FAILED",
        }
    }

    #[must_use]
    pub fn ssid_label(&self) -> &str {
        self.ssid.as_deref().unwrap_or("--")
    }

    #[must_use]
    pub fn ipv4_label(&self) -> &str {
        self.ipv4_address.as_deref().unwrap_or("--")
    }

    #[must_use]
    pub fn rssi_label(&self) -> String {
        self.rssi_dbm
            .map_or_else(|| "--".into(), |value| format!("{value} dBm"))
    }

    #[must_use]
    pub fn last_sync_label(&self) -> String {
        self.last_sync_utc.map_or_else(
            || "not synchronized".into(),
            |value| format!("{} UTC", value.date_time()),
        )
    }

    #[must_use]
    pub const fn config_path() -> &'static str {
        WIFI_CONFIG_PATH
    }

    /// Build a concise fingerprint for serial-marker rate limiting.
    #[must_use]
    pub fn log_fingerprint(&self) -> NetworkLogFingerprint {
        NetworkLogFingerprint {
            wifi_state: self.wifi_state,
            ntp_state: self.ntp_state,
            ssid: self.ssid.clone(),
            ipv4_address: self.ipv4_address.clone(),
            last_sync_utc: self.last_sync_utc,
            error: self.error.clone(),
        }
    }
}

#[cfg(target_os = "espidf")]
pub mod espidf {
    use std::time::{SystemTime, UNIX_EPOCH};

    use anyhow::{Context, Result};
    use embedded_svc::wifi::{AuthMethod, ClientConfiguration, Configuration};
    use esp_idf_svc::{
        eventloop::EspSystemEventLoop,
        hal::modem::WifiModemPeripheral,
        nvs::EspDefaultNvsPartition,
        sntp::{EspSntp, SntpConf},
        sys,
        wifi::{BlockingWifi, EspWifi},
    };

    use crate::{
        network::{NetworkSnapshot, NtpSyncState, WifiConnectionState},
        network_config::NetworkConfig,
        ntp::{utc_from_unix_seconds, MIN_VALID_SNTP_UNIX_SECONDS},
        rtc::RtcDateTime,
    };

    /// Own Wi-Fi and SNTP services for as long as the firmware is running.
    pub struct NetworkRuntime {
        wifi: Option<BlockingWifi<EspWifi<'static>>>,
        sntp: Option<EspSntp<'static>>,
        snapshot: NetworkSnapshot,
        ntp_reported: bool,
        suspended: bool,
    }

    impl NetworkRuntime {
        #[must_use]
        pub fn configuration_missing() -> Self {
            Self {
                wifi: None,
                sntp: None,
                snapshot: NetworkSnapshot::default(),
                ntp_reported: false,
                suspended: false,
            }
        }

        #[must_use]
        pub fn failed(config: &NetworkConfig, error: impl Into<String>) -> Self {
            Self {
                wifi: None,
                sntp: None,
                snapshot: NetworkSnapshot {
                    wifi_state: WifiConnectionState::Failed,
                    ntp_state: NtpSyncState::Failed,
                    ssid: Some(config.ssid.clone()),
                    timezone_name: config.timezone.clone(),
                    ntp_server: config.ntp_server.clone(),
                    error: Some(error.into()),
                    ..NetworkSnapshot::default()
                },
                ntp_reported: false,
                suspended: false,
            }
        }

        /// Start Wi-Fi after the initial e-paper frame is already visible.
        pub fn connect<M>(modem: M, config: &NetworkConfig) -> Result<Self>
        where
            M: WifiModemPeripheral + 'static,
        {
            let sys_loop = EspSystemEventLoop::take()?;
            let nvs = EspDefaultNvsPartition::take()?;
            let mut wifi =
                BlockingWifi::wrap(EspWifi::new(modem, sys_loop.clone(), Some(nvs))?, sys_loop)?;
            let auth_method = if config.password.is_empty() {
                AuthMethod::None
            } else {
                AuthMethod::WPA2Personal
            };
            wifi.set_configuration(&Configuration::Client(ClientConfiguration {
                ssid: config
                    .ssid
                    .as_str()
                    .try_into()
                    .context("SSID exceeds embedded Wi-Fi capacity")?,
                password: config
                    .password
                    .as_str()
                    .try_into()
                    .context("password exceeds embedded Wi-Fi capacity")?,
                auth_method,
                ..Default::default()
            }))?;
            wifi.start()?;
            wifi.connect()?;
            wifi.wait_netif_up()?;

            let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
            let mut conf = SntpConf::default();
            conf.servers[0] = config.ntp_server.as_str();
            let sntp = EspSntp::new(&conf)?;
            Ok(Self {
                wifi: Some(wifi),
                sntp: Some(sntp),
                snapshot: NetworkSnapshot {
                    wifi_state: WifiConnectionState::Connected,
                    ntp_state: NtpSyncState::Synchronizing,
                    ssid: Some(config.ssid.clone()),
                    ipv4_address: Some(format!("{}", ip_info.ip)),
                    rssi_dbm: read_rssi_dbm(),
                    timezone_name: config.timezone.clone(),
                    ntp_server: config.ntp_server.clone(),
                    last_sync_utc: None,
                    error: None,
                },
                ntp_reported: false,
                suspended: false,
            })
        }

        #[must_use]
        pub fn snapshot(&self) -> NetworkSnapshot {
            self.snapshot.clone()
        }

        #[must_use]
        pub const fn is_suspended(&self) -> bool {
            self.suspended
        }

        /// Stop optional network services while retaining station ownership so
        /// a later power-key or RTC-alarm wake can reconnect without rebuilding
        /// the complete application shell.
        pub fn suspend(&mut self) -> Result<()> {
            let _ = self.sntp.take();
            if let Some(wifi) = self.wifi.as_mut() {
                let _ = wifi.disconnect();
                wifi.stop()?;
            }
            self.snapshot.wifi_state = WifiConnectionState::Disabled;
            self.snapshot.ntp_state = NtpSyncState::Disabled;
            self.snapshot.ipv4_address = None;
            self.snapshot.rssi_dbm = None;
            self.snapshot.error = None;
            self.ntp_reported = false;
            self.suspended = true;
            Ok(())
        }

        /// Restart Wi-Fi association and SNTP after the wake frame is already
        /// visible. Failed recovery is non-fatal and remains visible in the
        /// product-facing network snapshot.
        pub fn resume(&mut self, config: &NetworkConfig) -> Result<()> {
            let wifi = self.wifi.as_mut().context("Wi-Fi runtime is unavailable")?;
            wifi.start()?;
            wifi.connect()?;
            wifi.wait_netif_up()?;
            let ip_info = wifi.wifi().sta_netif().get_ip_info()?;
            let mut conf = SntpConf::default();
            conf.servers[0] = config.ntp_server.as_str();
            self.sntp = Some(EspSntp::new(&conf)?);
            self.snapshot.wifi_state = WifiConnectionState::Connected;
            self.snapshot.ntp_state = NtpSyncState::Synchronizing;
            self.snapshot.ssid = Some(config.ssid.clone());
            self.snapshot.ipv4_address = Some(format!("{}", ip_info.ip));
            self.snapshot.rssi_dbm = read_rssi_dbm();
            self.snapshot.timezone_name = config.timezone.clone();
            self.snapshot.ntp_server = config.ntp_server.clone();
            self.snapshot.error = None;
            self.ntp_reported = false;
            self.suspended = false;
            Ok(())
        }

        pub fn record_resume_failure(&mut self, error: impl Into<String>) {
            self.snapshot.wifi_state = WifiConnectionState::Failed;
            self.snapshot.ntp_state = NtpSyncState::Failed;
            self.snapshot.ipv4_address = None;
            self.snapshot.rssi_dbm = None;
            self.snapshot.error = Some(error.into());
            self.suspended = false;
        }

        pub fn record_configuration_missing(&mut self) {
            self.snapshot = NetworkSnapshot::default();
            self.ntp_reported = false;
            self.suspended = false;
        }

        /// Poll for an SNTP-populated system clock. The official wrapper keeps
        /// the SNTP service alive and updates `SystemTime` in the background.
        pub fn tick(&mut self) -> Option<RtcDateTime> {
            if self.suspended {
                return None;
            }
            if self.wifi.is_some() {
                self.snapshot.rssi_dbm = read_rssi_dbm();
            }
            if self.ntp_reported || self.sntp.is_none() {
                return None;
            }
            let seconds = SystemTime::now().duration_since(UNIX_EPOCH).ok()?.as_secs();
            if seconds < MIN_VALID_SNTP_UNIX_SECONDS {
                return None;
            }
            let utc = utc_from_unix_seconds(seconds);
            self.snapshot.ntp_state = NtpSyncState::Synchronized;
            self.snapshot.last_sync_utc = Some(utc);
            self.ntp_reported = true;
            Some(utc)
        }
    }

    fn read_rssi_dbm() -> Option<i32> {
        let mut record = unsafe { core::mem::zeroed::<sys::wifi_ap_record_t>() };
        let status = unsafe { sys::esp_wifi_sta_get_ap_info(&mut record) };
        (status == sys::ESP_OK).then_some(i32::from(record.rssi))
    }
}

#[cfg(test)]
mod tests {
    use super::{NetworkSnapshot, NtpSyncState, WifiConnectionState};

    #[test]
    fn configuration_missing_snapshot_is_safe_for_home() {
        let snapshot = NetworkSnapshot::default();
        assert_eq!(
            snapshot.wifi_state,
            WifiConnectionState::ConfigurationMissing
        );
        assert_eq!(snapshot.ntp_state, NtpSyncState::WaitingForWifi);
        assert_eq!(snapshot.home_badge(), "NO CFG");
        assert_eq!(snapshot.ssid_label(), "--");
    }

    #[test]
    fn connected_and_synchronized_snapshot_has_ntp_badge() {
        let snapshot = NetworkSnapshot {
            wifi_state: WifiConnectionState::Connected,
            ntp_state: NtpSyncState::Synchronized,
            ..NetworkSnapshot::default()
        };
        assert_eq!(snapshot.home_badge(), "NTP OK");
    }
    #[test]
    fn log_fingerprint_ignores_rssi_churn() {
        let mut snapshot = NetworkSnapshot::default();
        snapshot.rssi_dbm = Some(-34);
        let first = snapshot.log_fingerprint();
        snapshot.rssi_dbm = Some(-61);
        assert_eq!(first, snapshot.log_fingerprint());
    }

    #[test]
    fn log_fingerprint_still_changes_for_ipv4_state() {
        let snapshot = NetworkSnapshot::default();
        let first = snapshot.log_fingerprint();
        let mut changed = snapshot;
        changed.ipv4_address = Some("192.0.2.10".into());
        assert_ne!(first, changed.log_fingerprint());
    }
}
