package com.ddc.vpn.tunnel

import com.wireguard.android.backend.Tunnel
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow

class AppTunnel(private val tunnelName: String) : Tunnel {
    private val _state = MutableStateFlow(Tunnel.State.DOWN)
    val state: StateFlow<Tunnel.State> = _state

    override fun getName(): String = tunnelName

    override fun onStateChange(newState: Tunnel.State) {
        _state.value = newState
    }
}
