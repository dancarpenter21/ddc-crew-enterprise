package com.ddc.vpn

import androidx.lifecycle.ViewModel
import androidx.lifecycle.ViewModelProvider
import com.ddc.vpn.data.ProfileRepository
import com.ddc.vpn.tunnel.TunnelManager

class MainViewModelFactory(
    private val repository: ProfileRepository,
    private val tunnelManager: TunnelManager,
) : ViewModelProvider.Factory {
    @Suppress("UNCHECKED_CAST")
    override fun <T : ViewModel> create(modelClass: Class<T>): T {
        require(modelClass == MainViewModel::class.java) { "unknown ViewModel ${modelClass.name}" }
        return MainViewModel(repository, tunnelManager) as T
    }
}
