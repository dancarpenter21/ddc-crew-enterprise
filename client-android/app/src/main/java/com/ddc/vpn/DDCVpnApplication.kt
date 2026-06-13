package com.ddc.vpn

import android.app.Application
import com.ddc.vpn.data.ProfileRepository
import com.ddc.vpn.tunnel.TunnelManager

class DDCVpnApplication : Application() {
    lateinit var repository: ProfileRepository
        private set
    lateinit var tunnelManager: TunnelManager
        private set

    override fun onCreate() {
        super.onCreate()
        repository = ProfileRepository(this)
        tunnelManager = TunnelManager(this)
    }
}
