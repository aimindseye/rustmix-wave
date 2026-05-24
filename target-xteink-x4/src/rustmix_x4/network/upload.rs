// Rustmix Wi-Fi transfer server: HTTP file upload + mDNS (x4.local)

use alloc::string::String;
use core::fmt::Write as FmtWrite;

use embassy_futures::select::{Either, select};
use embassy_net::IpListenEndpoint;
use embassy_net::tcp::TcpSocket;
use embassy_net::udp::{PacketMetadata, UdpSocket};
use embassy_time::{Duration, Timer};
use embedded_io_async::Write as AsyncWrite;
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

use super::network_time;

const HEADING_X: u16 = LARGE_MARGIN;
const HEADING_W: u16 = SCREEN_W - HEADING_X * 2;

const BODY_X: u16 = 24;
const BODY_W: u16 = SCREEN_W - BODY_X * 2;
const BODY_LINE_GAP: u16 = 10;
const FOOTER_Y: u16 = SCREEN_H - 60;

const HTTP_200_HTML: &[u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/html; charset=utf-8\r\nConnection: close\r\n\r\n";
const HTTP_200_JSON: &[u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: application/json\r\nAccess-Control-Allow-Origin: *\r\nConnection: close\r\n\r\n";
const HTTP_200_TEXT: &[u8] =
    b"HTTP/1.0 200 OK\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n";
const HTTP_500_TEXT: &[u8] =
    b"HTTP/1.0 500 Internal Server Error\r\nContent-Type: text/plain\r\nConnection: close\r\n\r\n";
const HTTP_404: &[u8] = b"HTTP/1.0 404 Not Found\r\nConnection: close\r\n\r\nNot Found";

const UPLOAD_PAGE: &[u8] = include_bytes!("upload.html");

const MDNS_PORT: u16 = 5353;

// "x4.local" in DNS wire format: length-prefixed labels + NUL
const HOSTNAME_WIRE: [u8; 10] = [2, b'x', b'4', 5, b'l', b'o', b'c', b'a', b'l', 0];

const MDNS_RESPONSE_LEN: usize = 12 + HOSTNAME_WIRE.len() + 2 + 2 + 4 + 2 + 4;

const MAX_BOUNDARY_LEN: usize = 120;
const WORK_BUF_SIZE: usize = 2048;

// TCP buffer sizes

const TCP_RX_BUF_SIZE: usize = 2048;
const TCP_TX_BUF_SIZE: usize = 1536;

const HTTP_HEADER_BUF_SIZE: usize = 1024;

const DIR_LIST_MAX: usize = 64;
const MANAGER_PATH_BUF_LEN: usize = 128;
const MANAGER_MAX_PATH_COMPONENTS: usize = 8;
const WIFI_V2_MAX_CHUNK: usize = 1536;

// HTTP timing
const HTTP_TIMEOUT_SECS: u64 = 30;
const ACCEPT_RETRY_MS: u64 = 200;

const SOCKET_CLOSE_DELAY_MS: u64 = 50;

const MDNS_BIND_RETRY_MS: u64 = 100;

enum ServerEvent {
    Nothing,
    Uploaded { name: [u8; 13], name_len: u8 },
    UploadFailed,
    TimeSynced,
    TimeSyncSaveFailed,
    TimeSyncFailed,
}

pub async fn run_upload_mode(
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
        render_screen(
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

    let ssid = wifi_cfg.ssid();
    let password = wifi_cfg.password();

    {
        let mut msg_buf = [0u8; 64];
        let msg_len = stack_fmt(&mut msg_buf, |w| {
            let _ = write!(w, "Connecting to '{}'...", ssid);
        });
        let msg = core::str::from_utf8(&msg_buf[..msg_len]).unwrap_or("Connecting...");
        render_screen(epd, strip, delay, heading, body, &[msg], None, bumps, true).await;
    }

    let radio = match esp_radio::init() {
        Ok(r) => r,
        Err(e) => {
            info!("upload: radio init failed: {:?}", e);
            render_screen(
                epd,
                strip,
                delay,
                heading,
                body,
                &["Radio init failed", "Back exits; retry later"],
                Some("Press BACK to exit"),
                bumps,
                false,
            )
            .await;
            drain_until_back().await;
            return;
        }
    };

    let (mut wifi_ctrl, interfaces) = match esp_radio::wifi::new(&radio, wifi, Config::default()) {
        Ok(pair) => pair,
        Err(e) => {
            info!("upload: wifi::new failed: {:?}", e);
            render_screen(
                epd,
                strip,
                delay,
                heading,
                body,
                &["Wi-Fi init failed", "Back exits; retry later"],
                Some("Press BACK to exit"),
                bumps,
                false,
            )
            .await;
            drain_until_back().await;
            return;
        }
    };

    let client_cfg = ClientConfig::default()
        .with_ssid(String::from(ssid))
        .with_password(String::from(password));

    if let Err(e) = wifi_ctrl.set_config(&ModeConfig::Client(client_cfg)) {
        info!("upload: set_config failed: {:?}", e);
        render_screen(
            epd,
            strip,
            delay,
            heading,
            body,
            &["Wi-Fi config failed", "Check settings file"],
            Some("Press BACK to exit"),
            bumps,
            false,
        )
        .await;
        drain_until_back().await;
        return;
    }

    if let Err(e) = wifi_ctrl.start_async().await {
        info!("upload: start failed: {:?}", e);
        render_screen(
            epd,
            strip,
            delay,
            heading,
            body,
            &["Wi-Fi start failed", "Back exits; retry later"],
            Some("Press BACK to exit"),
            bumps,
            false,
        )
        .await;
        drain_until_back().await;
        return;
    }

    info!("upload: wifi started, connecting to '{}'", ssid);

    if let Err(e) = wifi_ctrl.connect_async().await {
        info!("upload: connect failed: {:?}", e);
        render_screen(
            epd,
            strip,
            delay,
            heading,
            body,
            &[
                "Connection failed",
                "Check SSID/password",
                "Back exits; retry later",
            ],
            Some("Press BACK to exit"),
            bumps,
            false,
        )
        .await;
        drain_until_back().await;
        return;
    }

    info!("upload: connected to '{}'", ssid);

    let net_config = embassy_net::Config::dhcpv4(Default::default());
    let seed = {
        let rng = esp_hal::rng::Rng::new();
        (rng.random() as u64) << 32 | rng.random() as u64
    };

    let mut resources = embassy_net::StackResources::<8>::new();
    let (stack, mut runner) = embassy_net::new(interfaces.sta, net_config, &mut resources, seed);

    let got_ip = match select(
        runner.run(),
        select(stack.wait_config_up(), drain_until_back()),
    )
    .await
    {
        Either::Second(Either::First(_)) => true,
        Either::Second(Either::Second(_)) => false,
        _ => unreachable!(),
    };

    if !got_ip {
        info!("upload: user exited during DHCP");
        return;
    }

    let ip_octets: [u8; 4] = if let Some(cfg) = stack.config_v4() {
        cfg.address.address().octets()
    } else {
        [0, 0, 0, 0]
    };

    let mut ip_buf = [0u8; 48];
    let ip_len = stack_fmt(&mut ip_buf, |w| {
        let _ = write!(
            w,
            "({}.{}.{}.{})",
            ip_octets[0], ip_octets[1], ip_octets[2], ip_octets[3]
        );
    });
    let ip_str = core::str::from_utf8(&ip_buf[..ip_len]).unwrap_or("???");

    let mut time_sync_done = false;
    let sync_line = "Time sync: pending";

    info!(
        "upload: serving at http://x4.local/  ({})",
        core::str::from_utf8(&ip_buf[1..ip_len.saturating_sub(1)]).unwrap_or("?")
    );

    render_screen(
        epd,
        strip,
        delay,
        heading,
        body,
        &["http://x4.local/", ip_str, sync_line],
        Some("Press BACK to exit"),
        bumps,
        false,
    )
    .await;

    let mut rx_buf = [0u8; TCP_RX_BUF_SIZE];
    let mut tx_buf = [0u8; TCP_TX_BUF_SIZE];

    loop {
        let event = if time_sync_done {
            let inner_result = match select(
                runner.run(),
                select(
                    select(
                        serve_one_request(stack, &mut rx_buf, &mut tx_buf, sd),
                        mdns_respond_once(stack, ip_octets),
                    ),
                    drain_until_back(),
                ),
            )
            .await
            {
                Either::Second(Either::First(inner)) => inner,
                Either::Second(Either::Second(_)) => break, // back pressed
                _ => unreachable!(),
            };

            match inner_result {
                Either::First(ev) => ev,
                Either::Second(()) => ServerEvent::Nothing,
            }
        } else {
            match select(
                runner.run(),
                select(
                    select(
                        select(
                            serve_one_request(stack, &mut rx_buf, &mut tx_buf, sd),
                            mdns_respond_once(stack, ip_octets),
                        ),
                        network_time::sync_existing_stack(stack),
                    ),
                    drain_until_back(),
                ),
            )
            .await
            {
                Either::Second(Either::First(Either::First(Either::First(ev)))) => ev,
                Either::Second(Either::First(Either::First(Either::Second(())))) => {
                    ServerEvent::Nothing
                }
                Either::Second(Either::First(Either::Second(Ok(result)))) => {
                    time_sync_done = true;
                    let save_ok =
                        network_time::save_time_result_to_sd(sd, result, uptime_secs(), "transfer");
                    if save_ok {
                        ServerEvent::TimeSynced
                    } else {
                        ServerEvent::TimeSyncSaveFailed
                    }
                }
                Either::Second(Either::First(Either::Second(Err(error)))) => {
                    time_sync_done = true;
                    let _ = network_time::save_time_error_to_sd(sd, error, uptime_secs());
                    ServerEvent::TimeSyncFailed
                }
                Either::Second(Either::Second(_)) => break, // back pressed
                _ => unreachable!(),
            }
        };

        match event {
            ServerEvent::Uploaded { name, name_len } => {
                let fname = core::str::from_utf8(&name[..name_len as usize]).unwrap_or("???");
                info!("upload: file saved as '{}'", fname);
            }
            ServerEvent::UploadFailed => {
                info!("upload: file upload failed");
            }
            ServerEvent::TimeSynced => {
                info!("upload: transfer-start time sync saved");
                render_screen(
                    epd,
                    strip,
                    delay,
                    heading,
                    body,
                    &["http://x4.local/", ip_str, "Time sync: updated"],
                    Some("Press BACK to exit"),
                    bumps,
                    false,
                )
                .await;
            }
            ServerEvent::TimeSyncSaveFailed => {
                info!("upload: transfer-start time sync succeeded but save failed");
            }
            ServerEvent::TimeSyncFailed => {
                info!("upload: transfer-start time sync failed; server remains active");
            }
            ServerEvent::Nothing => {}
        }
    }

    info!("upload: exiting, tearing down WiFi");
}

async fn serve_one_request(
    stack: embassy_net::Stack<'_>,
    rx_buf: &mut [u8],
    tx_buf: &mut [u8],
    sd: &SdStorage,
) -> ServerEvent
where
{
    let mut socket = TcpSocket::new(stack, rx_buf, tx_buf);
    socket.set_timeout(Some(Duration::from_secs(HTTP_TIMEOUT_SECS)));

    if socket
        .accept(IpListenEndpoint {
            addr: None,
            port: 80,
        })
        .await
        .is_err()
    {
        Timer::after(Duration::from_millis(ACCEPT_RETRY_MS)).await;
        return ServerEvent::Nothing;
    }

    let mut hdr = [0u8; HTTP_HEADER_BUF_SIZE];
    let mut hdr_len = 0usize;

    loop {
        match socket.read(&mut hdr[hdr_len..]).await {
            Ok(0) => {
                close_socket(&mut socket).await;
                return ServerEvent::Nothing;
            }
            Ok(n) => {
                hdr_len += n;
                if hdr[..hdr_len].windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if hdr_len >= hdr.len() {
                    let _ = socket
                        .write_all(b"HTTP/1.0 431 Headers Too Large\r\n\r\n")
                        .await;
                    close_socket(&mut socket).await;
                    return ServerEvent::Nothing;
                }
            }
            Err(_) => {
                close_socket(&mut socket).await;
                return ServerEvent::Nothing;
            }
        }
    }

    let headers_end = match find_subsequence(&hdr[..hdr_len], b"\r\n\r\n") {
        Some(p) => p,
        None => {
            close_socket(&mut socket).await;
            return ServerEvent::Nothing;
        }
    };
    let body_offset = headers_end + 4;
    let initial_body = &hdr[body_offset..hdr_len];
    let headers = &hdr[..headers_end];

    let first_line_end = headers
        .iter()
        .position(|&b| b == b'\r')
        .unwrap_or(headers.len());
    let request_line = &headers[..first_line_end];

    let is_get = request_line.starts_with(b"GET ");
    let is_post = request_line.starts_with(b"POST ");

    let path = extract_path(request_line);
    let target = extract_target(request_line);

    if is_get && path == b"/" {
        let _ = socket.write_all(HTTP_200_HTML).await;
        let _ = socket.write_all(UPLOAD_PAGE).await;
        let _ = socket.flush().await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_get && path == b"/files" {
        handle_manager_list(&mut socket, sd, target).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_get && path == b"/download" {
        handle_manager_download(&mut socket, sd, target).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/mkdir" {
        handle_manager_mkdir(&mut socket, sd, headers, initial_body).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/rename" {
        handle_manager_rename(&mut socket, sd, headers, initial_body).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/delete" {
        handle_manager_delete(&mut socket, sd, headers, initial_body).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_get && path == b"/v2/stat" {
        handle_v2_stat(&mut socket, sd, target).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/v2/mkdir" {
        handle_v2_mkdir(&mut socket, sd, target).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/v2/chunk" {
        handle_v2_chunk(&mut socket, sd, headers, initial_body, target).await;
        close_socket(&mut socket).await;
        return ServerEvent::Nothing;
    }

    if is_post && path == b"/upload" {
        let boundary = match find_boundary(headers) {
            Some(b) => b,
            None => {
                send_error_response(&mut socket, "Missing multipart boundary").await;
                close_socket(&mut socket).await;
                return ServerEvent::UploadFailed;
            }
        };

        let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
        let path_len = match decode_query_path(target, &mut path_buf) {
            Ok(n) => n,
            Err(e) => {
                send_error_response(&mut socket, e).await;
                close_socket(&mut socket).await;
                return ServerEvent::UploadFailed;
            }
        };
        let target_dir = if path_len == 0 {
            None
        } else {
            core::str::from_utf8(&path_buf[..path_len]).ok()
        };

        match handle_upload(&mut socket, sd, target_dir, boundary, initial_body).await {
            Ok((name_buf, name_len)) => {
                let _ = socket.write_all(HTTP_200_TEXT).await;
                let _ = socket.write_all(b"OK").await;
                let _ = socket.flush().await;
                close_socket(&mut socket).await;
                return ServerEvent::Uploaded {
                    name: name_buf,
                    name_len,
                };
            }
            Err(e) => {
                info!("upload: handle_upload error: {}", e);
                send_error_response(&mut socket, e).await;
                close_socket(&mut socket).await;
                return ServerEvent::UploadFailed;
            }
        }
    }

    let _ = socket.write_all(HTTP_404).await;
    let _ = socket.flush().await;
    close_socket(&mut socket).await;
    ServerEvent::Nothing
}

async fn handle_v2_stat(socket: &mut TcpSocket<'_>, sd: &SdStorage, target: &[u8]) {
    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match decode_query_full_path(target, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    match v2_file_size(sd, &path_buf[..path_len]) {
        Ok(size) => {
            let mut out = [0u8; 96];
            let mut pos = 0usize;
            let prefix = b"{\"exists\":true,\"size\":";
            out[..prefix.len()].copy_from_slice(prefix);
            pos += prefix.len();
            pos += fmt_u32(size, &mut out[pos..]);
            out[pos] = b'}';
            pos += 1;
            let _ = socket.write_all(HTTP_200_JSON).await;
            let _ = socket.write_all(&out[..pos]).await;
            let _ = socket.flush().await;
        }
        Err(_) => {
            let _ = socket.write_all(HTTP_200_JSON).await;
            let _ = socket.write_all(b"{\"exists\":false,\"size\":0}").await;
            let _ = socket.flush().await;
        }
    }
}

async fn handle_v2_mkdir(socket: &mut TcpSocket<'_>, sd: &SdStorage, target: &[u8]) {
    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match decode_query_path(target, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };

    match dir {
        None => send_ok(socket).await,
        Some(path) => match v2_ensure_path(sd, path) {
            Ok(()) => send_ok(socket).await,
            Err(_) => send_error_response(socket, "mkdir failed").await,
        },
    }
}

async fn handle_v2_chunk(
    socket: &mut TcpSocket<'_>,
    sd: &SdStorage,
    headers: &[u8],
    initial_body: &[u8],
    target: &[u8],
) {
    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match decode_query_full_path(target, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let offset = match decode_query_u32(target, b"o") {
        Ok(v) => v,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let wanted = extract_content_length(headers).unwrap_or(0);
    if wanted == 0 || wanted > WIFI_V2_MAX_CHUNK {
        send_error_response(socket, "bad chunk size").await;
        return;
    }

    let mut body = [0u8; WIFI_V2_MAX_CHUNK];
    let have = initial_body.len().min(wanted).min(body.len());
    body[..have].copy_from_slice(&initial_body[..have]);
    let mut body_len = have;
    while body_len < wanted {
        match socket.read(&mut body[body_len..wanted]).await {
            Ok(0) | Err(_) => break,
            Ok(n) => body_len += n,
        }
    }

    if body_len != wanted {
        send_error_response(socket, "short chunk").await;
        return;
    }

    match v2_write_chunk(sd, &path_buf[..path_len], offset, &body[..body_len]) {
        Ok(size) => {
            let mut out = [0u8; 96];
            let mut pos = 0usize;
            let prefix = b"{\"ok\":true,\"size\":";
            out[..prefix.len()].copy_from_slice(prefix);
            pos += prefix.len();
            pos += fmt_u32(size, &mut out[pos..]);
            out[pos] = b'}';
            pos += 1;
            let _ = socket.write_all(HTTP_200_JSON).await;
            let _ = socket.write_all(&out[..pos]).await;
            let _ = socket.flush().await;
        }
        Err(e) => send_error_response(socket, e).await,
    }
}

fn decode_query_full_path(target: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let raw = query_value(target, b"p").ok_or("missing path")?;

    let mut decoded = [0u8; MANAGER_PATH_BUF_LEN];
    let len = url_decode_to_buf(raw, &mut decoded).ok_or("bad path encoding")?;

    normalize_v2_full_path(&decoded[..len], out)
}

fn normalize_v2_full_path(raw: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let mut raw = trim_ascii(raw);

    while raw.first() == Some(&b'/') {
        raw = &raw[1..];
    }

    while raw.last() == Some(&b'/') {
        raw = &raw[..raw.len() - 1];
    }

    if raw.is_empty() || raw.len() >= out.len() {
        return Err("bad path length");
    }

    let mut pos = 0usize;
    let mut component_start = 0usize;
    let mut component_count = 0usize;

    for i in 0..=raw.len() {
        if i == raw.len() || raw[i] == b'/' {
            let component = &raw[component_start..i];

            if component.is_empty() || component == b"." || component == b".." {
                return Err("invalid path component");
            }

            if component.iter().any(|&b| b == b'\\' || b == b':') {
                return Err("invalid path separator");
            }

            if component_count >= MANAGER_MAX_PATH_COMPONENTS {
                return Err("path depth exceeds Wi-Fi transfer limit");
            }

            let mut checked = [0u8; 13];
            let len = normalize_manager_name(component, &mut checked)?;

            if component_count > 0 {
                out[pos] = b'/';
                pos += 1;
            }

            out[pos..pos + len].copy_from_slice(&checked[..len]);
            pos += len;
            component_count += 1;
            component_start = i + 1;
        }
    }

    Ok(pos)
}

fn decode_query_u32(target: &[u8], key: &[u8]) -> Result<u32, &'static str> {
    let raw = query_value(target, key).ok_or("missing number")?;
    let mut value = 0u32;
    for &b in raw {
        if !b.is_ascii_digit() {
            return Err("bad number");
        }
        value = value.saturating_mul(10).saturating_add((b - b'0') as u32);
    }
    Ok(value)
}

fn v2_file_parts(path: &[u8]) -> Result<(Option<&str>, &str), &'static str> {
    let path = core::str::from_utf8(path).map_err(|_| "invalid path")?;
    let Some(pos) = path.rfind('/') else {
        if path.is_empty() {
            return Err("missing filename");
        }
        return Ok((None, path));
    };

    let dir = &path[..pos];
    let name = &path[pos + 1..];
    if name.is_empty() {
        return Err("missing filename");
    }
    Ok((Some(dir), name))
}

fn v2_file_size(sd: &SdStorage, path: &[u8]) -> crate::rustmix_x4::x4_kernel::error::Result<u32> {
    let (dir, name) = v2_file_parts(path).map_err(|_| {
        crate::rustmix_x4::x4_kernel::error::Error::new(
            crate::rustmix_x4::x4_kernel::error::ErrorKind::InvalidData,
            "v2_file_size_path",
        )
    })?;
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::file_size(sd, name),
        ManagerPathParts::Dir(d) => storage::file_size_in_dir(sd, d, name),
        ManagerPathParts::Subdir(d, s) => storage::file_size_in_subdir(sd, d, s, name),
        ManagerPathParts::Nested(path) => storage::file_size_in_path(sd, path, name),
    }
}

fn v2_write_chunk(
    sd: &SdStorage,
    path: &[u8],
    offset: u32,
    data: &[u8],
) -> Result<u32, &'static str> {
    let (dir, name) = v2_file_parts(path)?;
    let current = v2_file_size(sd, path).unwrap_or(0);
    let next = offset.saturating_add(data.len() as u32);

    if current >= next {
        return Ok(current);
    }
    if current != offset {
        return Err("resume offset mismatch");
    }

    if offset == 0 {
        manager_write_file(sd, dir, name, data).map_err(|_| "write failed")?;
    } else {
        manager_append_file(sd, dir, name, data).map_err(|_| "append failed")?;
    }
    Ok(next)
}

fn v2_ensure_path(sd: &SdStorage, path: &str) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    match split_manager_path(Some(path)) {
        ManagerPathParts::Root => Ok(()),
        ManagerPathParts::Dir(d) => storage::ensure_dir(sd, d),
        ManagerPathParts::Subdir(d, s) => {
            let _ = storage::ensure_dir(sd, d);
            storage::ensure_dir_in_dir(sd, d, s)
        }
        ManagerPathParts::Nested(path) => storage::ensure_path(sd, path),
    }
}

async fn handle_upload(
    socket: &mut TcpSocket<'_>,
    sd: &SdStorage,
    target_dir: Option<&str>,
    boundary: &[u8],
    initial_body: &[u8],
) -> Result<([u8; 13], u8), &'static str>
where
{
    if boundary.len() > MAX_BOUNDARY_LEN {
        return Err("boundary too long");
    }

    let em_len = 4 + boundary.len();
    let mut end_marker_buf = [0u8; MAX_BOUNDARY_LEN + 4];
    end_marker_buf[0] = b'\r';
    end_marker_buf[1] = b'\n';
    end_marker_buf[2] = b'-';
    end_marker_buf[3] = b'-';
    end_marker_buf[4..em_len].copy_from_slice(boundary);
    let end_marker = &end_marker_buf[..em_len];

    let mut work = [0u8; WORK_BUF_SIZE];
    let init_len = initial_body.len().min(work.len());
    work[..init_len].copy_from_slice(&initial_body[..init_len]);
    let mut filled = init_len;

    let (file_name_buf, file_name_len) = loop {
        if let Some(pos) = find_subsequence(&work[..filled], b"\r\n\r\n") {
            let part_headers = &work[..pos];

            let raw_name = extract_filename(part_headers).ok_or("no filename in upload")?;
            let (name_buf, name_len) = sanitize_83(raw_name);
            if name_len == 0 {
                return Err("invalid filename");
            }

            if raw_name != &name_buf[..name_len as usize] {
                log::warn!(
                    "upload: sanitised '{}' -> '{}' (may overwrite existing file)",
                    core::str::from_utf8(raw_name).unwrap_or("?"),
                    core::str::from_utf8(&name_buf[..name_len as usize]).unwrap_or("?"),
                );
            }

            let file_start = pos + 4;
            work.copy_within(file_start..filled, 0);
            filled -= file_start;

            break (name_buf, name_len);
        }

        if filled >= work.len() {
            return Err("part headers too large");
        }

        let n = socket
            .read(&mut work[filled..])
            .await
            .map_err(|_| "read error")?;
        if n == 0 {
            return Err("connection closed during headers");
        }
        filled += n;
    };

    let name_str = core::str::from_utf8(&file_name_buf[..file_name_len as usize])
        .map_err(|_| "filename encoding error")?;

    info!("upload: receiving file '{}'", name_str);

    manager_write_file(sd, target_dir, name_str, &[]).map_err(|_| "write failed")?;

    let mut total_written: u32 = 0;

    loop {
        if let Some(pos) = find_subsequence(&work[..filled], end_marker) {
            if pos > 0 {
                manager_append_file(sd, target_dir, name_str, &work[..pos])
                    .map_err(|_| "write failed")?;
                total_written += pos as u32;
            }
            info!("upload: complete, {} bytes written", total_written);
            return Ok((file_name_buf, file_name_len));
        }

        if filled > end_marker.len() {
            let safe = filled - end_marker.len();
            manager_append_file(sd, target_dir, name_str, &work[..safe])
                .map_err(|_| "write failed")?;
            total_written += safe as u32;

            work.copy_within(safe..filled, 0);
            filled = end_marker.len();
        }

        let n = socket
            .read(&mut work[filled..])
            .await
            .map_err(|_| "read error during upload")?;
        if n == 0 {
            if filled > 0 {
                let _ = manager_append_file(sd, target_dir, name_str, &work[..filled]);
            }
            return Err("upload incomplete");
        }
        filled += n;
    }
}

async fn handle_manager_list(socket: &mut TcpSocket<'_>, sd: &SdStorage, target: &[u8]) {
    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match decode_query_path(target, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };

    let mut entries = [storage::DirEntry::EMPTY; DIR_LIST_MAX];
    let count = manager_list_entries(sd, dir, &mut entries);

    let count = match count {
        Ok(n) => n,
        Err(_) => {
            send_error_response(socket, "directory list failed").await;
            return;
        }
    };

    let _ = socket.write_all(HTTP_200_JSON).await;
    let _ = socket.write_all(b"[").await;

    let mut json_buf = [0u8; 112];
    for (i, e) in entries.iter().enumerate().take(count) {
        let name = e.name_str();
        let mut pos = 0usize;

        let prefix = b"{\"name\":\"";
        json_buf[pos..pos + prefix.len()].copy_from_slice(prefix);
        pos += prefix.len();

        let nb = name.as_bytes();
        json_buf[pos..pos + nb.len()].copy_from_slice(nb);
        pos += nb.len();

        let mid = b"\",\"size\":";
        json_buf[pos..pos + mid.len()].copy_from_slice(mid);
        pos += mid.len();

        pos += fmt_u32(e.size, &mut json_buf[pos..]);

        let dir_part = if e.is_dir {
            b",\"dir\":true}" as &[u8]
        } else {
            b",\"dir\":false}" as &[u8]
        };
        json_buf[pos..pos + dir_part.len()].copy_from_slice(dir_part);
        pos += dir_part.len();

        if i + 1 < count {
            json_buf[pos] = b',';
            pos += 1;
        }

        let _ = socket.write_all(&json_buf[..pos]).await;
    }

    let _ = socket.write_all(b"]").await;
    let _ = socket.flush().await;
}

async fn handle_manager_download(socket: &mut TcpSocket<'_>, sd: &SdStorage, target: &[u8]) {
    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match decode_query_path(target, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let mut name_buf = [0u8; 13];
    let name_len = match decode_query_name(target, b"n", &mut name_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };
    let Some(name) = core::str::from_utf8(&name_buf[..name_len]).ok() else {
        send_error_response(socket, "invalid filename").await;
        return;
    };

    let _ = socket
        .write_all(
            b"HTTP/1.0 200 OK\r\nContent-Type: application/octet-stream\r\nContent-Disposition: attachment; filename=\"",
        )
        .await;
    let _ = socket.write_all(name.as_bytes()).await;
    let _ = socket.write_all(b"\"\r\nConnection: close\r\n\r\n").await;

    let mut chunk = [0u8; 512];
    let mut offset = 0u32;

    loop {
        let n = match manager_read_file(sd, dir, name, offset, &mut chunk) {
            Ok(n) => n,
            Err(_) => break,
        };

        if n == 0 {
            break;
        }

        if socket.write_all(&chunk[..n]).await.is_err() {
            break;
        }

        offset = offset.saturating_add(n as u32);

        if n < chunk.len() {
            break;
        }
    }

    let _ = socket.flush().await;
}

async fn handle_manager_mkdir(
    socket: &mut TcpSocket<'_>,
    sd: &SdStorage,
    headers: &[u8],
    initial_body: &[u8],
) {
    let mut body = [0u8; 96];
    let body_len = read_small_body(socket, headers, initial_body, &mut body).await;

    let path_line = body_line(&body[..body_len], 0);
    let name_line = body_line(&body[..body_len], 1);

    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match normalize_manager_path(path_line, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };

    let (name_buf, name_len) = match sanitize_new_manager_name(name_line, true) {
        Ok(v) => v,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let Some(name) = core::str::from_utf8(&name_buf[..name_len as usize]).ok() else {
        send_error_response(socket, "invalid directory name").await;
        return;
    };

    match manager_ensure_dir(sd, dir, name) {
        Ok(()) => send_ok(socket).await,
        Err(_) => send_error_response(socket, "create directory failed").await,
    }
}

async fn handle_manager_rename(
    socket: &mut TcpSocket<'_>,
    sd: &SdStorage,
    headers: &[u8],
    initial_body: &[u8],
) {
    let mut body = [0u8; 128];
    let body_len = read_small_body(socket, headers, initial_body, &mut body).await;

    let path_line = body_line(&body[..body_len], 0);
    let old_line = body_line(&body[..body_len], 1);
    let new_line = body_line(&body[..body_len], 2);

    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match normalize_manager_path(path_line, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let mut old_buf = [0u8; 13];
    let old_len = match normalize_manager_name(old_line, &mut old_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let (new_buf, new_len) = match sanitize_new_manager_name(new_line, false) {
        Ok(v) => v,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };
    let Some(old_name) = core::str::from_utf8(&old_buf[..old_len]).ok() else {
        send_error_response(socket, "invalid old filename").await;
        return;
    };
    let Some(new_name) = core::str::from_utf8(&new_buf[..new_len as usize]).ok() else {
        send_error_response(socket, "invalid new filename").await;
        return;
    };

    if old_name.eq_ignore_ascii_case(new_name) {
        send_ok(socket).await;
        return;
    }

    match copy_file_for_rename(sd, dir, old_name, new_name) {
        Ok(()) => send_ok(socket).await,
        Err(_) => send_error_response(socket, "rename failed").await,
    }
}

async fn handle_manager_delete(
    socket: &mut TcpSocket<'_>,
    sd: &SdStorage,
    headers: &[u8],
    initial_body: &[u8],
) {
    let mut body = [0u8; 128];
    let body_len = read_small_body(socket, headers, initial_body, &mut body).await;

    let path_line = body_line(&body[..body_len], 0);
    let name_line = body_line(&body[..body_len], 1);
    let kind_line = body_line(&body[..body_len], 2);

    let mut path_buf = [0u8; MANAGER_PATH_BUF_LEN];
    let path_len = match normalize_manager_path(path_line, &mut path_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let mut name_buf = [0u8; 13];
    let name_len = match normalize_manager_name(name_line, &mut name_buf) {
        Ok(n) => n,
        Err(e) => {
            send_error_response(socket, e).await;
            return;
        }
    };

    let dir = if path_len == 0 {
        None
    } else {
        core::str::from_utf8(&path_buf[..path_len]).ok()
    };

    let Some(name) = core::str::from_utf8(&name_buf[..name_len]).ok() else {
        send_error_response(socket, "invalid filename").await;
        return;
    };

    let is_dir = kind_line.eq_ignore_ascii_case(b"dir");

    let result = if is_dir {
        manager_delete_dir(sd, dir, name)
    } else {
        manager_delete_file(sd, dir, name)
    };

    match result {
        Ok(()) => send_ok(socket).await,
        Err(_) => {
            if is_dir {
                send_error_response(socket, "directory delete failed").await;
            } else {
                send_error_response(socket, "delete failed").await;
            }
        }
    }
}

async fn read_small_body(
    socket: &mut TcpSocket<'_>,
    headers: &[u8],
    initial_body: &[u8],
    out: &mut [u8],
) -> usize {
    let wanted = extract_content_length(headers).unwrap_or(0).min(out.len());
    let have = initial_body.len().min(wanted);
    out[..have].copy_from_slice(&initial_body[..have]);

    let mut body_len = have;
    while body_len < wanted {
        match socket.read(&mut out[body_len..wanted]).await {
            Ok(0) | Err(_) => break,
            Ok(n) => body_len += n,
        }
    }

    body_len
}

fn body_line(body: &[u8], index: usize) -> &[u8] {
    let mut start = 0usize;
    let mut line = 0usize;

    for (pos, &b) in body.iter().enumerate() {
        if b == b'\n' {
            if line == index {
                return trim_ascii(&body[start..pos]);
            }
            line += 1;
            start = pos + 1;
        }
    }

    if line == index {
        trim_ascii(&body[start..])
    } else {
        b""
    }
}

fn trim_ascii(mut input: &[u8]) -> &[u8] {
    while let Some((&first, rest)) = input.split_first() {
        if first == b' ' || first == b'\t' || first == b'\r' {
            input = rest;
        } else {
            break;
        }
    }

    while let Some((&last, rest)) = input.split_last() {
        if last == b' ' || last == b'\t' || last == b'\r' {
            input = rest;
        } else {
            break;
        }
    }

    input
}

fn extract_target(line: &[u8]) -> &[u8] {
    let start = match line.iter().position(|&b| b == b' ') {
        Some(p) => p + 1,
        None => return b"/",
    };

    let rest = &line[start..];
    let end = rest.iter().position(|&b| b == b' ').unwrap_or(rest.len());
    &rest[..end]
}

fn query_value<'a>(target: &'a [u8], key: &[u8]) -> Option<&'a [u8]> {
    let q = target.iter().position(|&b| b == b'?')?;
    let mut part = &target[q + 1..];

    loop {
        let amp = part.iter().position(|&b| b == b'&').unwrap_or(part.len());
        let item = &part[..amp];

        if item.len() > key.len() + 1
            && item[..key.len()].eq_ignore_ascii_case(key)
            && item[key.len()] == b'='
        {
            return Some(&item[key.len() + 1..]);
        }

        if amp == part.len() {
            break;
        }
        part = &part[amp + 1..];
    }

    None
}

fn decode_query_path(target: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let raw = query_value(target, b"p").unwrap_or(b"");

    let mut decoded = [0u8; MANAGER_PATH_BUF_LEN];
    let len = url_decode_to_buf(raw, &mut decoded).ok_or("bad path encoding")?;

    normalize_manager_path(&decoded[..len], out)
}

fn decode_query_name(target: &[u8], key: &[u8], out: &mut [u8; 13]) -> Result<usize, &'static str> {
    let raw = query_value(target, key).ok_or("missing filename")?;

    let mut decoded = [0u8; 13];
    let len = url_decode_to_buf(raw, &mut decoded).ok_or("bad filename encoding")?;

    normalize_manager_name(&decoded[..len], out)
}

fn url_decode_to_buf(input: &[u8], out: &mut [u8]) -> Option<usize> {
    let mut pos = 0usize;
    let mut i = 0usize;

    while i < input.len() {
        if pos >= out.len() {
            return None;
        }

        let b = input[i];
        if b == b'%' {
            if i + 2 >= input.len() {
                return None;
            }

            let hi = hex_val(input[i + 1])?;
            let lo = hex_val(input[i + 2])?;
            out[pos] = (hi << 4) | lo;
            pos += 1;
            i += 3;
        } else if b == b'+' {
            out[pos] = b' ';
            pos += 1;
            i += 1;
        } else {
            out[pos] = b;
            pos += 1;
            i += 1;
        }
    }

    Some(pos)
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

fn normalize_manager_path(raw: &[u8], out: &mut [u8]) -> Result<usize, &'static str> {
    let mut raw = trim_ascii(raw);

    if raw.is_empty() || raw == b"/" {
        return Ok(0);
    }

    while raw.first() == Some(&b'/') {
        raw = &raw[1..];
    }

    while raw.last() == Some(&b'/') {
        raw = &raw[..raw.len() - 1];
    }

    if raw.is_empty() {
        return Ok(0);
    }

    if raw.len() >= out.len() {
        return Err("path too long");
    }

    let mut pos = 0usize;
    let mut component_start = 0usize;
    let mut component_count = 0usize;

    for i in 0..=raw.len() {
        if i == raw.len() || raw[i] == b'/' {
            let component = &raw[component_start..i];

            if component.is_empty() || component == b"." || component == b".." {
                return Err("invalid path component");
            }

            if component.iter().any(|&b| b == b'\\' || b == b':') {
                return Err("invalid path separator");
            }

            if component_count >= MANAGER_MAX_PATH_COMPONENTS {
                return Err("path depth exceeds Wi-Fi transfer limit");
            }

            let mut checked = [0u8; 13];
            let len = normalize_manager_name(component, &mut checked)?;

            if component_count > 0 {
                out[pos] = b'/';
                pos += 1;
            }

            out[pos..pos + len].copy_from_slice(&checked[..len]);
            pos += len;
            component_count += 1;
            component_start = i + 1;
        }
    }

    Ok(pos)
}

fn normalize_manager_name(raw: &[u8], out: &mut [u8; 13]) -> Result<usize, &'static str> {
    let raw = trim_ascii(raw);

    if raw.is_empty() || raw == b"." || raw == b".." {
        return Err("invalid name");
    }

    if raw.len() > 12 {
        return Err("name must fit 8.3 format");
    }

    let mut dot_seen = false;
    for (idx, &b) in raw.iter().enumerate() {
        if b == b'.' {
            if dot_seen || idx == 0 || idx + 1 == raw.len() {
                return Err("invalid dot in name");
            }
            dot_seen = true;
        } else if !is_valid_83_char(b) {
            return Err("invalid name character");
        }
    }

    out[..raw.len()].copy_from_slice(raw);
    Ok(raw.len())
}

fn sanitize_new_manager_name(raw: &[u8], directory: bool) -> Result<([u8; 13], u8), &'static str> {
    let raw = trim_ascii(raw);

    if raw.is_empty() || raw == b"." || raw == b".." {
        return Err("invalid name");
    }

    if raw.iter().any(|&b| b == b'/' || b == b'\\' || b == b':') {
        return Err("path separators are not allowed");
    }

    if directory && raw.iter().any(|&b| b == b'.') {
        return Err("directory names must not contain dot");
    }

    let (name, len) = sanitize_83(raw);
    if len == 0 {
        return Err("invalid name");
    }

    Ok((name, len))
}

enum ManagerPathParts<'a> {
    Root,
    Dir(&'a str),
    Subdir(&'a str, &'a str),
    Nested(&'a str),
}

fn split_manager_path(path: Option<&str>) -> ManagerPathParts<'_> {
    let Some(path) = path else {
        return ManagerPathParts::Root;
    };

    let path = path.trim_matches('/');
    if path.is_empty() {
        return ManagerPathParts::Root;
    }

    let mut first_slash = None;
    let mut second_slash = None;
    for (idx, &b) in path.as_bytes().iter().enumerate() {
        if b == b'/' {
            if first_slash.is_none() {
                first_slash = Some(idx);
            } else {
                second_slash = Some(idx);
                break;
            }
        }
    }

    match (first_slash, second_slash) {
        (None, _) => ManagerPathParts::Dir(path),
        (Some(pos), None) => {
            let first = &path[..pos];
            let second = &path[pos + 1..];
            if first.is_empty() || second.is_empty() {
                ManagerPathParts::Root
            } else {
                ManagerPathParts::Subdir(first, second)
            }
        }
        (Some(_), Some(_)) => ManagerPathParts::Nested(path),
    }
}

fn manager_list_entries(
    sd: &SdStorage,
    dir: Option<&str>,
    entries: &mut [storage::DirEntry],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::list_root_entries(sd, entries),
        ManagerPathParts::Dir(d) => storage::list_dir_entries(sd, d, entries),
        ManagerPathParts::Subdir(d, s) => storage::list_subdir_entries(sd, d, s, entries),
        ManagerPathParts::Nested(path) => storage::list_path_entries(sd, path, entries),
    }
}

fn manager_ensure_dir(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::ensure_dir(sd, name),
        ManagerPathParts::Dir(d) => storage::ensure_dir_in_dir(sd, d, name),
        ManagerPathParts::Subdir(d, s) => storage::ensure_dir_in_subdir(sd, d, s, name),
        ManagerPathParts::Nested(_) => {
            let mut child_path = [0u8; MANAGER_PATH_BUF_LEN];
            let child_len = compose_child_manager_path(dir, name, &mut child_path).ok_or(
                crate::rustmix_x4::x4_kernel::error::Error::new(
                    crate::rustmix_x4::x4_kernel::error::ErrorKind::WriteFailed,
                    "mkdir_nested_path",
                ),
            )?;
            let child_dir = core::str::from_utf8(&child_path[..child_len]).map_err(|_| {
                crate::rustmix_x4::x4_kernel::error::Error::new(
                    crate::rustmix_x4::x4_kernel::error::ErrorKind::WriteFailed,
                    "mkdir_nested_utf8",
                )
            })?;
            storage::ensure_path(sd, child_dir)
        }
    }
}

fn manager_read_file(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
    offset: u32,
    buf: &mut [u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<usize> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::read_file_chunk(sd, name, offset, buf),
        ManagerPathParts::Dir(d) => storage::read_file_chunk_in_dir(sd, d, name, offset, buf),
        ManagerPathParts::Subdir(d, s) => {
            storage::read_file_chunk_in_subdir(sd, d, s, name, offset, buf)
        }
        ManagerPathParts::Nested(path) => {
            storage::read_file_chunk_in_path(sd, path, name, offset, buf)
        }
    }
}

fn manager_write_file(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::write_file(sd, name, data),
        ManagerPathParts::Dir(d) => storage::write_file_in_dir(sd, d, name, data),
        ManagerPathParts::Subdir(d, s) => storage::write_file_in_subdir(sd, d, s, name, data),
        ManagerPathParts::Nested(path) => storage::write_file_in_path(sd, path, name, data),
    }
}

fn manager_append_file(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
    data: &[u8],
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::append_root_file(sd, name, data),
        ManagerPathParts::Dir(d) => storage::append_file_in_dir(sd, d, name, data),
        ManagerPathParts::Subdir(d, s) => storage::append_file_in_subdir(sd, d, s, name, data),
        ManagerPathParts::Nested(path) => storage::append_file_in_path(sd, path, name, data),
    }
}

fn manager_delete_dir(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let mut child_path = [0u8; MANAGER_PATH_BUF_LEN];
    let child_len = compose_child_manager_path(dir, name, &mut child_path).ok_or(
        crate::rustmix_x4::x4_kernel::error::Error::new(
            crate::rustmix_x4::x4_kernel::error::ErrorKind::DeleteFailed,
            "delete_dir_path",
        ),
    )?;

    let child_dir = core::str::from_utf8(&child_path[..child_len]).map_err(|_| {
        crate::rustmix_x4::x4_kernel::error::Error::new(
            crate::rustmix_x4::x4_kernel::error::ErrorKind::DeleteFailed,
            "delete_dir_utf8",
        )
    })?;

    let mut entries = [storage::DirEntry::EMPTY; DIR_LIST_MAX];
    let count = manager_list_entries(sd, Some(child_dir), &mut entries)?;

    for entry in entries.iter().take(count) {
        let entry_name = entry.name_str();

        if entry.is_dir {
            manager_delete_dir(sd, Some(child_dir), entry_name)?;
        } else {
            manager_delete_file(sd, Some(child_dir), entry_name)?;
        }
    }

    manager_delete_file(sd, dir, name)
}

fn compose_child_manager_path(dir: Option<&str>, name: &str, out: &mut [u8]) -> Option<usize> {
    let mut pos = 0usize;

    if let Some(dir) = dir {
        if !dir.is_empty() {
            let db = dir.as_bytes();
            if db.len() >= out.len() {
                return None;
            }

            out[..db.len()].copy_from_slice(db);
            pos += db.len();

            if pos >= out.len() {
                return None;
            }

            out[pos] = b'/';
            pos += 1;
        }
    }

    let nb = name.as_bytes();
    if pos + nb.len() > out.len() {
        return None;
    }

    out[pos..pos + nb.len()].copy_from_slice(nb);
    Some(pos + nb.len())
}

fn manager_delete_file(
    sd: &SdStorage,
    dir: Option<&str>,
    name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    match split_manager_path(dir) {
        ManagerPathParts::Root => storage::delete_file(sd, name),
        ManagerPathParts::Dir(d) => storage::delete_file_in_dir(sd, d, name),
        ManagerPathParts::Subdir(d, s) => storage::delete_file_in_subdir(sd, d, s, name),
        ManagerPathParts::Nested(path) => storage::delete_file_in_path(sd, path, name),
    }
}

fn copy_file_for_rename(
    sd: &SdStorage,
    dir: Option<&str>,
    old_name: &str,
    new_name: &str,
) -> crate::rustmix_x4::x4_kernel::error::Result<()> {
    let mut buf = [0u8; 512];
    let mut offset = 0u32;
    let mut first = true;

    loop {
        let n = manager_read_file(sd, dir, old_name, offset, &mut buf)?;

        if first {
            manager_write_file(sd, dir, new_name, &[])?;
            first = false;
        }

        if n == 0 {
            break;
        }

        manager_append_file(sd, dir, new_name, &buf[..n])?;
        offset = offset.saturating_add(n as u32);

        if n < buf.len() {
            break;
        }
    }

    manager_delete_file(sd, dir, old_name)
}

async fn send_ok(socket: &mut TcpSocket<'_>) {
    let _ = socket.write_all(HTTP_200_TEXT).await;
    let _ = socket.write_all(b"OK").await;
    let _ = socket.flush().await;
}

fn extract_path(line: &[u8]) -> &[u8] {
    let start = match line.iter().position(|&b| b == b' ') {
        Some(p) => p + 1,
        None => return b"/",
    };

    let rest = &line[start..];
    let end = rest.iter().position(|&b| b == b' ').unwrap_or(rest.len());

    let path = &rest[..end];
    let qmark = path.iter().position(|&b| b == b'?').unwrap_or(path.len());
    &path[..qmark]
}

fn find_boundary(headers: &[u8]) -> Option<&[u8]> {
    let marker = b"boundary=";
    let pos = headers
        .windows(marker.len())
        .position(|w| w.eq_ignore_ascii_case(marker))?;
    let start = pos + marker.len();
    let rest = &headers[start..];

    if rest.is_empty() {
        return None;
    }

    if rest[0] == b'"' {
        let inner = &rest[1..];
        let end = inner.iter().position(|&b| b == b'"')?;
        if end == 0 {
            return None;
        }
        Some(&inner[..end])
    } else {
        let end = rest
            .iter()
            .position(|&b| b == b'\r' || b == b'\n' || b == b';' || b == b' ')
            .unwrap_or(rest.len());
        if end == 0 {
            return None;
        }
        Some(&rest[..end])
    }
}

fn extract_filename(headers: &[u8]) -> Option<&[u8]> {
    let marker = b"filename=\"";
    let pos = headers
        .windows(marker.len())
        .position(|w| w.eq_ignore_ascii_case(marker))?;
    let start = pos + marker.len();
    let rest = &headers[start..];
    let end = rest.iter().position(|&b| b == b'"')?;
    if end == 0 {
        return None;
    }
    Some(&rest[..end])
}

fn sanitize_83(raw: &[u8]) -> ([u8; 13], u8) {
    let name = match raw.iter().rposition(|&b| b == b'/' || b == b'\\') {
        Some(p) => &raw[p + 1..],
        None => raw,
    };

    let (base_src, ext_src) = match name.iter().rposition(|&b| b == b'.') {
        Some(dot) => (&name[..dot], &name[dot + 1..]),
        None => (name, &[] as &[u8]),
    };

    let mut out = [0u8; 13];
    let mut pos: usize = 0;

    for &b in base_src.iter() {
        if pos >= 8 {
            break;
        }
        if is_valid_83_char(b) {
            out[pos] = b.to_ascii_uppercase();
            pos += 1;
        }
    }

    if pos == 0 {
        out[..6].copy_from_slice(b"UPLOAD");
        pos = 6;
    }

    if !ext_src.is_empty() {
        out[pos] = b'.';
        pos += 1;
        let ext_start = pos;
        for &b in ext_src.iter() {
            if pos - ext_start >= 3 {
                break;
            }
            if is_valid_83_char(b) {
                out[pos] = b.to_ascii_uppercase();
                pos += 1;
            }
        }

        if pos == ext_start {
            pos -= 1;
        }
    }

    (out, pos as u8)
}

fn is_valid_83_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || matches!(b, b'_' | b'-' | b'~' | b'!' | b'#' | b'$' | b'&')
}

async fn send_error_response(socket: &mut TcpSocket<'_>, msg: &str) {
    let _ = socket.write_all(HTTP_500_TEXT).await;
    let _ = socket.write_all(msg.as_bytes()).await;
    let _ = socket.flush().await;
}

fn fmt_u32(mut n: u32, buf: &mut [u8]) -> usize {
    if n == 0 {
        buf[0] = b'0';
        return 1;
    }
    let mut tmp = [0u8; 10];
    let mut pos = 0;
    while n > 0 {
        tmp[pos] = b'0' + (n % 10) as u8;
        n /= 10;
        pos += 1;
    }
    for i in 0..pos {
        buf[i] = tmp[pos - 1 - i];
    }
    pos
}

fn extract_content_length(headers: &[u8]) -> Option<usize> {
    let marker = b"content-length:";
    let pos = headers
        .windows(marker.len())
        .position(|w| w.eq_ignore_ascii_case(marker))?;
    let start = pos + marker.len();
    let rest = &headers[start..];

    let trimmed = rest.iter().position(|&b| b != b' ' && b != b'\t')?;
    let rest = &rest[trimmed..];
    let end = rest
        .iter()
        .position(|&b| b == b'\r' || b == b'\n')
        .unwrap_or(rest.len());
    let digits = &rest[..end];
    let mut val: usize = 0;
    for &b in digits {
        if b.is_ascii_digit() {
            val = val.saturating_mul(10).saturating_add((b - b'0') as usize);
        } else {
            break;
        }
    }
    Some(val)
}

async fn close_socket(socket: &mut TcpSocket<'_>) {
    Timer::after(Duration::from_millis(SOCKET_CLOSE_DELAY_MS)).await;
    socket.close();
    Timer::after(Duration::from_millis(SOCKET_CLOSE_DELAY_MS)).await;
    socket.abort();
}

fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    if needle.is_empty() || needle.len() > haystack.len() {
        return None;
    }
    haystack
        .windows(needle.len())
        .position(|window| window == needle)
}

async fn mdns_respond_once(stack: embassy_net::Stack<'_>, ip_octets: [u8; 4]) {
    let mut rx_meta = [PacketMetadata::EMPTY; 2];
    let mut rx_buf = [0u8; 512];
    let mut tx_meta = [PacketMetadata::EMPTY; 2];
    let mut tx_buf = [0u8; 512];

    let mut socket = UdpSocket::new(stack, &mut rx_meta, &mut rx_buf, &mut tx_meta, &mut tx_buf);

    if socket.bind(MDNS_PORT).is_err() {
        Timer::after(Duration::from_millis(MDNS_BIND_RETRY_MS)).await;
        return;
    }

    let mut pkt = [0u8; 256];
    let (n, _remote) = match socket.recv_from(&mut pkt).await {
        Ok(r) => r,
        Err(_) => return,
    };

    if !is_mdns_query_for_x4(&pkt[..n]) {
        return;
    }

    info!("upload: mDNS query for x4.local -- responding");

    let mut resp = [0u8; MDNS_RESPONSE_LEN];
    let len = build_mdns_response(&mut resp, ip_octets);

    let mdns_dest = embassy_net::IpEndpoint::new(
        embassy_net::IpAddress::Ipv4(embassy_net::Ipv4Address::new(224, 0, 0, 251)),
        MDNS_PORT,
    );
    let _ = socket.send_to(&resp[..len], mdns_dest).await;
}

fn is_mdns_query_for_x4(pkt: &[u8]) -> bool {
    let qname_start = 12;
    let qname_end = qname_start + HOSTNAME_WIRE.len();
    let qtype_start = qname_end;
    let qclass_start = qtype_start + 2;
    let min_len = qclass_start + 2;

    if pkt.len() < min_len {
        return false;
    }

    let flags = u16::from_be_bytes([pkt[2], pkt[3]]);
    if flags & 0x8000 != 0 {
        return false;
    }

    let qdcount = u16::from_be_bytes([pkt[4], pkt[5]]);
    if qdcount < 1 {
        return false;
    }

    let qname = &pkt[qname_start..qname_end];
    if qname[0] != 2 || qname[3] != 5 || qname[9] != 0 {
        return false;
    }
    if !qname[1..3].eq_ignore_ascii_case(b"x4") {
        return false;
    }
    if !qname[4..9].eq_ignore_ascii_case(b"local") {
        return false;
    }

    let qtype = u16::from_be_bytes([pkt[qtype_start], pkt[qtype_start + 1]]);
    let qclass = u16::from_be_bytes([pkt[qclass_start], pkt[qclass_start + 1]]) & 0x7FFF;

    (qtype == 1 || qtype == 255) && qclass == 1
}

fn build_mdns_response(buf: &mut [u8], ip: [u8; 4]) -> usize {
    let r = &mut buf[..MDNS_RESPONSE_LEN];

    r[0..2].copy_from_slice(&[0x00, 0x00]);
    r[2..4].copy_from_slice(&[0x84, 0x00]);
    r[4..6].copy_from_slice(&[0x00, 0x00]);
    r[6..8].copy_from_slice(&[0x00, 0x01]);
    r[8..10].copy_from_slice(&[0x00, 0x00]);
    r[10..12].copy_from_slice(&[0x00, 0x00]);

    let mut pos = 12;
    r[pos..pos + HOSTNAME_WIRE.len()].copy_from_slice(&HOSTNAME_WIRE);
    pos += HOSTNAME_WIRE.len();
    r[pos..pos + 2].copy_from_slice(&[0x00, 0x01]);
    pos += 2;
    r[pos..pos + 2].copy_from_slice(&[0x80, 0x01]);
    pos += 2;
    r[pos..pos + 4].copy_from_slice(&[0x00, 0x00, 0x00, 0x78]);
    pos += 4;
    r[pos..pos + 2].copy_from_slice(&[0x00, 0x04]);
    pos += 2;
    r[pos..pos + 4].copy_from_slice(&ip);

    MDNS_RESPONSE_LEN
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

async fn render_screen(
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
        BitmapLabel::new(heading_region, "Wi-Fi Transfer", heading)
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
