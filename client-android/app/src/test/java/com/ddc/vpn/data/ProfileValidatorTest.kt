package com.ddc.vpn.data

import com.ddc.vpn.model.VpnProfile
import org.junit.Assert.assertFalse
import org.junit.Test

class ProfileValidatorTest {
    @Test
    fun validatesProfileAndAssignsId() {
        val normalized = ProfileValidator.normalize(profile())
        assertFalse(normalized.id.isEmpty())
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsInvalidKey() {
        ProfileValidator.normalize(profile().copy(privateKey = "not-a-key"))
    }

    @Test
    fun acceptsDnsEndpoint() {
        ProfileValidator.normalize(profile().copy(endpoint = "vpn.example.com:51820"))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsMissingEndpointPort() {
        ProfileValidator.normalize(profile().copy(endpoint = "vpn.example.com"))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsEmptyAllowedIps() {
        ProfileValidator.normalize(profile().copy(allowedIps = emptyList()))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsInvalidDnsServer() {
        ProfileValidator.normalize(profile().copy(dnsServers = listOf("not-an-ip")))
    }

    @Test(expected = IllegalArgumentException::class)
    fun rejectsOutOfRangeMtu() {
        ProfileValidator.normalize(profile().copy(mtu = 100))
    }

    private fun profile() = VpnProfile(
        id = "",
        name = "test",
        privateKey = "0000000000000000000000000000000000000000000000000000000000000001",
        serverPublicKey = "0000000000000000000000000000000000000000000000000000000000000002",
        presharedKey = null,
        endpoint = "127.0.0.1:51820",
        tunnelAddress = "10.44.0.2/32",
        allowedIps = listOf("10.44.0.0/24"),
        dnsServers = listOf("1.1.1.1"),
        mtu = 1420,
        persistentKeepaliveSeconds = 25,
    )
}
