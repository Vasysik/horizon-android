package dev.horizon.android.launcher

import android.os.Build
import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.MaterialExpressiveTheme
import androidx.compose.material3.MotionScheme
import androidx.compose.material3.Typography
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.text.TextStyle

/** Horizon's violet, used where Android has no wallpaper colours (before Android 12). */
private val HorizonDark = darkColorScheme(
    primary = Color(0xFFCFBCFF),
    onPrimary = Color(0xFF381E72),
    primaryContainer = Color(0xFF4F378A),
    onPrimaryContainer = Color(0xFFE9DDFF),
    secondary = Color(0xFFCBC2DB),
    onSecondary = Color(0xFF332D41),
    secondaryContainer = Color(0xFF4A4458),
    onSecondaryContainer = Color(0xFFE8DEF8),
    tertiary = Color(0xFFEFB8C8),
    onTertiary = Color(0xFF4A2532),
    tertiaryContainer = Color(0xFF633B48),
    onTertiaryContainer = Color(0xFFFFD9E3),
    background = Color(0xFF141218),
    onBackground = Color(0xFFE6E0E9),
    surface = Color(0xFF141218),
    onSurface = Color(0xFFE6E0E9),
    surfaceVariant = Color(0xFF49454F),
    onSurfaceVariant = Color(0xFFCAC4D0),
    surfaceContainerLowest = Color(0xFF0F0D13),
    surfaceContainerLow = Color(0xFF1D1B20),
    surfaceContainer = Color(0xFF211F26),
    surfaceContainerHigh = Color(0xFF2B2930),
    surfaceContainerHighest = Color(0xFF36343B),
    outline = Color(0xFF948F99),
    outlineVariant = Color(0xFF49454F),
)

private val HorizonLight = lightColorScheme(
    primary = Color(0xFF65558F),
    onPrimary = Color(0xFFFFFFFF),
    primaryContainer = Color(0xFFE9DDFF),
    onPrimaryContainer = Color(0xFF22005D),
    secondaryContainer = Color(0xFFE8DEF8),
    onSecondaryContainer = Color(0xFF1E192B),
    tertiaryContainer = Color(0xFFFFD9E3),
    onTertiaryContainer = Color(0xFF31101D),
)

/**
 * Material's default scale is sized for roomy screens; phones at high display density ran out of
 * room, so large styles step down further than body and label text.
 */
private val CompactTypography = Typography().run {
    // Letter spacing keeps its sp unit: text fields animate their label between styles, and Compose
    // cannot interpolate em against sp.
    fun TextStyle.scaled(factor: Float) = copy(fontSize = fontSize * factor, lineHeight = lineHeight * factor)
    Typography(
        displayLarge = displayLarge.scaled(0.75f),
        displayMedium = displayMedium.scaled(0.75f),
        displaySmall = displaySmall.scaled(0.75f),
        headlineLarge = headlineLarge.scaled(0.8f),
        headlineMedium = headlineMedium.scaled(0.8f),
        headlineSmall = headlineSmall.scaled(0.85f),
        titleLarge = titleLarge.scaled(0.85f),
        titleMedium = titleMedium.scaled(0.9f),
        titleSmall = titleSmall.scaled(0.92f),
        bodyLarge = bodyLarge.scaled(0.9f),
        bodyMedium = bodyMedium.scaled(0.92f),
        bodySmall = bodySmall,
        labelLarge = labelLarge.scaled(0.92f),
        labelMedium = labelMedium,
        labelSmall = labelSmall,
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun HorizonTheme(content: @Composable () -> Unit) {
    val dark = isSystemInDarkTheme()
    val context = LocalContext.current
    val colors = when {
        Build.VERSION.SDK_INT >= Build.VERSION_CODES.S ->
            if (dark) dynamicDarkColorScheme(context) else dynamicLightColorScheme(context)
        dark -> HorizonDark
        else -> HorizonLight
    }
    MaterialExpressiveTheme(
        colorScheme = colors,
        motionScheme = MotionScheme.expressive(),
        typography = CompactTypography,
        content = content,
    )
}
