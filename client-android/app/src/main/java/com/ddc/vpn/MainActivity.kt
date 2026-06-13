package com.ddc.vpn

import android.app.Activity
import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Button
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import androidx.lifecycle.viewmodel.compose.viewModel
import com.ddc.vpn.model.ConnectionState
import com.ddc.vpn.model.VpnProfile
import com.ddc.vpn.ui.theme.DDCVpnTheme

class MainActivity : ComponentActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val app = application as DDCVpnApplication
        setContent {
            val factory = remember {
                MainViewModelFactory(app.repository, app.tunnelManager)
            }
            val viewModel: MainViewModel = viewModel(factory = factory)
            DDCVpnTheme {
                VpnApp(viewModel)
            }
        }
    }
}

@Composable
private fun VpnApp(viewModel: MainViewModel) {
    val state by viewModel.uiState.collectAsState()
    val vpnConsent = rememberLauncherForActivityResult(ActivityResultContracts.StartActivityForResult()) { result ->
        if (result.resultCode == Activity.RESULT_OK) {
            viewModel.connectSelected()
        } else {
            viewModel.vpnPermissionDenied()
        }
    }

    Scaffold { padding ->
        Surface(Modifier.fillMaxSize().padding(padding)) {
            Column(
                modifier = Modifier
                    .fillMaxSize()
                    .padding(16.dp),
                verticalArrangement = Arrangement.spacedBy(14.dp),
            ) {
                Header(state.status.state.name, state.status.lastError)
                ProfileList(
                    profiles = state.profiles,
                    selectedId = state.selectedProfileId,
                    onSelect = viewModel::selectProfile,
                    onNew = viewModel::newProfile,
                )
                ProfileEditor(
                    draft = state.draft,
                    onChange = viewModel::updateDraft,
                    onSave = viewModel::saveProfile,
                    onDelete = viewModel::deleteProfile,
                    canDelete = state.selectedProfileId.isNotEmpty(),
                )
                ConnectionControls(
                    connected = state.status.state == ConnectionState.Connected || state.status.state == ConnectionState.Connecting,
                    canConnect = state.selectedProfileId.isNotEmpty(),
                    onConnect = {
                        val intent: Intent? = viewModel.prepareVpnIntent()
                        if (intent == null) viewModel.connectSelected() else vpnConsent.launch(intent)
                    },
                    onDisconnect = viewModel::disconnect,
                )
                Logs(state.logs)
            }
        }
    }
}

@Composable
private fun Header(state: String, error: String?) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("DDC VPN", style = MaterialTheme.typography.headlineMedium, fontWeight = FontWeight.Bold)
        Text(state, style = MaterialTheme.typography.labelLarge)
        if (!error.isNullOrBlank()) {
            Text(error, color = MaterialTheme.colorScheme.error)
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProfileList(
    profiles: List<VpnProfile>,
    selectedId: String,
    onSelect: (String) -> Unit,
    onNew: () -> Unit,
) {
    Column(verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Text("Profiles", modifier = Modifier.weight(1f), fontWeight = FontWeight.SemiBold)
            TextButton(onClick = onNew) { Text("New") }
        }
        if (profiles.isEmpty()) {
            Text("No profiles saved.")
        } else {
            LazyColumn(verticalArrangement = Arrangement.spacedBy(8.dp), modifier = Modifier.fillMaxWidth()) {
                items(profiles, key = { it.id }) { profile ->
                    Card(
                        colors = CardDefaults.cardColors(
                            containerColor = if (profile.id == selectedId) MaterialTheme.colorScheme.primaryContainer else MaterialTheme.colorScheme.surfaceVariant,
                        ),
                        onClick = { onSelect(profile.id) },
                    ) {
                        Column(Modifier.padding(12.dp)) {
                            Text(profile.name, fontWeight = FontWeight.SemiBold)
                            Text(profile.endpoint, style = MaterialTheme.typography.bodySmall)
                        }
                    }
                }
            }
        }
    }
}

@Composable
private fun ProfileEditor(
    draft: VpnProfile,
    onChange: (VpnProfile) -> Unit,
    onSave: () -> Unit,
    onDelete: () -> Unit,
    canDelete: Boolean,
) {
    Column(
        modifier = Modifier.verticalScroll(rememberScrollState()),
        verticalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        Text("Profile", fontWeight = FontWeight.SemiBold)
        Field("Profile name", draft.name) { onChange(draft.copy(name = it)) }
        Field("Endpoint", draft.endpoint) { onChange(draft.copy(endpoint = it)) }
        Field("Tunnel address", draft.tunnelAddress) { onChange(draft.copy(tunnelAddress = it)) }
        Field("Allowed IPs", draft.allowedIps.joinToString(", ")) { onChange(draft.copy(allowedIps = splitList(it))) }
        Field("DNS servers", draft.dnsServers.joinToString(", ")) { onChange(draft.copy(dnsServers = splitList(it))) }
        Field("MTU", draft.mtu.toString()) { onChange(draft.copy(mtu = it.toIntOrNull() ?: 0)) }
        Field("Keepalive", draft.persistentKeepaliveSeconds.toString()) {
            onChange(draft.copy(persistentKeepaliveSeconds = it.toIntOrNull() ?: 0))
        }
        Field("Private key", draft.privateKey) { onChange(draft.copy(privateKey = it)) }
        Field("Server public key", draft.serverPublicKey) { onChange(draft.copy(serverPublicKey = it)) }
        Field("Preshared key", draft.presharedKey.orEmpty()) { onChange(draft.copy(presharedKey = it)) }
        Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
            Button(onClick = onSave) { Text("Save profile") }
            OutlinedButton(onClick = onDelete, enabled = canDelete) { Text("Delete") }
        }
    }
}

@Composable
private fun Field(label: String, value: String, onChange: (String) -> Unit) {
    OutlinedTextField(
        value = value,
        onValueChange = onChange,
        label = { Text(label) },
        modifier = Modifier.fillMaxWidth(),
        singleLine = label != "Private key" && label != "Server public key" && label != "Preshared key",
    )
}

@Composable
private fun ConnectionControls(
    connected: Boolean,
    canConnect: Boolean,
    onConnect: () -> Unit,
    onDisconnect: () -> Unit,
) {
    Button(
        enabled = canConnect,
        onClick = if (connected) onDisconnect else onConnect,
        modifier = Modifier.fillMaxWidth(),
    ) {
        Text(if (connected) "Disconnect" else "Connect")
    }
}

@Composable
private fun Logs(logs: List<String>) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text("Recent logs", fontWeight = FontWeight.SemiBold)
        if (logs.isEmpty()) {
            Text("No events yet.")
        } else {
            logs.takeLast(8).forEach { Text(it, style = MaterialTheme.typography.bodySmall) }
        }
    }
}

private fun splitList(value: String): List<String> =
    value.split(",").map { it.trim() }.filter { it.isNotEmpty() }
