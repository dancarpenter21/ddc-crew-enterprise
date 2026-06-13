package com.ddc.vpn.tunnel

import com.ddc.vpn.data.ProfileValidator
import com.ddc.vpn.model.VpnProfile
import com.wireguard.config.Config
import java.io.ByteArrayInputStream

object WireGuardConfig {
    fun render(profile: VpnProfile): String {
        ProfileValidator.validate(profile)
        return buildString {
            appendLine("[Interface]")
            appendLine("PrivateKey = ${profile.privateKey.trim()}")
            appendLine("Address = ${profile.tunnelAddress}")
            appendLine("MTU = ${profile.mtu}")
            if (profile.dnsServers.isNotEmpty()) {
                appendLine("DNS = ${profile.dnsServers.joinToString(", ")}")
            }
            appendLine()
            appendLine("[Peer]")
            appendLine("PublicKey = ${profile.serverPublicKey.trim()}")
            profile.presharedKey?.trim()?.takeIf { it.isNotEmpty() }?.let {
                appendLine("PresharedKey = $it")
            }
            appendLine("Endpoint = ${profile.endpoint}")
            appendLine("AllowedIPs = ${profile.allowedIps.joinToString(", ")}")
            if (profile.persistentKeepaliveSeconds > 0) {
                appendLine("PersistentKeepalive = ${profile.persistentKeepaliveSeconds}")
            }
        }
    }

    fun parse(profile: VpnProfile): Config {
        val bytes = render(profile).toByteArray(Charsets.UTF_8)
        return Config.parse(ByteArrayInputStream(bytes))
    }
}
