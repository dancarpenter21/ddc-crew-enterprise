use std::{collections::VecDeque, sync::Arc};

use tokio::sync::Mutex;

use crate::{
    profile::{ConnectionState, VpnProfile, VpnStatus},
    storage::ProfileStore,
    tunnel::TunnelManager,
};

#[derive(Clone)]
pub struct AppState {
    inner: Arc<Mutex<Inner>>,
}

pub struct Inner {
    pub store: ProfileStore,
    pub status: VpnStatus,
    pub logs: VecDeque<String>,
    pub tunnel: TunnelManager,
}

impl AppState {
    pub fn load() -> Self {
        let mut logs = VecDeque::with_capacity(200);
        let store = match ProfileStore::load_default() {
            Ok(store) => store,
            Err(error) => {
                logs.push_back(format!("failed to load profile store: {error}"));
                ProfileStore::empty_default()
            }
        };

        Self {
            inner: Arc::new(Mutex::new(Inner {
                store,
                status: VpnStatus::default(),
                logs,
                tunnel: TunnelManager::new(),
            })),
        }
    }

    pub async fn list_profiles(&self) -> Vec<VpnProfile> {
        self.inner.lock().await.store.profiles().to_vec()
    }

    pub async fn save_profile(&self, profile: VpnProfile) -> anyhow::Result<VpnProfile> {
        let mut inner = self.inner.lock().await;
        let saved = inner.store.save(profile)?;
        inner.store.flush()?;
        inner.log(format!("saved profile {}", saved.name));
        Ok(saved)
    }

    pub async fn delete_profile(&self, id: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        let deleted = inner.store.delete(id)?;
        if inner.status.active_profile_id.as_deref() == Some(id) {
            inner.tunnel.disconnect().await?;
            inner.status = VpnStatus::default();
        }
        inner.store.flush()?;
        inner.log(format!("deleted profile {}", deleted.name));
        Ok(())
    }

    pub async fn connect(&self, profile_id: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        let profile = inner.store.find(profile_id)?.clone();
        inner.status = VpnStatus::connecting(&profile);
        inner.log(format!("connecting to {}", profile.endpoint));

        match inner.tunnel.connect(&profile).await {
            Ok(()) => {
                inner.status = VpnStatus::connected(&profile);
                inner.log(format!("connected profile {}", profile.name));
                Ok(())
            }
            Err(error) => {
                inner.status = VpnStatus::failed(&profile, error.to_string());
                inner.log(format!("connect failed: {error}"));
                Err(error)
            }
        }
    }

    pub async fn disconnect(&self) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().await;
        inner.status.state = ConnectionState::Disconnecting;
        inner.log("disconnecting".to_string());
        inner.tunnel.disconnect().await?;
        inner.status = VpnStatus::default();
        inner.log("disconnected".to_string());
        Ok(())
    }

    pub async fn status(&self) -> VpnStatus {
        self.inner.lock().await.status.clone()
    }

    pub async fn recent_logs(&self) -> Vec<String> {
        self.inner.lock().await.logs.iter().cloned().collect()
    }
}

impl Inner {
    pub fn log(&mut self, message: String) {
        if self.logs.len() == 200 {
            self.logs.pop_front();
        }
        let now = time::OffsetDateTime::now_utc()
            .format(&time::macros::format_description!(
                "[year]-[month]-[day] [hour]:[minute]:[second]Z"
            ))
            .unwrap_or_else(|_| "unknown-time".to_string());
        self.logs.push_back(format!("{now} {message}"));
    }
}
