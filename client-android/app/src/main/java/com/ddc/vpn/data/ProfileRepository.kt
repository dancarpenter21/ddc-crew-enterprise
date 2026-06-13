package com.ddc.vpn.data

import android.content.Context
import com.ddc.vpn.model.VpnProfile
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.withContext
import kotlinx.serialization.Serializable
import kotlinx.serialization.json.Json
import java.io.File

class ProfileRepository(context: Context) {
    private val storeFile = File(context.filesDir, "profiles.json")
    private val json = Json {
        prettyPrint = true
        ignoreUnknownKeys = true
    }

    suspend fun loadProfiles(): List<VpnProfile> = withContext(Dispatchers.IO) {
        if (!storeFile.exists()) return@withContext emptyList()
        val store = json.decodeFromString(ProfileStore.serializer(), storeFile.readText())
        store.profiles
    }

    suspend fun saveProfile(profile: VpnProfile): VpnProfile = withContext(Dispatchers.IO) {
        val normalized = ProfileValidator.normalize(profile)
        val current = loadProfiles().toMutableList()
        val index = current.indexOfFirst { it.id == normalized.id }
        if (index >= 0) current[index] = normalized else current += normalized
        flush(current)
        normalized
    }

    suspend fun deleteProfile(id: String): VpnProfile = withContext(Dispatchers.IO) {
        val current = loadProfiles().toMutableList()
        val index = current.indexOfFirst { it.id == id }
        require(index >= 0) { "unknown profile $id" }
        val deleted = current.removeAt(index)
        flush(current)
        deleted
    }

    private fun flush(profiles: List<VpnProfile>) {
        storeFile.parentFile?.mkdirs()
        storeFile.writeText(json.encodeToString(ProfileStore.serializer(), ProfileStore(profiles)))
    }
}

@Serializable
private data class ProfileStore(val profiles: List<VpnProfile> = emptyList())
