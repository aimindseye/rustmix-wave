//! ESP-IDF Bluedroid BLE GATT server for Rustmix Remote.
//!
//! This module is compiled only with `--features rustmix-remote-ble` on ESP-IDF.
//! The service exposes a single write characteristic for RRBP command packets.

use std::sync::{Arc, Mutex};

use anyhow::Result;
use enumset::enum_set;
use esp_idf_svc::{
    bt::{
        ble::{
            gap::{AdvConfiguration, BleGapEvent, EspBleGap},
            gatt::{
                server::{ConnectionId, EspGatts, GattsEvent, TransferId},
                AutoResponse, GattCharacteristic, GattId, GattInterface, GattResponse,
                GattServiceId, GattStatus, Handle, Permission, Property,
            },
        },
        BdAddr, Ble, BtDriver, BtStatus, BtUuid,
    },
    sys::{EspError, ESP_FAIL},
};
use log::{info, warn};

use super::{RemoteBridgeWriteOutcome, RemoteEventQueue, RustmixRemoteBridge, RRBP_PACKET_LEN};

const APP_ID: u16 = 0x5252; // "RR" / Rustmix Remote

type RustmixGap = EspBleGap<'static, Ble, Arc<BtDriver<'static, Ble>>>;
type RustmixGatts = EspGatts<'static, Ble, Arc<BtDriver<'static, Ble>>>;

/// Stable Rustmix Remote service UUID.
pub const RUSTMIX_REMOTE_SERVICE_UUID: u128 = 0x8f7a_0000_6b8f_4a91_9e2c_7275_7374_0001;
/// RRBP Command characteristic UUID. Watch writes 6-byte RRBP packets here.
pub const RUSTMIX_REMOTE_COMMAND_UUID: u128 = 0x8f7a_0001_6b8f_4a91_9e2c_7275_7374_0001;

#[derive(Default)]
struct ServerState {
    gatt_if: Option<GattInterface>,
    service_handle: Option<Handle>,
    command_handle: Option<Handle>,
    bridge: Option<RustmixRemoteBridge>,
    response: GattResponse,
}

/// Own this value for the life of the firmware. Dropping it would unregister
/// callbacks and stop BLE remote handling.
#[derive(Clone)]
pub struct RustmixRemoteBleGattService {
    gap: Arc<RustmixGap>,
    gatts: Arc<RustmixGatts>,
    state: Arc<Mutex<ServerState>>,
}

impl RustmixRemoteBleGattService {
    pub fn start(bt: Arc<BtDriver<'static, Ble>>, queue: RemoteEventQueue) -> Result<Self> {
        let service = Self {
            gap: Arc::new(EspBleGap::new(bt.clone())?),
            gatts: Arc::new(EspGatts::new(bt)?),
            state: Arc::new(Mutex::new(ServerState {
                bridge: Some(RustmixRemoteBridge::new(queue)),
                ..ServerState::default()
            })),
        };

        let gap_service = service.clone();
        service.gap.subscribe(move |event| {
            gap_service.check_esp_status(gap_service.on_gap_event(event));
        })?;

        let gatts_service = service.clone();
        service.gatts.subscribe(move |(gatt_if, event)| {
            gatts_service.check_esp_status(gatts_service.on_gatts_event(gatt_if, event));
        })?;

        service.gatts.register_app(APP_ID)?;
        info!("rustmix-wave=rustmix-remote-ble status=registering app-id={APP_ID}");
        Ok(service)
    }

    fn on_gap_event(&self, event: BleGapEvent) -> Result<(), EspError> {
        info!("rustmix-wave=rustmix-remote-gap event={event:?}");
        if let BleGapEvent::AdvertisingConfigured(status) = event {
            self.check_bt_status(status)?;
            self.gap.start_advertising()?;
            info!("rustmix-wave=rustmix-remote-ble status=advertising name=Rustmix Remote");
        }
        Ok(())
    }

    fn on_gatts_event(&self, gatt_if: GattInterface, event: GattsEvent) -> Result<(), EspError> {
        info!("rustmix-wave=rustmix-remote-gatts event={event:?}");
        match event {
            GattsEvent::ServiceRegistered { status, app_id } => {
                self.check_gatt_status(status)?;
                if app_id == APP_ID {
                    self.create_service(gatt_if)?;
                }
            }
            GattsEvent::ServiceCreated {
                status,
                service_handle,
                ..
            } => {
                self.check_gatt_status(status)?;
                self.configure_and_start_service(service_handle)?;
            }
            GattsEvent::CharacteristicAdded {
                status,
                attr_handle,
                service_handle,
                char_uuid,
            } => {
                self.check_gatt_status(status)?;
                self.register_characteristic(service_handle, attr_handle, char_uuid)?;
            }
            GattsEvent::PeerConnected { addr, .. } => {
                info!("rustmix-wave=rustmix-remote-ble status=connected peer={addr}");
            }
            GattsEvent::PeerDisconnected { addr, .. } => {
                if let Some(bridge) = self.state.lock().unwrap().bridge.as_mut() {
                    bridge.reset_sequence_tracking();
                }
                self.set_adv_conf()?;
                info!("rustmix-wave=rustmix-remote-ble status=disconnected peer={addr} action=advertise-restart");
            }
            GattsEvent::Write {
                conn_id,
                trans_id,
                addr,
                handle,
                offset,
                need_rsp,
                is_prep,
                value,
            } => {
                let handled = self.handle_write(
                    gatt_if, conn_id, trans_id, addr, handle, offset, need_rsp, is_prep, value,
                )?;
                if handled {
                    self.send_write_response(
                        gatt_if, conn_id, trans_id, handle, offset, need_rsp, is_prep, value,
                    )?;
                }
            }
            _ => {}
        }
        Ok(())
    }

    fn set_adv_conf(&self) -> Result<(), EspError> {
        // Name-only legacy advertising is the most reliable Wear OS discovery path.
        // The 128-bit Rustmix service UUID is still present in the GATT database and
        // is verified after connection during service discovery.
        self.gap.set_adv_conf(&AdvConfiguration {
            include_name: true,
            include_txpower: false,
            flag: 6,
            service_uuid: None,
            ..Default::default()
        })
    }

    fn create_service(&self, gatt_if: GattInterface) -> Result<(), EspError> {
        self.state.lock().unwrap().gatt_if = Some(gatt_if);
        self.gap.set_device_name("Rustmix Remote")?;
        self.gatts.create_service(
            gatt_if,
            &GattServiceId {
                id: GattId {
                    uuid: BtUuid::uuid128(RUSTMIX_REMOTE_SERVICE_UUID),
                    inst_id: 0,
                },
                is_primary: true,
            },
            4,
        )?;
        Ok(())
    }

    fn configure_and_start_service(&self, service_handle: Handle) -> Result<(), EspError> {
        self.state.lock().unwrap().service_handle = Some(service_handle);
        self.gatts.start_service(service_handle)?;
        self.gatts.add_characteristic(
            service_handle,
            &GattCharacteristic {
                uuid: BtUuid::uuid128(RUSTMIX_REMOTE_COMMAND_UUID),
                permissions: enum_set!(Permission::Write),
                properties: enum_set!(Property::Write),
                max_len: RRBP_PACKET_LEN,
                auto_rsp: AutoResponse::ByApp,
            },
            &[],
        )?;
        info!(
            "rustmix-wave=rustmix-remote-ble status=service-started command-len={RRBP_PACKET_LEN}"
        );
        Ok(())
    }

    fn register_characteristic(
        &self,
        service_handle: Handle,
        attr_handle: Handle,
        char_uuid: BtUuid,
    ) -> Result<(), EspError> {
        let should_configure_advertising = {
            let mut state = self.state.lock().unwrap();
            if state.service_handle == Some(service_handle)
                && char_uuid == BtUuid::uuid128(RUSTMIX_REMOTE_COMMAND_UUID)
            {
                state.command_handle = Some(attr_handle);
                info!(
                    "rustmix-wave=rustmix-remote-ble status=command-characteristic-ready handle={attr_handle:?}"
                );
                true
            } else {
                false
            }
        };

        if should_configure_advertising {
            self.set_adv_conf()?;
            info!(
                "rustmix-wave=rustmix-remote-ble status=advertising-config-requested payload=uuid-only"
            );
        }

        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn handle_write(
        &self,
        _gatt_if: GattInterface,
        _conn_id: ConnectionId,
        _trans_id: TransferId,
        addr: BdAddr,
        handle: Handle,
        offset: u16,
        _need_rsp: bool,
        is_prep: bool,
        value: &[u8],
    ) -> Result<bool, EspError> {
        let mut state = self.state.lock().unwrap();
        if state.command_handle != Some(handle) {
            return Ok(false);
        }
        if offset != 0 || is_prep {
            warn!("rustmix-wave=rustmix-remote-command status=ignored reason=prepared-or-offset-write peer={addr} offset={offset} is-prep={is_prep}");
            return Ok(true);
        }
        let Some(bridge) = state.bridge.as_mut() else {
            warn!("rustmix-wave=rustmix-remote-command status=ignored reason=bridge-missing peer={addr}");
            return Ok(true);
        };
        match bridge.on_command_write(value) {
            RemoteBridgeWriteOutcome::Enqueued => {
                info!("rustmix-wave=rustmix-remote-command status=enqueued peer={addr} bytes={value:02X?}");
            }
            RemoteBridgeWriteOutcome::DuplicateOrUnsupported => {
                info!("rustmix-wave=rustmix-remote-command status=ignored reason=duplicate-or-unsupported peer={addr} bytes={value:02X?}");
            }
            RemoteBridgeWriteOutcome::InvalidPacket(error) => {
                warn!("rustmix-wave=rustmix-remote-command status=invalid peer={addr} error={error:?} bytes={value:02X?}");
            }
        }
        Ok(true)
    }

    #[allow(clippy::too_many_arguments)]
    fn send_write_response(
        &self,
        gatt_if: GattInterface,
        conn_id: ConnectionId,
        trans_id: TransferId,
        handle: Handle,
        offset: u16,
        need_rsp: bool,
        is_prep: bool,
        value: &[u8],
    ) -> Result<(), EspError> {
        if !need_rsp {
            return Ok(());
        }
        if is_prep {
            let mut state = self.state.lock().unwrap();
            state
                .response
                .attr_handle(handle)
                .auth_req(0)
                .offset(offset)
                .value(value)
                .map_err(|_| EspError::from_infallible::<ESP_FAIL>())?;
            self.gatts.send_response(
                gatt_if,
                conn_id,
                trans_id,
                GattStatus::Ok,
                Some(&state.response),
            )?;
        } else {
            self.gatts
                .send_response(gatt_if, conn_id, trans_id, GattStatus::Ok, None)?;
        }
        Ok(())
    }

    fn check_esp_status(&self, status: Result<(), EspError>) {
        if let Err(error) = status {
            warn!("rustmix-wave=rustmix-remote-ble status=callback-error error={error:?}");
        }
    }

    fn check_bt_status(&self, status: BtStatus) -> Result<(), EspError> {
        if matches!(status, BtStatus::Success) {
            Ok(())
        } else {
            warn!("rustmix-wave=rustmix-remote-ble status=bt-status-error status={status:?}");
            Err(EspError::from_infallible::<ESP_FAIL>())
        }
    }

    fn check_gatt_status(&self, status: GattStatus) -> Result<(), EspError> {
        if matches!(status, GattStatus::Ok) {
            Ok(())
        } else {
            warn!("rustmix-wave=rustmix-remote-ble status=gatt-status-error status={status:?}");
            Err(EspError::from_infallible::<ESP_FAIL>())
        }
    }
}
