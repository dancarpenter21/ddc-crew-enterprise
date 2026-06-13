package com.ddc.vpn.tunnel

import com.ddc.vpn.model.VpnProfile
import org.junit.Assert.assertTrue
import org.junit.Test

class WireGuardConfigTest {
    @Test
    fun rendersWireGuardConfig() {
        val rendered = WireGuardConfig.render(profile())

        assertTrue(rendered.contains("[Interface]"))
        assertTrue(rendered.contains("PrivateKey = 0000000000000000000000000000000000000000000000000000000000000001"))
        assertTrue(rendered.contains("Address = 10.44.0.2/32"))
        assertTrue(rendered.contains("DNS = 1.1.1.1"))
        assertTrue(rendered.contains("MTU = 1420"))
        assertTrue(rendered.contains("[Peer]"))
        assertTrue(rendered.contains("PublicKey = 0000000000000000000000000000000000000000000000000000000000000002"))
        assertTrue(rendered.contains("Endpoint = 127.0.0.1:51820"))
        assertTrue(rendered.contains("AllowedIPs = 0.0.0.0/0"))
        assertTrue(rendered.contains("PersistentKeepalive = 25"))
    }

    private fun profile() = VpnProfile(
        name = "test",
        privateKey = "0000000000000000000000000000000000000000000000000000000000000001",
        serverPublicKey = "0000000000000000000000000000000000000000000000000000000000000002",
        endpoint = "127.0.0.1:51820",
        tunnelAddress = "10.44.0.2/32",
        allowedIps = listOf("0.0.0.0/0"),
        dnsServers = listOf("1.1.1.1"),
        mtu = 1420,
        persistentKeepaliveSeconds = 25,
    )
}
