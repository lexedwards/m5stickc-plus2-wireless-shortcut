use esp_idf_svc::nvs::{EspDefaultNvs, EspDefaultNvsPartition};

use crate::model::Target;

const NAMESPACE: &str = "wireless_btn";
const TARGET_KEY: &str = "target";
const HOST_KEYS: [&str; 2] = ["host1", "host2"];

pub struct Storage {
    nvs: EspDefaultNvs,
}

impl Storage {
    pub fn open() -> anyhow::Result<Self> {
        let partition = EspDefaultNvsPartition::take()?;
        let nvs = EspDefaultNvs::new(partition, NAMESPACE, true)?;
        Ok(Self { nvs })
    }

    pub fn load_target(&self) -> anyhow::Result<Target> {
        Ok(Target::from_storage(
            self.nvs.get_u8(TARGET_KEY)?.unwrap_or(0),
        ))
    }

    pub fn save_target(&self, target: Target) -> anyhow::Result<()> {
        self.nvs.set_u8(TARGET_KEY, target.storage_value())?;
        Ok(())
    }

    pub fn load_hosts(&self) -> anyhow::Result<[Option<[u8; 6]>; 2]> {
        let mut hosts = [None, None];
        for (index, key) in HOST_KEYS.iter().enumerate() {
            let mut buffer = [0u8; 6];
            if let Some(value) = self.nvs.get_blob(key, &mut buffer)? {
                if value.len() == buffer.len() {
                    hosts[index] = Some(buffer);
                }
            }
        }
        Ok(hosts)
    }

    pub fn save_hosts(&mut self, hosts: [Option<[u8; 6]>; 2]) -> anyhow::Result<()> {
        for (index, identity) in hosts.iter().enumerate() {
            if let Some(identity) = identity {
                self.nvs.set_blob(HOST_KEYS[index], identity)?;
            }
        }
        Ok(())
    }
}
