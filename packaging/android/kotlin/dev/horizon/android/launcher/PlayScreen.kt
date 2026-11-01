package dev.horizon.android.launcher

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.CheckCircle
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material.icons.rounded.Download
import androidx.compose.material.icons.rounded.ErrorOutline
import androidx.compose.material.icons.rounded.Layers
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.RadioButtonUnchecked
import androidx.compose.material.icons.rounded.Refresh
import androidx.compose.material.icons.rounded.SportsEsports
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

@Composable
fun PlayScreen(state: Snapshot, wide: Boolean, short: Boolean, actions: LauncherActions) {
    if (wide) {
        Row(
            Modifier.fillMaxSize().padding(bottom = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Hero(state, short, Modifier.weight(1f).fillMaxHeight(), scroll = true)
            Column(
                Modifier.width(340.dp).fillMaxHeight(),
                verticalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                AccountCard(state.account, state.config, actions.openAccount)
                VersionCard(state, onClick = { actions.choose("profile") })
                Spacer(Modifier.weight(1f))
                LaunchButton(state, actions.play, Modifier.fillMaxWidth().height(if (short) 88.dp else 104.dp))
            }
        }
    } else {
        Column(
            Modifier.fillMaxSize().verticalScroll(rememberScrollState()).padding(bottom = 16.dp),
            verticalArrangement = Arrangement.spacedBy(12.dp),
        ) {
            Hero(state, short, Modifier.fillMaxWidth(), scroll = false)
            AccountCard(state.account, state.config, actions.openAccount)
            VersionCard(state, onClick = { actions.choose("profile") })
            LaunchButton(state, actions.play, Modifier.fillMaxWidth().height(88.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Hero(state: Snapshot, short: Boolean, modifier: Modifier, scroll: Boolean) {
    val install = state.install
    Card(
        modifier = modifier,
        shape = RoundedCornerShape(32.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
    ) {
        Column(
            (if (scroll) Modifier.verticalScroll(rememberScrollState()) else Modifier).padding(if (short) 24.dp else 28.dp).fillMaxWidth(),
            verticalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            Row(verticalAlignment = Alignment.Top) {
                Column(Modifier.weight(1f), verticalArrangement = Arrangement.spacedBy(6.dp)) {
                    Text(
                        "HORIZON ANDROID ${state.version}",
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    Text(
                        when {
                            install.installing -> "Getting your game ready"
                            install.installed -> "Ready to play"
                            else -> "Your Java worlds, on Android"
                        },
                        style = if (short) MaterialTheme.typography.headlineLarge else MaterialTheme.typography.displaySmall,
                        fontWeight = FontWeight.Bold,
                    )
                    Text(
                        "Minecraft 26.3  ·  ${state.config.renderer}",
                        style = MaterialTheme.typography.bodyLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Surface(
                    modifier = Modifier.padding(start = 16.dp).size(if (short) 88.dp else 112.dp),
                    shape = MaterialShapes.Cookie12Sided.toShape(),
                    color = MaterialTheme.colorScheme.primaryContainer,
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(
                            when {
                                install.installing -> Icons.Rounded.Download
                                install.installed -> Icons.Rounded.SportsEsports
                                else -> Icons.Rounded.Download
                            },
                            contentDescription = null,
                            modifier = Modifier.size(if (short) 40.dp else 52.dp),
                            tint = MaterialTheme.colorScheme.onPrimaryContainer,
                        )
                    }
                }
            }
            Spacer(Modifier.height(if (short) 4.dp else 12.dp))
            state.error?.let { ErrorCard(it) }
            if (install.installing) {
                install.stages.forEachIndexed { i, stage ->
                    StageRow(stage, done = i < install.stageIndex, current = i == install.stageIndex)
                }
                Spacer(Modifier.height(8.dp))
                LinearWavyProgressIndicator(
                    progress = { install.overall.coerceIn(0f, 1f) },
                    modifier = Modifier.fillMaxWidth(),
                )
                Text(
                    install.detail,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            } else {
                val steps = if (install.installed) {
                    listOf(
                        "Open a world or join a Java server",
                        "Set up touch in Options › Controls",
                        "Find packs, shaders and .hmod in the tabs",
                    )
                } else {
                    listOf(
                        "Download resources from Mojang",
                        "Install and verify on this phone",
                        "Play offline or sign in for servers",
                    )
                }
                steps.forEachIndexed { i, step -> NumberedStep(i + 1, step) }
            }
        }
    }
}

@Composable
private fun ErrorCard(message: String) {
    Surface(
        shape = RoundedCornerShape(20.dp),
        color = MaterialTheme.colorScheme.errorContainer,
        modifier = Modifier.fillMaxWidth().padding(bottom = 8.dp),
    ) {
        Row(Modifier.padding(start = 16.dp, top = 8.dp, bottom = 8.dp), verticalAlignment = Alignment.CenterVertically) {
            Icon(Icons.Rounded.ErrorOutline, null, tint = MaterialTheme.colorScheme.onErrorContainer)
            Text(
                message,
                modifier = Modifier.weight(1f).padding(horizontal = 12.dp),
                style = MaterialTheme.typography.bodyMedium,
                color = MaterialTheme.colorScheme.onErrorContainer,
                maxLines = 4,
                overflow = TextOverflow.Ellipsis,
            )
            IconButton(onClick = { act("dismiss_error") }) {
                Icon(Icons.Rounded.Close, "Dismiss", tint = MaterialTheme.colorScheme.onErrorContainer)
            }
        }
    }
}

@Composable
private fun StageRow(label: String, done: Boolean, current: Boolean) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Icon(
            if (done) Icons.Rounded.CheckCircle else Icons.Rounded.RadioButtonUnchecked,
            contentDescription = null,
            tint = if (done || current) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.outline,
            modifier = Modifier.size(22.dp),
        )
        Text(
            label,
            style = MaterialTheme.typography.bodyLarge,
            fontWeight = if (current) FontWeight.SemiBold else FontWeight.Normal,
            color = if (current) MaterialTheme.colorScheme.onSurface else MaterialTheme.colorScheme.onSurfaceVariant,
        )
    }
}

@Composable
private fun NumberedStep(number: Int, text: String) {
    Row(verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(14.dp)) {
        Surface(shape = CircleShape, color = MaterialTheme.colorScheme.secondaryContainer, modifier = Modifier.size(36.dp)) {
            Box(contentAlignment = Alignment.Center) {
                Text(
                    number.toString(),
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.onSecondaryContainer,
                )
            }
        }
        Text(text, style = MaterialTheme.typography.bodyLarge)
    }
}

@Composable
private fun AccountCard(account: Account, config: Config, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(28.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Avatar(account, 56.dp)
            Column(Modifier.padding(start = 16.dp).weight(1f)) {
                Text(
                    account.profile ?: config.name.ifBlank { "Player" },
                    style = MaterialTheme.typography.titleLarge,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    account.status,
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun VersionCard(state: Snapshot, onClick: () -> Unit) {
    Card(
        onClick = onClick,
        shape = RoundedCornerShape(28.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainerHigh),
        modifier = Modifier.fillMaxWidth(),
    ) {
        Row(Modifier.padding(16.dp), verticalAlignment = Alignment.CenterVertically) {
            Surface(
                modifier = Modifier.size(56.dp),
                shape = MaterialShapes.Pill.toShape(),
                color = MaterialTheme.colorScheme.tertiaryContainer,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    Icon(Icons.Rounded.Layers, null, tint = MaterialTheme.colorScheme.onTertiaryContainer)
                }
            }
            Column(Modifier.padding(start = 16.dp).weight(1f)) {
                Text("Minecraft 26.3", style = MaterialTheme.typography.titleLarge, fontWeight = FontWeight.SemiBold)
                Text(
                    "Profile ${state.config.profile}  ·  ${state.config.renderScale}%  ·  ${state.config.input}",
                    style = MaterialTheme.typography.bodyMedium,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LaunchButton(state: Snapshot, onPlay: () -> Unit, modifier: Modifier) {
    val install = state.install
    if (install.installing) {
        Surface(modifier = modifier, shape = RoundedCornerShape(32.dp), color = MaterialTheme.colorScheme.surfaceContainerHigh) {
            Row(Modifier.padding(horizontal = 24.dp), verticalAlignment = Alignment.CenterVertically) {
                LoadingIndicator(Modifier.size(48.dp))
                Column(Modifier.padding(start = 16.dp).weight(1f)) {
                    Text(install.stage, style = MaterialTheme.typography.titleMedium, maxLines = 1)
                    Text(
                        "${(install.overall * 100).toInt()}%",
                        style = MaterialTheme.typography.titleLarge,
                        color = MaterialTheme.colorScheme.primary,
                        fontWeight = FontWeight.Bold,
                    )
                }
            }
        }
        return
    }
    Button(
        onClick = onPlay,
        modifier = modifier,
        shapes = ButtonDefaults.shapes(),
        contentPadding = PaddingValues(horizontal = 24.dp),
    ) {
        Icon(
            when {
                install.installed -> Icons.Rounded.PlayArrow
                state.error != null -> Icons.Rounded.Refresh
                else -> Icons.Rounded.Download
            },
            contentDescription = null,
            modifier = Modifier.size(36.dp),
        )
        Spacer(Modifier.width(12.dp))
        Text(
            when {
                install.installed -> "Play"
                state.error != null -> "Retry installation"
                else -> "Download Minecraft 26.3"
            },
            style = if (install.installed) MaterialTheme.typography.headlineMedium else MaterialTheme.typography.titleLarge,
            fontWeight = FontWeight.Bold,
            maxLines = 1,
        )
    }
}
