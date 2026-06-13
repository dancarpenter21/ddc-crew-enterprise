import React, { useEffect, useMemo, useState } from "react";
import ReactDOM from "react-dom/client";
import { invoke } from "@tauri-apps/api/core";
import { Activity, FileKey2, Power, RefreshCw, Trash2 } from "lucide-react";
import "./styles.css";

type ConnectionState =
  | "disconnected"
  | "connecting"
  | "connected"
  | "disconnecting"
  | "failed";

type VpnProfile = {
  id: string;
  name: string;
  private_key: string;
  server_public_key: string;
  preshared_key?: string | null;
  endpoint: string;
  tunnel_address: string;
  allowed_ips: string[];
  dns_servers: string[];
  mtu: number;
  persistent_keepalive_seconds: number;
};

type VpnStatus = {
  state: ConnectionState;
  active_profile_id?: string | null;
  active_profile_name?: string | null;
  endpoint?: string | null;
  tunnel_address?: string | null;
  last_error?: string | null;
};

const emptyProfile: VpnProfile = {
  id: "",
  name: "Production VPN",
  private_key: "",
  server_public_key: "",
  preshared_key: "",
  endpoint: "vpn.example.com:51820",
  tunnel_address: "10.44.0.2/32",
  allowed_ips: ["0.0.0.0/0"],
  dns_servers: ["1.1.1.1"],
  mtu: 1420,
  persistent_keepalive_seconds: 25,
};

function App() {
  const [profiles, setProfiles] = useState<VpnProfile[]>([]);
  const [selectedId, setSelectedId] = useState("");
  const [draft, setDraft] = useState<VpnProfile>(emptyProfile);
  const [status, setStatus] = useState<VpnStatus>({ state: "disconnected" });
  const [logs, setLogs] = useState<string[]>([]);

  const selected = useMemo(
    () => profiles.find((profile) => profile.id === selectedId),
    [profiles, selectedId],
  );

  async function refresh() {
    const [nextProfiles, nextStatus, nextLogs] = await Promise.all([
      invoke<VpnProfile[]>("list_profiles"),
      invoke<VpnStatus>("status"),
      invoke<string[]>("recent_logs"),
    ]);
    setProfiles(nextProfiles);
    setStatus(nextStatus);
    setLogs(nextLogs);
    if (!selectedId && nextProfiles[0]) {
      setSelectedId(nextProfiles[0].id);
      setDraft(nextProfiles[0]);
    }
  }

  useEffect(() => {
    refresh();
    const timer = window.setInterval(refresh, 2000);
    return () => window.clearInterval(timer);
  }, []);

  useEffect(() => {
    if (selected) setDraft(selected);
  }, [selected]);

  async function saveProfile() {
    const saved = await invoke<VpnProfile>("save_profile", { profile: draft });
    setSelectedId(saved.id);
    setDraft(saved);
    await refresh();
  }

  async function deleteProfile() {
    if (!selectedId) return;
    await invoke("delete_profile", { id: selectedId });
    setSelectedId("");
    setDraft(emptyProfile);
    await refresh();
  }

  async function toggleConnection() {
    if (status.state === "connected" || status.state === "connecting") {
      await invoke("disconnect");
    } else if (selectedId) {
      await invoke("connect", { profileId: selectedId });
    }
    await refresh();
  }

  const canConnect = selectedId && status.state !== "disconnecting";

  return (
    <main className="shell">
      <section className="sidebar">
        <div className="brand">
          <Activity size={24} />
          <div>
            <h1>DDC VPN</h1>
            <span>Windows client</span>
          </div>
        </div>
        <button className="new-profile" onClick={() => setDraft(emptyProfile)}>
          <FileKey2 size={16} /> New profile
        </button>
        <div className="profile-list">
          {profiles.map((profile) => (
            <button
              key={profile.id}
              className={profile.id === selectedId ? "profile active" : "profile"}
              onClick={() => setSelectedId(profile.id)}
            >
              <strong>{profile.name}</strong>
              <span>{profile.endpoint}</span>
            </button>
          ))}
          {profiles.length === 0 && <p className="empty">No profiles saved.</p>}
        </div>
      </section>

      <section className="content">
        <div className="status-bar">
          <div>
            <span className={`state ${status.state}`}>{status.state}</span>
            <h2>{status.active_profile_name ?? draft.name}</h2>
            <p>{status.endpoint ?? draft.endpoint}</p>
          </div>
          <button className="connect" disabled={!canConnect} onClick={toggleConnection}>
            <Power size={18} />
            {status.state === "connected" || status.state === "connecting"
              ? "Disconnect"
              : "Connect"}
          </button>
        </div>

        {status.last_error && <div className="error">{status.last_error}</div>}

        <div className="workspace">
          <form className="editor" onSubmit={(event) => event.preventDefault()}>
            <label>
              Profile name
              <input value={draft.name} onChange={(event) => setDraft({ ...draft, name: event.target.value })} />
            </label>
            <label>
              Endpoint
              <input value={draft.endpoint} onChange={(event) => setDraft({ ...draft, endpoint: event.target.value })} />
            </label>
            <label>
              Tunnel address
              <input value={draft.tunnel_address} onChange={(event) => setDraft({ ...draft, tunnel_address: event.target.value })} />
            </label>
            <label>
              Allowed IPs
              <input value={draft.allowed_ips.join(", ")} onChange={(event) => setDraft({ ...draft, allowed_ips: splitList(event.target.value) })} />
            </label>
            <label>
              DNS servers
              <input value={draft.dns_servers.join(", ")} onChange={(event) => setDraft({ ...draft, dns_servers: splitList(event.target.value) })} />
            </label>
            <div className="grid-two">
              <label>
                MTU
                <input type="number" value={draft.mtu} onChange={(event) => setDraft({ ...draft, mtu: Number(event.target.value) })} />
              </label>
              <label>
                Keepalive
                <input type="number" value={draft.persistent_keepalive_seconds} onChange={(event) => setDraft({ ...draft, persistent_keepalive_seconds: Number(event.target.value) })} />
              </label>
            </div>
            <label>
              Private key
              <textarea value={draft.private_key} onChange={(event) => setDraft({ ...draft, private_key: event.target.value })} />
            </label>
            <label>
              Server public key
              <textarea value={draft.server_public_key} onChange={(event) => setDraft({ ...draft, server_public_key: event.target.value })} />
            </label>
            <label>
              Preshared key
              <textarea value={draft.preshared_key ?? ""} onChange={(event) => setDraft({ ...draft, preshared_key: event.target.value })} />
            </label>
            <div className="actions">
              <button type="button" onClick={saveProfile}>Save profile</button>
              <button type="button" className="danger" disabled={!selectedId} onClick={deleteProfile}>
                <Trash2 size={16} /> Delete
              </button>
            </div>
          </form>

          <aside className="logs">
            <div className="logs-title">
              <h3>Recent logs</h3>
              <button onClick={refresh} aria-label="Refresh logs">
                <RefreshCw size={16} />
              </button>
            </div>
            {logs.map((line, index) => <code key={`${line}-${index}`}>{line}</code>)}
            {logs.length === 0 && <p className="empty">No events yet.</p>}
          </aside>
        </div>
      </section>
    </main>
  );
}

function splitList(value: string): string[] {
  return value.split(",").map((item) => item.trim()).filter(Boolean);
}

ReactDOM.createRoot(document.getElementById("root")!).render(<App />);
