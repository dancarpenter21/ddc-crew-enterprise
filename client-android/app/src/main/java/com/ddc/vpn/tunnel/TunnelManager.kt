package com.ddc.vpn.tunnel

import android.content.Context
import android.content.Intent
import android.net.VpnService
import com.ddc.vpn.model.VpnProfile
import com.wireguard.android.backend.GoBackend
import com.wireguard.android.backend.Tunnel
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext

class TunnelManager(context: Context) {
    private val appContext = context.applicationContext
    private val backend by lazy { GoBackend(appContext) }
    private var activeProfileId: String? = null
    private var activeTunnel: AppTunnel? = null

    fun prepareIntent(): Intent? = VpnService.prepare(appContext)

    suspend fun connect(profile: VpnProfile) = withContext(Dispatchers.IO) {
        if (activeProfileId == profile.id && activeTunnel?.state?.value == Tunnel.State.UP) return@withContext
        if (activeProfileId != null) disconnect()

        val tunnel = AppTunnel(tunnelName(profile))
        val config = WireGuardConfig.parse(profile)
        val state = backend.setState(tunnel, Tunnel.State.UP, config)
        require(state == Tunnel.State.UP) { "WireGuard backend did not enter UP state" }
        activeProfileId = profile.id
        activeTunnel = tunnel
    }

    suspend fun disconnect() = withContext(Dispatchers.IO) {
        activeTunnel?.let { tunnel ->
            backend.setState(tunnel, Tunnel.State.DOWN, null)
        }
        activeTunnel = null
        activeProfileId = null
    }

    private fun tunnelName(profile: VpnProfile): String {
        val suffix = profile.id.take(8).filter { it.isLetterOrDigit() }
        return "ddc-vpn-${suffix.ifEmpty { "android" }}"
    }
}
