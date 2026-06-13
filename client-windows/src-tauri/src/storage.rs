use std::{fs, path::PathBuf};

use anyhow::{bail, Context};
use serde::{Deserialize, Serialize};

use crate::profile::VpnProfile;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct StoredProfiles {
    profiles: Vec<VpnProfile>,
}

#[derive(Debug, Clone)]
pub struct ProfileStore {
    path: PathBuf,
    profiles: Vec<VpnProfile>,
}

impl ProfileStore {
    pub fn load_default() -> anyhow::Result<Self> {
        let path = default_store_path()?;
        if !path.exists() {
            return Ok(Self {
                path,
                profiles: Vec::new(),
            });
        }

        let raw = fs::read_to_string(&path)
            .with_context(|| format!("read profile store {}", path.display()))?;
        let stored: StoredProfiles = toml::from_str(&raw).context("parse profile store")?;
        Ok(Self {
            path,
            profiles: stored.profiles,
        })
    }

    pub fn empty_default() -> Self {
        let path = default_store_path().unwrap_or_else(|_| PathBuf::from("profiles.toml"));
        Self {
            path,
            profiles: Vec::new(),
        }
    }

    pub fn profiles(&self) -> &[VpnProfile] {
        &self.profiles
    }

    pub fn find(&self, id: &str) -> anyhow::Result<&VpnProfile> {
        self.profiles
            .iter()
            .find(|profile| profile.id == id)
            .with_context(|| format!("unknown profile {id}"))
    }

    pub fn save(&mut self, profile: VpnProfile) -> anyhow::Result<VpnProfile> {
        let profile = profile.normalize()?;
        if let Some(existing) = self
            .profiles
            .iter_mut()
            .find(|existing| existing.id == profile.id)
        {
            *existing = profile.clone();
        } else {
            self.profiles.push(profile.clone());
        }
        Ok(profile)
    }

    pub fn delete(&mut self, id: &str) -> anyhow::Result<VpnProfile> {
        let index = self
            .profiles
            .iter()
            .position(|profile| profile.id == id)
            .with_context(|| format!("unknown profile {id}"))?;
        Ok(self.profiles.remove(index))
    }

    pub fn flush(&self) -> anyhow::Result<()> {
        if self.path.as_os_str().is_empty() {
            bail!("profile store path is empty");
        }
        if let Some(parent) = self.path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("create profile dir {}", parent.display()))?;
        }
        let stored = StoredProfiles {
            profiles: self.profiles.clone(),
        };
        let raw = toml::to_string_pretty(&stored).context("serialize profile store")?;
        fs::write(&self.path, raw)
            .with_context(|| format!("write profile store {}", self.path.display()))?;
        Ok(())
    }
}

fn default_store_path() -> anyhow::Result<PathBuf> {
    let mut dir = dirs::data_local_dir().context("cannot resolve local app data directory")?;
    dir.push("DDC");
    dir.push("VPN");
    dir.push("profiles.toml");
    Ok(dir)
}
