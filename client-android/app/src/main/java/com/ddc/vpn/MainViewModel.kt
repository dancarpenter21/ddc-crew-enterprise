package com.ddc.vpn

import android.content.Intent
import androidx.lifecycle.ViewModel
import androidx.lifecycle.viewModelScope
import com.ddc.vpn.data.ProfileRepository
import com.ddc.vpn.model.ConnectionState
import com.ddc.vpn.model.VpnProfile
import com.ddc.vpn.model.VpnStatus
import com.ddc.vpn.tunnel.TunnelManager
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.launch
import java.time.Instant

data class MainUiState(
    val profiles: List<VpnProfile> = emptyList(),
    val selectedProfileId: String = "",
    val draft: VpnProfile = VpnProfile(),
    val status: VpnStatus = VpnStatus(),
    val logs: List<String> = emptyList(),
) {
    val selectedProfile: VpnProfile?
        get() = profiles.firstOrNull { it.id == selectedProfileId }
}

class MainViewModel(
    private val repository: ProfileRepository,
    private val tunnelManager: TunnelManager,
) : ViewModel() {
    private val _uiState = MutableStateFlow(MainUiState())
    val uiState: StateFlow<MainUiState> = _uiState.asStateFlow()

    init {
        refresh()
    }

    fun refresh() {
        viewModelScope.launch {
            runCatching { repository.loadProfiles() }
                .onSuccess { profiles ->
                    _uiState.value = _uiState.value.let { state ->
                        val selectedId = state.selectedProfileId.ifEmpty { profiles.firstOrNull()?.id.orEmpty() }
                        state.copy(
                            profiles = profiles,
                            selectedProfileId = selectedId,
                            draft = profiles.firstOrNull { it.id == selectedId } ?: state.draft,
                        )
                    }
                }
                .onFailure { log("failed to load profiles: ${it.message}") }
        }
    }

    fun selectProfile(id: String) {
        val profile = _uiState.value.profiles.firstOrNull { it.id == id } ?: return
        _uiState.value = _uiState.value.copy(selectedProfileId = id, draft = profile)
    }

    fun newProfile() {
        _uiState.value = _uiState.value.copy(selectedProfileId = "", draft = VpnProfile())
    }

    fun updateDraft(profile: VpnProfile) {
        _uiState.value = _uiState.value.copy(draft = profile)
    }

    fun saveProfile() {
        val draft = _uiState.value.draft
        viewModelScope.launch {
            runCatching { repository.saveProfile(draft) }
                .onSuccess { saved ->
                    log("saved profile ${saved.name}")
                    _uiState.value = _uiState.value.copy(selectedProfileId = saved.id, draft = saved)
                    refresh()
                }
                .onFailure { log("save failed: ${it.message}") }
        }
    }

    fun deleteProfile() {
        val id = _uiState.value.selectedProfileId
        if (id.isEmpty()) return
        viewModelScope.launch {
            if (_uiState.value.status.activeProfileId == id) {
                runCatching { tunnelManager.disconnect() }
                _uiState.value = _uiState.value.copy(status = VpnStatus())
            }
            runCatching { repository.deleteProfile(id) }
                .onSuccess { deleted ->
                    log("deleted profile ${deleted.name}")
                    _uiState.value = _uiState.value.copy(selectedProfileId = "", draft = VpnProfile())
                    refresh()
                }
                .onFailure { log("delete failed: ${it.message}") }
        }
    }

    fun prepareVpnIntent(): Intent? {
        val profile = _uiState.value.selectedProfile ?: return null
        _uiState.value = _uiState.value.copy(status = VpnStatus.preparing(profile))
        return tunnelManager.prepareIntent()
    }

    fun vpnPermissionDenied() {
        val profile = _uiState.value.selectedProfile ?: return
        val message = "Android VPN permission was not granted"
        _uiState.value = _uiState.value.copy(status = VpnStatus.failed(profile, message))
        log(message)
    }

    fun connectSelected() {
        val profile = _uiState.value.selectedProfile ?: return
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(status = VpnStatus.connecting(profile))
            log("connecting to ${profile.endpoint}")
            runCatching { tunnelManager.connect(profile) }
                .onSuccess {
                    _uiState.value = _uiState.value.copy(status = VpnStatus.connected(profile))
                    log("connected profile ${profile.name}")
                }
                .onFailure { error ->
                    _uiState.value = _uiState.value.copy(status = VpnStatus.failed(profile, error.message ?: "connect failed"))
                    log("connect failed: ${error.message}")
                }
        }
    }

    fun disconnect() {
        viewModelScope.launch {
            _uiState.value = _uiState.value.copy(status = _uiState.value.status.copy(state = ConnectionState.Disconnecting))
            log("disconnecting")
            runCatching { tunnelManager.disconnect() }
                .onSuccess {
                    _uiState.value = _uiState.value.copy(status = VpnStatus())
                    log("disconnected")
                }
                .onFailure { log("disconnect failed: ${it.message}") }
        }
    }

    private fun log(message: String) {
        val line = "${Instant.now()} $message"
        _uiState.value = _uiState.value.copy(logs = (_uiState.value.logs + line).takeLast(200))
    }
}
