use alloc::string::String;
use core::fmt::Write as FmtWrite;

use embassy_futures::select::{Either, select};
use embassy_net::dns::DnsQueryType;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_time::{Duration, Timer};
use esp_hal::delay::Delay;
use esp_radio::wifi::{ClientConfig, Config, ModeConfig};
use log::info;

use crate::rustmix_x4::x4_apps::fonts;
use crate::rustmix_x4::x4_apps::fonts::bitmap::BitmapFont;
use crate::rustmix_x4::x4_apps::ui::{
    Alignment, BitmapLabel, ButtonFeedback, CONTENT_TOP, LARGE_MARGIN, Region, stack_fmt,
};
use crate::rustmix_x4::x4_kernel::board::action::{Action, ActionEvent, ButtonMapper};
use crate::rustmix_x4::x4_kernel::board::{Epd, SCREEN_H, SCREEN_W};
use crate::rustmix_x4::x4_kernel::drivers::sdcard::SdStorage;
use crate::rustmix_x4::x4_kernel::drivers::storage;
use crate::rustmix_x4::x4_kernel::drivers::strip::StripBuffer;
use crate::rustmix_x4::x4_kernel::kernel::config::WifiConfig;
use crate::rustmix_x4::x4_kernel::kernel::tasks;
use crate::rustmix_x4::x4_kernel::kernel::wake::uptime_secs;

use super::time_status;

const NTP_PORT: u16 = 123;
const WIFI_START_TIMEOUT_SECS: u64 = 8;
const WIFI_CONNECT_TIMEOUT_SECS: u64 = 12;
const DHCP_TIMEOUT_SECS: u64 = 12;
const DNS_TIMEOUT_SECS: u64 = 8;
const NTP_TIMEOUT_SECS: u64 = 8;
const NTP_SERVERS: [&str; 3] = ["pool.ntp.org", "time.google.com", "time.cloudflare.com"];

const HEADING_X: u16 = LARGE_MARGIN;
const HEADING_W: u16 = SCREEN_W - HEADING_X * 2;
const BODY_X: u16 = 24;
const BODY_W: u16 = SCREEN_W - BODY_X * 2;
const BODY_LINE_GAP: u16 = 10;
const FOOTER_Y: u16 = SCREEN_H - 60;
const TIME_STATUS_BUF_LEN: usize = 512;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NetworkTimeError {
    MissingCredentials,
    RadioInit,
    WifiInit,
    WifiConfig,
    WifiStart,
    WifiConnect,
    DhcpTimeout,
    DnsFailed,
    UdpBind,
    UdpSend,
    Timeout,
    InvalidPacket,
    Cancelled,
}

impl NetworkTimeError {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::MissingCredentials => "missing Wi-Fi",
            Self::RadioInit => "radio init failed",
            Self::WifiInit => "Wi-Fi init failed",
            Self::WifiConfig => "Wi-Fi config failed",
            Self::WifiStart => "Wi-Fi start failed",
            Self::WifiConnect => "Wi-Fi connect failed",
            Self::DhcpTimeout => "DHCP timeout",
            Self::DnsFailed => "DNS failed",
            Self::UdpBind => "UDP bind failed",
            Self::UdpSend => "NTP send failed",
            Self::Timeout => "NTP timeout",
            Self::InvalidPacket => "invalid NTP packet",
            Self::Cancelled => "cancelled",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct NetworkTimeResult {
    pub unix: u64,
    pub ip: Option<[u8; 4]>,
}

pub async fn sync_now(wifi_cfg: &WifiConfig) -> Result<NetworkTimeResult, NetworkTimeError> {
    if !wifi_cfg.has_credentials() || wifi_cfg.password().is_empty() {
        return Err(NetworkTimeError::MissingCredentials);
    }

    let wifi = unsafe { esp_hal::peripherals::WIFI::steal() };
    let radio = esp_radio::init().map_err(|_| NetworkTimeError::RadioInit)?;
    let (mut wifi_ctrl, interfaces) = esp_radio::wifi::new(&radio, wifi, Config::default())
        .map_err(|_| NetworkTimeError::WifiInit)?;

    let client_cfg = ClientConfig::default()
        .with_ssid(String::from(wifi_cfg.ssid()))
        .with_password(String::from(wifi_cfg.password()));
    wifi_ctrl
        .set_config(&ModeConfig::Client(client_cfg))
        .map_err(|_| NetworkTimeError::WifiConfig)?;
    match select(
        wifi_ctrl.start_async(),
        Timer::after(Duration::from_secs(WIFI_START_TIMEOUT_SECS)),
    )
    .await
    {
        Either::First(Ok(())) => {}
        Either::First(Err(_)) => return Err(NetworkTimeError::WifiStart),
        Either::Second(_) => return Err(NetworkTimeError::Timeout),
    }

    info!("time: connecting to configured Wi-Fi");
    match select(
        wifi_ctrl.connect_async(),
        Timer::after(Duration::from_secs(WIFI_CONNECT_TIMEOUT_SECS)),
    )
    .await
    {
        Either::First(Ok(())) => {}
        Either::First(Err(_)) => return Err(NetworkTimeError::WifiConnect),
        Either::Second(_) => return Err(NetworkTimeError::Timeout),
    }

    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = {
        let rng = esp_hal::rng::Rng::new();
        (rng.random() as u64) << 32 | rng.random() as u64
    };
    let mut resources = embassy_net::StackResources::<4>::new();
    let (stack, mut runner) = embassy_net::new(interfaces.sta, net_config, &mut resources, seed);

    match select(runner.run(), query_after_dhcp(stack)).await {
        Either::Second(result) => result,
        Either::First(_) => unreachable!(),
    }
}

pub async fn run_time_sync_mode(
    wifi: esp_hal::peripherals::WIFI<'static>,
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    sd: &SdStorage,
    ui_font_size_idx: u8,
    bumps: &ButtonFeedback,
    wifi_cfg: &WifiConfig,
) {
    let heading = fonts::heading_font(ui_font_size_idx);
    let body = fonts::chrome_font();

    if !wifi_cfg.has_credentials() || wifi_cfg.password().is_empty() {
        render_time_screen(
            epd,
            strip,
            delay,
            heading,
            body,
            &[
                "Wi-Fi credentials missing",
                "Set wifi_ssid and wifi_pass in",
                "_x4/SETTINGS.TXT",
                "Password is never shown",
            ],
            Some("Press BACK to exit"),
            bumps,
            false,
        )
        .await;
        drain_until_back().await;
        return;
    }

    {
        let mut msg_buf = [0u8; 80];
        let msg_len = stack_fmt(&mut msg_buf, |w| {
            let _ = write!(w, "Connecting to '{}'...", wifi_cfg.ssid());
        });
        let msg = core::str::from_utf8(&msg_buf[..msg_len]).unwrap_or("Connecting Wi-Fi...");
        render_time_screen(
            epd,
            strip,
            delay,
            heading,
            body,
            &[msg, "Press BACK to cancel"],
            None,
            bumps,
            true,
        )
        .await;
    }

    let result =
        sync_now_with_peripheral(wifi, wifi_cfg, epd, strip, delay, heading, body, bumps).await;
    let now_uptime = uptime_secs();

    match result {
        Ok(result) => {
            let cache = time_status::TimeCache::synced(result.unix, now_uptime, result.ip, "ntp");
            let mut buf = [0u8; TIME_STATUS_BUF_LEN];
            let n = time_status::write_time_txt(&cache, &mut buf);
            let save_ok = storage::write_in_x4(sd, time_status::TIME_FILE, &buf[..n]).is_ok();

            let mut time_buf = [0u8; 72];
            let time_len = stack_fmt(&mut time_buf, |w| {
                let _ = write!(w, "Time: ");
                let _ = cache.write_time_value(now_uptime, w);
            });
            let time_line = core::str::from_utf8(&time_buf[..time_len]).unwrap_or("Time synced");

            let mut date_buf = [0u8; 96];
            let date_len = stack_fmt(&mut date_buf, |w| {
                let _ = write!(w, "Date: ");
                let _ = cache.write_date_value(now_uptime, w);
            });
            let date_line = core::str::from_utf8(&date_buf[..date_len]).unwrap_or("Date synced");

            let status = if save_ok {
                "Saved to /RUSTMIX/TIME.TXT"
            } else {
                "Synced, save failed"
            };
            render_time_screen(
                epd,
                strip,
                delay,
                heading,
                body,
                &["Time synced", time_line, date_line, status],
                Some("Press BACK to exit"),
                bumps,
                false,
            )
            .await;
            drain_until_back().await;
        }
        Err(NetworkTimeError::Cancelled) => {}
        Err(err) => {
            let cache = load_time_cache_from_sd(sd).with_error(err.as_str(), now_uptime);
            let mut buf = [0u8; TIME_STATUS_BUF_LEN];
            let n = time_status::write_time_txt(&cache, &mut buf);
            let _ = storage::write_in_x4(sd, time_status::TIME_FILE, &buf[..n]);

            let mut err_buf = [0u8; 72];
            let err_len = stack_fmt(&mut err_buf, |w| {
                let _ = write!(w, "Sync failed: {}", err.as_str());
            });
            let err_line = core::str::from_utf8(&err_buf[..err_len]).unwrap_or("Sync failed");
            render_time_screen(
                epd,
                strip,
                delay,
                heading,
                body,
                &[
                    err_line,
                    "Wi-Fi Transfer can still be used",
                    "Try again later",
                ],
                Some("Press BACK to exit"),
                bumps,
                false,
            )
            .await;
            drain_until_back().await;
        }
    }
}

pub async fn sync_existing_stack(
    stack: embassy_net::Stack<'_>,
) -> Result<NetworkTimeResult, NetworkTimeError> {
    let ip = stack.config_v4().map(|cfg| cfg.address.address().octets());

    for server in NTP_SERVERS {
        if let Ok(unix) = query_server(stack, server).await {
            return Ok(NetworkTimeResult { unix, ip });
        }
    }

    Err(NetworkTimeError::DnsFailed)
}

pub fn save_time_result_to_sd(
    sd: &SdStorage,
    result: NetworkTimeResult,
    uptime_secs: u32,
    source: &'static str,
) -> bool {
    let cache = time_status::TimeCache::synced(result.unix, uptime_secs, result.ip, source);
    let mut buf = [0u8; TIME_STATUS_BUF_LEN];
    let n = time_status::write_time_txt(&cache, &mut buf);
    storage::write_in_x4(sd, time_status::TIME_FILE, &buf[..n]).is_ok()
}

pub fn save_time_error_to_sd(sd: &SdStorage, error: NetworkTimeError, uptime_secs: u32) -> bool {
    let cache = load_time_cache_from_sd(sd).with_error(error.as_str(), uptime_secs);
    let mut buf = [0u8; TIME_STATUS_BUF_LEN];
    let n = time_status::write_time_txt(&cache, &mut buf);
    storage::write_in_x4(sd, time_status::TIME_FILE, &buf[..n]).is_ok()
}

fn load_time_cache_from_sd(sd: &SdStorage) -> time_status::TimeCache {
    let mut buf = [0u8; TIME_STATUS_BUF_LEN];
    match storage::read_chunk_in_x4(sd, time_status::TIME_FILE, 0, &mut buf) {
        Ok(n) if n > 0 => time_status::parse_time_txt(&buf[..n]),
        _ => time_status::TimeCache::default(),
    }
}

async fn sync_now_with_peripheral(
    wifi: esp_hal::peripherals::WIFI<'static>,
    wifi_cfg: &WifiConfig,
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    bumps: &ButtonFeedback,
) -> Result<NetworkTimeResult, NetworkTimeError> {
    let radio = esp_radio::init().map_err(|_| NetworkTimeError::RadioInit)?;
    let (mut wifi_ctrl, interfaces) = esp_radio::wifi::new(&radio, wifi, Config::default())
        .map_err(|_| NetworkTimeError::WifiInit)?;

    let client_cfg = ClientConfig::default()
        .with_ssid(String::from(wifi_cfg.ssid()))
        .with_password(String::from(wifi_cfg.password()));
    wifi_ctrl
        .set_config(&ModeConfig::Client(client_cfg))
        .map_err(|_| NetworkTimeError::WifiConfig)?;

    match select(
        wifi_ctrl.start_async(),
        select(
            Timer::after(Duration::from_secs(WIFI_START_TIMEOUT_SECS)),
            drain_until_back(),
        ),
    )
    .await
    {
        Either::First(Ok(())) => {}
        Either::First(Err(_)) => return Err(NetworkTimeError::WifiStart),
        Either::Second(Either::First(_)) => return Err(NetworkTimeError::Timeout),
        Either::Second(Either::Second(_)) => return Err(NetworkTimeError::Cancelled),
    }

    match select(
        wifi_ctrl.connect_async(),
        select(
            Timer::after(Duration::from_secs(WIFI_CONNECT_TIMEOUT_SECS)),
            drain_until_back(),
        ),
    )
    .await
    {
        Either::First(Ok(())) => {}
        Either::First(Err(_)) => return Err(NetworkTimeError::WifiConnect),
        Either::Second(Either::First(_)) => return Err(NetworkTimeError::Timeout),
        Either::Second(Either::Second(_)) => return Err(NetworkTimeError::Cancelled),
    }

    render_time_screen(
        epd,
        strip,
        delay,
        heading,
        body,
        &[
            "Wi-Fi connected",
            "Waiting for DHCP...",
            "Press BACK to cancel",
        ],
        None,
        bumps,
        false,
    )
    .await;

    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = {
        let rng = esp_hal::rng::Rng::new();
        (rng.random() as u64) << 32 | rng.random() as u64
    };
    let mut resources = embassy_net::StackResources::<4>::new();
    let (stack, mut runner) = embassy_net::new(interfaces.sta, net_config, &mut resources, seed);

    match select(runner.run(), query_after_dhcp_or_cancel(stack)).await {
        Either::Second(result) => result,
        Either::First(_) => unreachable!(),
    }
}

async fn query_after_dhcp(
    stack: embassy_net::Stack<'_>,
) -> Result<NetworkTimeResult, NetworkTimeError> {
    match select(
        stack.wait_config_up(),
        Timer::after(Duration::from_secs(DHCP_TIMEOUT_SECS)),
    )
    .await
    {
        Either::First(_) => {}
        Either::Second(_) => return Err(NetworkTimeError::DhcpTimeout),
    }

    let ip = stack.config_v4().map(|cfg| cfg.address.address().octets());

    for server in NTP_SERVERS {
        if let Ok(unix) = query_server(stack, server).await {
            return Ok(NetworkTimeResult { unix, ip });
        }
    }

    Err(NetworkTimeError::DnsFailed)
}

async fn query_after_dhcp_or_cancel(
    stack: embassy_net::Stack<'_>,
) -> Result<NetworkTimeResult, NetworkTimeError> {
    match select(
        stack.wait_config_up(),
        select(
            Timer::after(Duration::from_secs(DHCP_TIMEOUT_SECS)),
            drain_until_back(),
        ),
    )
    .await
    {
        Either::First(_) => {}
        Either::Second(Either::First(_)) => return Err(NetworkTimeError::DhcpTimeout),
        Either::Second(Either::Second(_)) => return Err(NetworkTimeError::Cancelled),
    }

    let ip = stack.config_v4().map(|cfg| cfg.address.address().octets());

    for server in NTP_SERVERS {
        if let Ok(unix) = query_server_or_cancel(stack, server).await {
            return Ok(NetworkTimeResult { unix, ip });
        }
    }

    Err(NetworkTimeError::DnsFailed)
}

async fn query_server(
    stack: embassy_net::Stack<'_>,
    server: &str,
) -> Result<u64, NetworkTimeError> {
    let dns = embassy_net::dns::DnsSocket::new(stack);
    let addresses = match select(
        dns.query(server, DnsQueryType::A),
        Timer::after(Duration::from_secs(DNS_TIMEOUT_SECS)),
    )
    .await
    {
        Either::First(Ok(addresses)) => addresses,
        Either::First(Err(_)) => return Err(NetworkTimeError::DnsFailed),
        Either::Second(_) => return Err(NetworkTimeError::DnsFailed),
    };

    for address in addresses {
        if let Ok(unix) = query_address(stack, address).await {
            return Ok(unix);
        }
    }
    Err(NetworkTimeError::DnsFailed)
}

async fn query_server_or_cancel(
    stack: embassy_net::Stack<'_>,
    server: &str,
) -> Result<u64, NetworkTimeError> {
    let dns = embassy_net::dns::DnsSocket::new(stack);
    let addresses = match select(
        dns.query(server, DnsQueryType::A),
        select(
            Timer::after(Duration::from_secs(DNS_TIMEOUT_SECS)),
            drain_until_back(),
        ),
    )
    .await
    {
        Either::First(Ok(addresses)) => addresses,
        Either::First(Err(_)) => return Err(NetworkTimeError::DnsFailed),
        Either::Second(Either::First(_)) => return Err(NetworkTimeError::DnsFailed),
        Either::Second(Either::Second(_)) => return Err(NetworkTimeError::Cancelled),
    };

    for address in addresses {
        if let Ok(unix) = query_address_or_cancel(stack, address).await {
            return Ok(unix);
        }
    }
    Err(NetworkTimeError::DnsFailed)
}

async fn query_address(
    stack: embassy_net::Stack<'_>,
    address: embassy_net::IpAddress,
) -> Result<u64, NetworkTimeError> {
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx_buf = [0u8; 128];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buf = [0u8; 64];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    socket.bind(0).map_err(|_| NetworkTimeError::UdpBind)?;

    let mut request = [0u8; 48];
    request[0] = 0x23;
    let endpoint = embassy_net::IpEndpoint::new(address, NTP_PORT);
    socket
        .send_to(&request, endpoint)
        .await
        .map_err(|_| NetworkTimeError::UdpSend)?;

    let mut response = [0u8; 64];
    let (n, _) = match select(
        socket.recv_from(&mut response),
        Timer::after(Duration::from_secs(NTP_TIMEOUT_SECS)),
    )
    .await
    {
        Either::First(Ok(result)) => result,
        Either::First(Err(_)) => return Err(NetworkTimeError::Timeout),
        Either::Second(_) => return Err(NetworkTimeError::Timeout),
    };

    time_status::parse_ntp_unix_seconds(&response[..n]).ok_or(NetworkTimeError::InvalidPacket)
}

async fn query_address_or_cancel(
    stack: embassy_net::Stack<'_>,
    address: embassy_net::IpAddress,
) -> Result<u64, NetworkTimeError> {
    let mut rx_meta = [PacketMetadata::EMPTY; 1];
    let mut rx_buf = [0u8; 128];
    let mut tx_meta = [PacketMetadata::EMPTY; 1];
    let mut tx_buf = [0u8; 64];
    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);
    socket.bind(0).map_err(|_| NetworkTimeError::UdpBind)?;

    let mut request = [0u8; 48];
    request[0] = 0x23;
    let endpoint = embassy_net::IpEndpoint::new(address, NTP_PORT);
    match select(socket.send_to(&request, endpoint), drain_until_back()).await {
        Either::First(Ok(())) => {}
        Either::First(Err(_)) => return Err(NetworkTimeError::UdpSend),
        Either::Second(_) => return Err(NetworkTimeError::Cancelled),
    }

    let mut response = [0u8; 64];
    let (n, _) = match select(
        socket.recv_from(&mut response),
        select(
            Timer::after(Duration::from_secs(NTP_TIMEOUT_SECS)),
            drain_until_back(),
        ),
    )
    .await
    {
        Either::First(Ok(result)) => result,
        Either::First(Err(_)) => return Err(NetworkTimeError::Timeout),
        Either::Second(Either::First(_)) => return Err(NetworkTimeError::Timeout),
        Either::Second(Either::Second(_)) => return Err(NetworkTimeError::Cancelled),
    };

    time_status::parse_ntp_unix_seconds(&response[..n]).ok_or(NetworkTimeError::InvalidPacket)
}

async fn drain_until_back() {
    let mapper = ButtonMapper::new();
    loop {
        let hw = tasks::INPUT_EVENTS.receive().await;
        let ev = mapper.map_event(hw);
        if matches!(
            ev,
            ActionEvent::Press(Action::Back) | ActionEvent::LongPress(Action::Back)
        ) {
            return;
        }
    }
}

async fn render_time_screen(
    epd: &mut Epd,
    strip: &mut StripBuffer,
    delay: &mut Delay,
    heading: &'static BitmapFont,
    body: &'static BitmapFont,
    lines: &[&str],
    footer: Option<&str>,
    bumps: &ButtonFeedback,
    full_refresh: bool,
) {
    let heading_h = heading.line_height;
    let body_h = body.line_height;
    let body_stride = body_h + BODY_LINE_GAP;

    let heading_region = Region::new(HEADING_X, CONTENT_TOP + 12, HEADING_W, heading_h);
    let body_area_top = CONTENT_TOP + 12 + heading_h + 40;
    let body_area_bottom = FOOTER_Y.saturating_sub(20);
    let body_area_h = body_area_bottom.saturating_sub(body_area_top);
    let total_body_h = if lines.is_empty() {
        0
    } else {
        (lines.len() as u16 - 1) * body_stride + body_h
    };
    let body_start_y = body_area_top + body_area_h.saturating_sub(total_body_h) / 2;
    let footer_region = Region::new(BODY_X, FOOTER_Y, BODY_W, body_h);

    let draw = |s: &mut StripBuffer| {
        BitmapLabel::new(heading_region, "Date & Time", heading)
            .alignment(Alignment::CenterLeft)
            .draw(s)
            .unwrap();

        for (i, line) in lines.iter().enumerate() {
            if line.is_empty() {
                continue;
            }
            let y = body_start_y + (i as u16) * body_stride;
            let region = Region::new(BODY_X, y, BODY_W, body_h);
            BitmapLabel::new(region, line, body)
                .alignment(Alignment::Center)
                .draw(s)
                .unwrap();
        }

        if let Some(text) = footer {
            BitmapLabel::new(footer_region, text, body)
                .alignment(Alignment::Center)
                .draw(s)
                .unwrap();
        }

        bumps.draw(s);
    };

    if full_refresh {
        epd.full_refresh_async(strip, delay, &draw).await;
    } else {
        epd.partial_refresh_async(strip, delay, 0, 0, SCREEN_W, SCREEN_H, &draw)
            .await;
    }
}
