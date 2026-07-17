use std::sync::{Arc, Mutex};

use esp32_nimble::{
    enums::{AuthReq, OwnAddrType, PairKeyDist, SecurityIOCap},
    utilities::{mutex::Mutex as NimbleMutex, BleUuid},
    BLEAdvertisementData, BLECharacteristic, BLEDevice, BLEHIDDevice, NimbleSub,
};
use esp_idf_svc::hal::delay::FreeRtos;

use crate::model::{Shortcut, Target};

const MAX_HOSTS: usize = 2;

const HID_DESCRIPTOR: &[u8] = &[
    0x05, 0x01, 0x09, 0x06, 0xA1, 0x01, 0x05, 0x07, 0x19, 0xE0, 0x29, 0xE7, 0x15, 0x00, 0x25, 0x01,
    0x75, 0x01, 0x95, 0x08, 0x81, 0x02, 0x95, 0x01, 0x75, 0x08, 0x81, 0x01, 0x95, 0x06, 0x75, 0x08,
    0x15, 0x00, 0x25, 0x65, 0x05, 0x07, 0x19, 0x00, 0x29, 0x65, 0x81, 0x00, 0xC0,
];

type InputHandle = Arc<NimbleMutex<BLECharacteristic>>;

#[derive(Clone, Copy, Debug, Default)]
struct HostSlot {
    identity: Option<[u8; 6]>,
    connection_handle: Option<u16>,
    ready: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HostView {
    pub identity: Option<[u8; 6]>,
    pub connected: bool,
    pub ready: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub struct HostsView {
    pub slots: [HostView; MAX_HOSTS],
    pub revision: u32,
}

impl HostsView {
    pub fn connected_count(&self) -> usize {
        self.slots.iter().filter(|host| host.connected).count()
    }

    pub fn target_ready(&self, target: Target) -> bool {
        match target {
            Target::All => self.slots.iter().any(|host| host.ready),
            Target::Device1 => self.slots[0].ready,
            Target::Device2 => self.slots[1].ready,
        }
    }
}

#[derive(Debug)]
pub struct HostState {
    slots: [HostSlot; MAX_HOSTS],
    revision: u32,
}

impl HostState {
    fn new(identities: [Option<[u8; 6]>; MAX_HOSTS]) -> Self {
        Self {
            slots: identities.map(|identity| HostSlot {
                identity,
                ..Default::default()
            }),
            revision: 1,
        }
    }

    fn view(&self) -> HostsView {
        HostsView {
            slots: self.slots.map(|host| HostView {
                identity: host.identity,
                connected: host.connection_handle.is_some(),
                ready: host.ready,
            }),
            revision: self.revision,
        }
    }

    fn identities(&self) -> [Option<[u8; 6]>; MAX_HOSTS] {
        self.slots.map(|host| host.identity)
    }

    fn find_slot(&self, identity: [u8; 6]) -> Option<usize> {
        self.slots
            .iter()
            .position(|slot| slot.identity == Some(identity))
    }

    fn assign_slot(&mut self, identity: [u8; 6]) -> Option<usize> {
        if let Some(index) = self.find_slot(identity) {
            return Some(index);
        }

        let index = self.slots.iter().position(|slot| slot.identity.is_none())?;
        self.slots[index].identity = Some(identity);
        self.revision = self.revision.wrapping_add(1);
        log::info!("assigned BLE host to Device {}", index + 1);
        Some(index)
    }

    fn connected(&mut self, identity: [u8; 6], handle: u16, assign: bool) {
        let index = if assign {
            self.assign_slot(identity)
        } else {
            self.find_slot(identity)
        };

        if let Some(index) = index {
            self.slots[index].connection_handle = Some(handle);
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn subscribed(&mut self, identity: [u8; 6], handle: u16, ready: bool) {
        if ready {
            if let Some(index) = self.assign_slot(identity) {
                self.slots[index].connection_handle = Some(handle);
                self.slots[index].ready = true;
                self.revision = self.revision.wrapping_add(1);
            }
        } else if let Some(index) = self.find_slot(identity) {
            self.slots[index].ready = false;
            self.revision = self.revision.wrapping_add(1);
        }
    }

    fn disconnected(&mut self, handle: u16) {
        for slot in &mut self.slots {
            if slot.connection_handle == Some(handle) {
                slot.connection_handle = None;
                slot.ready = false;
                self.revision = self.revision.wrapping_add(1);
            }
        }
    }

    fn recipients(&self, target: Target) -> [Option<u16>; MAX_HOSTS] {
        let ready_handle = |slot: HostSlot| slot.ready.then_some(slot.connection_handle).flatten();
        match target {
            Target::All => self.slots.map(ready_handle),
            Target::Device1 => [ready_handle(self.slots[0]), None],
            Target::Device2 => [ready_handle(self.slots[1]), None],
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SendResult {
    Sent(usize),
    NoDestination,
}

pub struct BleKeyboard {
    input: InputHandle,
    hosts: Arc<Mutex<HostState>>,
}

impl BleKeyboard {
    pub fn init(passkey: u32, identities: [Option<[u8; 6]>; MAX_HOSTS]) -> anyhow::Result<Self> {
        let hosts = Arc::new(Mutex::new(HostState::new(identities)));
        let device = BLEDevice::take();

        BLEDevice::set_device_name("Shortcut Button")?;
        device.set_own_addr_type(OwnAddrType::RpaPublicDefault);
        device
            .security()
            .set_auth(AuthReq::Bond | AuthReq::Mitm)
            .set_io_cap(SecurityIOCap::DisplayOnly)
            .set_passkey(passkey)
            .set_security_init_key(PairKeyDist::ENC | PairKeyDist::ID)
            .set_security_resp_key(PairKeyDist::ENC | PairKeyDist::ID);

        let server = device.get_server();
        server.advertise_on_disconnect(true);

        let connect_hosts = hosts.clone();
        let advertising = device.get_advertising();
        server.on_connect(move |server, desc| {
            let identity = desc.id_address().as_be_bytes();
            log::info!("BLE host connected: {:?}", desc.id_address());
            if let Ok(mut state) = connect_hosts.lock() {
                state.connected(identity, desc.conn_handle(), false);
            }
            if server.connected_count() < MAX_HOSTS {
                if let Err(error) = advertising.lock().start() {
                    log::warn!("failed to continue BLE advertising: {error:?}");
                }
            }
        });

        let disconnect_hosts = hosts.clone();
        server.on_disconnect(move |desc, reason| {
            log::info!(
                "BLE host disconnected: {:?} ({:?})",
                desc.id_address(),
                reason
            );
            if let Ok(mut state) = disconnect_hosts.lock() {
                state.disconnected(desc.conn_handle());
            }
        });

        let authenticated_hosts = hosts.clone();
        server.on_authentication_complete(move |_server, desc, result| {
            if result.is_ok() && desc.authenticated() {
                if let Ok(mut state) = authenticated_hosts.lock() {
                    state.connected(desc.id_address().as_be_bytes(), desc.conn_handle(), true);
                }
            } else {
                log::warn!(
                    "BLE authentication failed for {:?}: {:?}",
                    desc.address(),
                    result
                );
            }
        });

        let mut hid = BLEHIDDevice::new(server);
        let input = hid.input_report(0);
        let subscribed_hosts = hosts.clone();
        input
            .lock()
            .on_subscribe(move |_characteristic, desc, subscription| {
                let ready = subscription.contains(NimbleSub::NOTIFY);
                if let Ok(mut state) = subscribed_hosts.lock() {
                    state.subscribed(desc.id_address().as_be_bytes(), desc.conn_handle(), ready);
                }
            });

        hid.manufacturer("M5Stack");
        hid.pnp(0x02, 0x05ac, 0x820a, 0x0210);
        hid.hid_info(0x00, 0x01);
        hid.report_map(HID_DESCRIPTOR);
        hid.set_battery_level(100);

        device.get_advertising().lock().set_data(
            BLEAdvertisementData::new()
                .name("Shortcut Button")
                .appearance(0x03C1)
                .add_service_uuid(BleUuid::Uuid16(0x1812)),
        )?;
        device.get_advertising().lock().start()?;

        log::info!("BLE advertising; pairing PIN {passkey:06}");
        Ok(Self { input, hosts })
    }

    pub fn hosts_view(&self) -> HostsView {
        self.hosts
            .lock()
            .map(|state| state.view())
            .unwrap_or_default()
    }

    pub fn host_identities(&self) -> [Option<[u8; 6]>; MAX_HOSTS] {
        self.hosts
            .lock()
            .map(|state| state.identities())
            .unwrap_or([None, None])
    }

    pub fn send(&self, target: Target, shortcut: Shortcut) -> SendResult {
        let recipients = self
            .hosts
            .lock()
            .map(|state| state.recipients(target))
            .unwrap_or([None, None]);

        if recipients.iter().all(Option::is_none) {
            return SendResult::NoDestination;
        }

        let press = shortcut.press_report();
        let mut sent = [None; MAX_HOSTS];
        {
            let input = self.input.lock();
            for (index, handle) in recipients.into_iter().enumerate() {
                if let Some(handle) = handle {
                    match input.notify_with(&press, handle) {
                        Ok(()) => sent[index] = Some(handle),
                        Err(error) => log::warn!("HID key-down failed for {handle}: {error:?}"),
                    }
                }
            }
        }

        FreeRtos::delay_ms(20);

        let input = self.input.lock();
        let mut count = 0;
        for handle in sent.into_iter().flatten() {
            if let Err(error) = input.notify_with(&[0u8; 8], handle) {
                log::warn!("HID key-up failed for {handle}: {error:?}");
            } else {
                count += 1;
            }
        }

        if count == 0 {
            SendResult::NoDestination
        } else {
            SendResult::Sent(count)
        }
    }
}
