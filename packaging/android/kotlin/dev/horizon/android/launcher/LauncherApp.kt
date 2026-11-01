package dev.horizon.android.launcher

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
import androidx.compose.animation.AnimatedVisibility
import androidx.compose.animation.fadeIn
import androidx.compose.animation.fadeOut
import androidx.compose.animation.togetherWith
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AutoAwesome
import androidx.compose.material.icons.rounded.Extension
import androidx.compose.material.icons.rounded.Inventory2
import androidx.compose.material.icons.rounded.Palette
import androidx.compose.material.icons.rounded.Person
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Settings
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalIconButton
import androidx.compose.animation.Crossfade
import androidx.compose.material.icons.rounded.Close
import androidx.compose.material3.Icon
import androidx.compose.material3.LocalContentColor
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.NavigationBar
import androidx.compose.material3.NavigationBarItem
import androidx.compose.material3.NavigationRail
import androidx.compose.material3.NavigationRailItem
import androidx.compose.material3.Scaffold
import androidx.compose.material3.SnackbarHost
import androidx.compose.material3.SnackbarHostState
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.foundation.Canvas
import androidx.compose.ui.Modifier
import androidx.compose.ui.geometry.Offset
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.horizon.android.NativeBridge
import androidx.lifecycle.Lifecycle
import androidx.lifecycle.compose.LocalLifecycleOwner
import androidx.lifecycle.repeatOnLifecycle
import kotlinx.coroutines.delay

enum class Destination(val id: String, val label: String, val icon: ImageVector, val inNavigation: Boolean = true) {
    Play("play", "Play", Icons.Rounded.PlayArrow),
    Mods("mods", "Mods", Icons.Rounded.Extension),
    Packs("packs", "Packs", Icons.Rounded.Palette),
    Shaders("shaders", "Shaders", Icons.Rounded.AutoAwesome),
    Installed("installed", "Installed", Icons.Rounded.Inventory2),
    // Opened from the gear beside the account chip on every page, not from the navigation.
    Settings("settings", "Settings", Icons.Rounded.Settings, inNavigation = false),
}

/** Runs a launcher action; failures also reach the status line through the bridge. */
fun act(method: String, arg: String = ""): Reply = Reply.parse(NativeBridge.call(method, arg))

/** The open dialog or sheet, if any. */
sealed interface Overlay {
    data object Account : Overlay
    data object Profiles : Overlay
    data class Choose(val id: String, val options: Options) : Overlay
    data class Edit(val id: String, val title: String, val value: String, val hint: String) : Overlay
    data class Remove(val item: LocalItem) : Overlay
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun LauncherApp(onPlay: () -> Unit, onImport: (String, String?) -> Unit) {
    var snapshot by remember { mutableStateOf<Snapshot?>(null) }
    // Poll only while the launcher is on screen: never while the game runs.
    val lifecycle = LocalLifecycleOwner.current.lifecycle
    LaunchedEffect(lifecycle) {
        lifecycle.repeatOnLifecycle(Lifecycle.State.STARTED) {
            while (true) {
                NativeBridge.snapshot()?.let { text -> Snapshot.parse(text)?.let { snapshot = it } }
                delay(200)
            }
        }
    }
    var destination by rememberSaveable { mutableStateOf(Destination.Play) }
    // Settings is a layer over the current page, not a destination: closing it reveals that page as it was.
    var settingsOpen by rememberSaveable { mutableStateOf(false) }
    LaunchedEffect(destination, settingsOpen) {
        act("page", if (settingsOpen) Destination.Settings.id else destination.id)
    }
    var overlay by remember { mutableStateOf<Overlay?>(null) }
    var importKind by rememberSaveable { mutableStateOf("mods") }
    val picker = rememberLauncherForActivityResult(ActivityResultContracts.OpenDocument()) { uri ->
        onImport(importKind, uri?.toString())
    }
    val snackbars = remember { SnackbarHostState() }
    val state = snapshot
    LaunchedEffect(state?.status) {
        val status = state?.status.orEmpty()
        if (status.isNotBlank()) {
            snackbars.showSnackbar(status, withDismissAction = true)
            act("dismiss_status")
        }
    }
    BackHandler(enabled = settingsOpen || destination != Destination.Play) {
        if (settingsOpen) settingsOpen = false else destination = Destination.Play
    }

    val actions = LauncherActions(
        play = {
            val reply = act("play")
            if ((reply.ok as? org.json.JSONObject)?.optBoolean("launch") == true) onPlay()
        },
        openAccount = { overlay = Overlay.Account },
        choose = { id ->
            if (id == "profile") {
                overlay = Overlay.Profiles
            } else {
                act("options", id).options()?.let { overlay = Overlay.Choose(id, it) }
            }
        },
        edit = { id, title, value, hint -> overlay = Overlay.Edit(id, title, value, hint) },
        import = { kind ->
            importKind = kind
            picker.launch(arrayOf("*/*"))
        },
        remove = { overlay = Overlay.Remove(it) },
        toggleSettings = { settingsOpen = !settingsOpen },
    )

    LauncherScaffold(state, destination, { destination = it }, actions, snackbars, settingsOpen)

    if (state != null) {
        when (val o = overlay) {
            Overlay.Account -> AccountSheet(state, actions, onDismiss = { overlay = null })
            is Overlay.Choose -> ChooseDialog(o, state.config.renderScale, onDismiss = { overlay = null })
            Overlay.Profiles -> ProfilesSheet(onDismiss = { overlay = null })
            is Overlay.Edit -> EditDialog(o, onDismiss = { overlay = null })
            is Overlay.Remove -> RemoveDialog(o.item, onDismiss = { overlay = null })
            null -> {}
        }
        ImportDialogs(state.documents)
    }
}

/** Navigation and the current page; [LauncherApp] adds the bridge, dialogs and pickers. */
@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun LauncherScaffold(
    state: Snapshot?,
    destination: Destination,
    onDestination: (Destination) -> Unit,
    actions: LauncherActions,
    snackbars: SnackbarHostState,
    settingsOpen: Boolean = false,
) {
    // Status snackbars sit over the page, never over the navigation bar or rail.
    val snackbarHost = @Composable { SnackbarHost(snackbars, Modifier.padding(bottom = 4.dp)) }
    Scaffold(
        contentWindowInsets = WindowInsets(0),
    ) { padding ->
        BoxWithConstraints(Modifier.fillMaxSize().padding(padding)) {
            val wide = maxWidth >= 600.dp
            // Phones held sideways are ~400 dp tall: tighter header, no rail logo.
            val short = maxHeight < 520.dp
            if (state == null) {
                Box(Modifier.fillMaxSize(), contentAlignment = Alignment.Center) { LoadingIndicator() }
                return@BoxWithConstraints
            }
            if (wide) {
                Row(Modifier.fillMaxSize()) {
                    NavigationRail(
                        modifier = Modifier.windowInsetsPadding(WindowInsets.safeDrawing),
                    ) {
                        Spacer(Modifier.weight(1f))
                        Destination.entries.filter { it.inNavigation }.forEach { d ->
                            NavigationRailItem(
                                selected = destination == d,
                                onClick = { onDestination(d) },
                                icon = { Icon(d.icon, contentDescription = null) },
                                label = { Text(d.label, maxLines = 1, softWrap = false) },
                            )
                        }
                        Spacer(Modifier.weight(1f))
                    }
                    Box(Modifier.weight(1f).windowInsetsPadding(WindowInsets.safeDrawing)) {
                        Page(destination, state, wide = true, short, actions, Modifier.fillMaxSize())
                        if (!settingsOpen) Box(Modifier.align(Alignment.BottomCenter)) { snackbarHost() }
                    }
                }
            } else {
                Column(Modifier.fillMaxSize()) {
                    Box(
                        Modifier.weight(1f).windowInsetsPadding(
                            WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal),
                        ),
                    ) {
                        Page(destination, state, wide = false, short, actions, Modifier.fillMaxSize())
                        if (!settingsOpen) Box(Modifier.align(Alignment.BottomCenter)) { snackbarHost() }
                    }
                    NavigationBar {
                        Destination.entries.filter { it.inNavigation }.forEach { d ->
                            NavigationBarItem(
                                selected = destination == d,
                                onClick = { onDestination(d) },
                                icon = { Icon(d.icon, contentDescription = null) },
                                label = { Text(d.label, maxLines = 1, softWrap = false) },
                            )
                        }
                    }
                }
            }
            // Settings covers everything, navigation included, and simply fades in and out.
            AnimatedVisibility(settingsOpen, enter = fadeIn(), exit = fadeOut()) {
                Surface(Modifier.fillMaxSize(), color = MaterialTheme.colorScheme.surface) {
                    Box(Modifier.fillMaxSize().windowInsetsPadding(WindowInsets.safeDrawing)) {
                        Column(Modifier.fillMaxSize().padding(horizontal = if (wide) 24.dp else 16.dp)) {
                            Header(Destination.Settings, state, short, actions)
                            Box(Modifier.weight(1f).fillMaxWidth()) { SettingsScreen(state, wide, actions) }
                        }
                        Box(Modifier.align(Alignment.BottomCenter)) { snackbarHost() }
                    }
                }
            }
        }
    }

}

/** What the pages ask the launcher shell to do. */
class LauncherActions(
    val play: () -> Unit,
    val openAccount: () -> Unit,
    val choose: (String) -> Unit,
    val edit: (id: String, title: String, value: String, hint: String) -> Unit,
    val import: (String) -> Unit,
    val remove: (LocalItem) -> Unit,
    val toggleSettings: () -> Unit,
)

/**
 * Horizon's mark (docs/media/brand/mark.svg): an H whose crossbar is the horizon, on a 16-unit grid.
 * Stems take the content colour, their reflection a third of it, the horizon the primary colour.
 */
@Composable
fun HorizonMark(modifier: Modifier = Modifier) {
    val stem = MaterialTheme.colorScheme.onSurface
    val line = MaterialTheme.colorScheme.primary
    Canvas(modifier.semantics { contentDescription = "Horizon" }) {
        val u = size.minDimension / 16f
        fun rect(x: Int, y: Int, w: Int, h: Int, color: Color) =
            drawRect(color, Offset(x * u, y * u), Size(w * u, h * u))
        for (x in intArrayOf(4, 10)) {
            rect(x, 3, 2, 4, stem)
            rect(x, 8, 2, 4, stem.copy(alpha = 0.36f))
        }
        rect(1, 7, 14, 1, line)
    }
}

@Composable
private fun Page(
    destination: Destination,
    state: Snapshot,
    wide: Boolean,
    short: Boolean,
    actions: LauncherActions,
    modifier: Modifier,
) {
    AnimatedContent(
        targetState = destination,
        transitionSpec = { fadeIn() togetherWith fadeOut() },
        modifier = modifier,
        label = "page",
    ) { page ->
        Column(Modifier.fillMaxSize().padding(horizontal = if (wide) 24.dp else 16.dp)) {
            Header(page, state, short, actions)
            Box(Modifier.weight(1f).fillMaxWidth()) {
                when (page) {
                    Destination.Play -> PlayScreen(state, wide, short, actions)
                    Destination.Mods, Destination.Packs, Destination.Shaders ->
                        CatalogScreen(page, state, wide, actions)
                    Destination.Installed -> InstalledScreen(state, actions)
                    Destination.Settings -> {} // drawn as a layer by LauncherScaffold
                }
            }
        }
    }
}

@Composable
private fun Header(page: Destination, state: Snapshot, short: Boolean, actions: LauncherActions) {
    Row(
        Modifier.fillMaxWidth().padding(top = if (short) 6.dp else 12.dp, bottom = if (short) 6.dp else 8.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(8.dp),
    ) {
        if (page == Destination.Play) HorizonMark(Modifier.size(if (short) 28.dp else 32.dp))
        Text(
            when (page) {
                Destination.Play -> "Horizon Android"
                Destination.Mods -> "Horizon Mods"
                Destination.Packs -> "Resource Packs"
                Destination.Shaders -> "Shader Packs"
                Destination.Installed -> "Installed"
                Destination.Settings -> "Settings"
            },
            style = if (short) MaterialTheme.typography.titleLarge else MaterialTheme.typography.headlineSmall,
            fontWeight = FontWeight.Bold,
            modifier = Modifier.weight(1f),
            maxLines = 1,
            overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
        )
        if (page != Destination.Settings) AccountChip(state.account, actions.openAccount)
        val open = page == Destination.Settings
        FilledTonalIconButton(onClick = actions.toggleSettings) {
            Crossfade(open, label = "settings-icon") { closing ->
                Icon(
                    if (closing) Icons.Rounded.Close else Icons.Rounded.Settings,
                    contentDescription = if (closing) "Close settings" else "Settings",
                )
            }
        }
    }
}

/** [inChip]: drawn on the account chip, taking its tonal colour like the settings button beside it. */
@Composable
fun Avatar(account: Account, size: androidx.compose.ui.unit.Dp, inChip: Boolean = false) {
    val letter = account.profile?.firstOrNull()?.uppercaseChar()
    Surface(
        modifier = Modifier.size(size),
        shape = CircleShape,
        color = if (inChip) Color.Transparent else MaterialTheme.colorScheme.primary,
        contentColor = if (inChip) LocalContentColor.current else MaterialTheme.colorScheme.onPrimary,
    ) {
        Box(contentAlignment = Alignment.Center) {
            if (letter != null) {
                Text(
                    letter.toString(),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = LocalContentColor.current,
                )
            } else {
                Icon(
                    Icons.Rounded.Person,
                    contentDescription = null,
                    modifier = Modifier.size(size * 0.6f),
                    tint = LocalContentColor.current,
                )
            }
        }
    }
}

@Composable
private fun AccountChip(account: Account, onClick: () -> Unit) {
    val named = account.profile != null || account.signedIn
    Surface(
        onClick = onClick,
        shape = CircleShape,
        color = MaterialTheme.colorScheme.secondaryContainer,
        contentColor = MaterialTheme.colorScheme.onSecondaryContainer,
    ) {
        Row(
            Modifier.padding(start = if (named) 4.dp else 0.dp, end = if (named) 12.dp else 0.dp, top = if (named) 4.dp else 0.dp, bottom = if (named) 4.dp else 0.dp).height(if (named) 32.dp else 40.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Avatar(account, if (named) 32.dp else 40.dp, inChip = true)
            val label = account.profile ?: account.status.takeIf { account.signedIn }
            if (label != null) {
                Text(
                    label,
                    style = MaterialTheme.typography.labelLarge,
                    maxLines = 1,
                    modifier = Modifier.widthIn(max = 140.dp),
                    overflow = androidx.compose.ui.text.style.TextOverflow.Ellipsis,
                )
            }
        }
    }
}
