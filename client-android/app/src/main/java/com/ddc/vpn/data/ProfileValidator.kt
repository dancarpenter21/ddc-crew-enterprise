package com.ddc.vpn.data

import com.ddc.vpn.model.VpnProfile
import java.net.InetAddress
import java.util.Base64
import java.util.UUID

object ProfileValidator {
    fun normalize(profile: VpnProfile): VpnProfile {
        val normalized = profile.copy(
            id = profile.id.trim().ifEmpty { UUID.randomUUID().toString() },
            name = profile.name.trim(),
            privateKey = profile.privateKey.trim(),
            serverPublicKey = profile.serverPublicKey.trim(),
            presharedKey = profile.presharedKey?.trim()?.ifEmpty { null },
            endpoint = profile.endpoint.trim(),
            tunnelAddress = profile.tunnelAddress.trim(),
            allowedIps = profile.allowedIps.map { it.trim() }.filter { it.isNotEmpty() },
            dnsServers = profile.dnsServers.map { it.trim() }.filter { it.isNotEmpty() },
        )
        validate(normalized)
        return normalized
    }

    fun validate(profile: VpnProfile) {
        require(profile.name.isNotEmpty()) { "profile name is required" }
        parseKey(profile.privateKey, "private key")
        parseKey(profile.serverPublicKey, "server public key")
        profile.presharedKey?.trim()?.takeIf { it.isNotEmpty() }?.let { parseKey(it, "preshared key") }
        validateEndpoint(profile.endpoint)
        validateCidr(profile.tunnelAddress, "tunnel address")
        require(profile.allowedIps.isNotEmpty()) { "at least one allowed IP is required" }
        profile.allowedIps.forEach { validateCidr(it, "allowed IP $it") }
        profile.dnsServers.forEach { validateIpAddress(it, "DNS server $it") }
        require(profile.mtu in 576..9000) { "MTU must be between 576 and 9000" }
        require(profile.persistentKeepaliveSeconds in 0..65535) {
            "persistent keepalive must be between 0 and 65535"
        }
    }

    fun parseKey(raw: String, label: String = "key"): ByteArray {
        val trimmed = raw.trim()
        val bytes = if (trimmed.length == 64 && trimmed.all { it in '0'..'9' || it in 'a'..'f' || it in 'A'..'F' }) {
            trimmed.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
        } else {
            try {
                Base64.getDecoder().decode(trimmed)
            } catch (error: IllegalArgumentException) {
                throw IllegalArgumentException("invalid $label", error)
            }
        }
        require(bytes.size == 32) { "$label must decode to exactly 32 bytes" }
        return bytes
    }

    private fun validateEndpoint(endpoint: String) {
        val separator = endpoint.lastIndexOf(':')
        require(separator > 0 && separator < endpoint.lastIndex) { "endpoint must include a port" }
        val host = endpoint.substring(0, separator).trim()
        val port = endpoint.substring(separator + 1).toIntOrNull()
        require(host.isNotEmpty()) { "endpoint host is required" }
        require(port != null && port in 1..65535) { "endpoint port must be a valid TCP/UDP port" }
    }

    private fun validateCidr(value: String, label: String) {
        val parts = value.split('/')
        require(parts.size == 2) { "invalid $label" }
        val address = validateIpAddress(parts[0], label)
        val maxMask = if (address.size == 4) 32 else 128
        val mask = parts[1].toIntOrNull()
        require(mask != null && mask in 0..maxMask) { "invalid $label mask" }
    }

    private fun validateIpAddress(value: String, label: String): ByteArray {
        return try {
            InetAddress.getByName(value).address
        } catch (error: Exception) {
            throw IllegalArgumentException("invalid $label", error)
        }
    }
}
