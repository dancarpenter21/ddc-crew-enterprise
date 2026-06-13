package com.ddc.vpn.model

import kotlinx.serialization.Serializable

@Serializable
data class VpnProfile(
    val id: String = "",
    val name: String = "Production VPN",
    val privateKey: String = "",
    val serverPublicKey: String = "",
    val presharedKey: String? = "",
    val endpoint: String = "vpn.example.com:51820",
    val tunnelAddress: String = "10.44.0.2/32",
    val allowedIps: List<String> = listOf("0.0.0.0/0"),
    val dnsServers: List<String> = listOf("1.1.1.1"),
    val mtu: Int = 1420,
    val persistentKeepaliveSeconds: Int = 25,
)

enum class ConnectionState {
    Disconnected,
    Preparing,
    Connecting,
    Connected,
    Disconnecting,
    Failed,
}

data class VpnStatus(
    val state: ConnectionState = ConnectionState.Disconnected,
    val activeProfileId: String? = null,
    val activeProfileName: String? = null,
    val endpoint: String? = null,
    val tunnelAddress: String? = null,
    val lastError: String? = null,
) {
    companion object {
        fun preparing(profile: VpnProfile) = fromProfile(ConnectionState.Preparing, profile)
        fun connecting(profile: VpnProfile) = fromProfile(ConnectionState.Connecting, profile)
        fun connected(profile: VpnProfile) = fromProfile(ConnectionState.Connected, profile)
        fun failed(profile: VpnProfile, error: String) = fromProfile(ConnectionState.Failed, profile, error)

        private fun fromProfile(
            state: ConnectionState,
            profile: VpnProfile,
            lastError: String? = null,
        ) = VpnStatus(
            state = state,
            activeProfileId = profile.id,
            activeProfileName = profile.name,
            endpoint = profile.endpoint,
            tunnelAddress = profile.tunnelAddress,
            lastError = lastError,
        )
    }
}
