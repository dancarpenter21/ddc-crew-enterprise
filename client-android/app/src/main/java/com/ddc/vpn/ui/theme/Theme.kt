package com.ddc.vpn.ui.theme

import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color

private val Colors = lightColorScheme(
    primary = Color(0xFF155EEF),
    primaryContainer = Color(0xFFE8F1FF),
    surface = Color(0xFFF7FAFC),
    surfaceVariant = Color(0xFFEDF2F7),
    error = Color(0xFFB3261E),
)

@Composable
fun DDCVpnTheme(content: @Composable () -> Unit) {
    MaterialTheme(
        colorScheme = Colors,
        content = content,
    )
}
