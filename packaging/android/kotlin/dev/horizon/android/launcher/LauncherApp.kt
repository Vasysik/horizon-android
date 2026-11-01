package dev.horizon.android.launcher

import androidx.activity.compose.BackHandler
import androidx.activity.compose.rememberLauncherForActivityResult
import androidx.activity.result.contract.ActivityResultContracts
import androidx.compose.animation.AnimatedContent
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
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.windowInsetsPadding
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.AutoAwesome
import androidx.compose.material.icons.rounded.Extension
import androidx.compose.material.icons.rounded.Inventory2
import androidx.compose.material.icons.rounded.Palette
import androidx.compose.material.icons.rounded.Person
import androidx.compose.material.icons.rounded.PlayArrow
import androidx.compose.material.icons.rounded.Tune
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
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
import androidx.compose.ui.Modifier
import androidx.compose.ui.draw.clip
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.unit.dp
import dev.horizon.android.NativeBridge
import kotlinx.coroutines.delay

enum class Destination(val id: String, val label: String, val icon: ImageVector) {
    Play("play", "Play", Icons.Rounded.PlayArrow),
    Mods("mods", "Mods", Icons.Rounded.Extension),
    Packs("packs", "Packs", Icons.Rounded.Palette),
    Shaders("shaders", "Shaders", Icons.Rounded.AutoAwesome),
    Installed("installed", "Installed", Icons.Rounded.Inventory2),
    Settings("settings", "Settings", Icons.Rounded.Tune),
}

/** Runs a launcher action; failures also reach the status line through the bridge. */
fun act(method: String, arg: String = ""): Reply = Reply.parse(NativeBridge.call(method, arg))

/** The open dialog or sheet, if any. */
sealed interface Overlay {
    data object Account : Overlay
    data class Choose(val id: String, val options: Options) : Overlay
    data class Edit(val id: String, val title: String, val value: String, val hint: String) : Overlay
    data class Remove(val item: LocalItem) : Overlay
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun LauncherApp(onPlay: () -> Unit, onImport: (String, String?) -> Unit) {
    var snapshot by remember { mutableStateOf<Snapshot?>(null) }
    LaunchedEffect(Unit) {
        while (true) {
            NativeBridge.snapshot()?.let { text -> Snapshot.parse(text)?.let { snapshot = it } }
            delay(200)
        }
    }
    var destination by rememberSaveable { mutableStateOf(Destination.Play) }
    LaunchedEffect(destination) { act("page", destination.id) }
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
    BackHandler(enabled = destination != Destination.Play) { destination = Destination.Play }

    val actions = LauncherActions(
        play = {
            val reply = act("play")
            if ((reply.ok as? org.json.JSONObject)?.optBoolean("launch") == true) onPlay()
        },
        openAccount = { overlay = Overlay.Account },
        choose = { id ->
            act("options", id).options()?.let { overlay = Overlay.Choose(id, it) }
        },
        edit = { id, title, value, hint -> overlay = Overlay.Edit(id, title, value, hint) },
        import = { kind ->
            importKind = kind
            picker.launch(arrayOf("*/*"))
        },
        remove = { overlay = Overlay.Remove(it) },
    )

    LauncherScaffold(state, destination, { destination = it }, actions, snackbars)

    if (state != null) {
        when (val o = overlay) {
            Overlay.Account -> AccountSheet(state, actions, onDismiss = { overlay = null })
            is Overlay.Choose -> ChooseDialog(o, onDismiss = { overlay = null }, onCustomScale = {
                overlay = Overlay.Edit("render_scale", "Client resolution", state.config.renderScale.toString(), "25 to 100 percent")
            })
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
) {
    Scaffold(
        snackbarHost = { SnackbarHost(snackbars) },
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
                        header = if (short) null else ({ Brand() }),
                    ) {
                        Spacer(Modifier.weight(1f))
                        Destination.entries.forEach { d ->
                            NavigationRailItem(
                                selected = destination == d,
                                onClick = { onDestination(d) },
                                icon = { Icon(d.icon, contentDescription = null) },
                                label = { Text(d.label) },
                            )
                        }
                        Spacer(Modifier.weight(1f))
                    }
                    Page(
                        destination,
                        state,
                        wide = true,
                        short,
                        actions,
                        Modifier.weight(1f).windowInsetsPadding(WindowInsets.safeDrawing),
                    )
                }
            } else {
                Column(Modifier.fillMaxSize()) {
                    Page(
                        destination,
                        state,
                        wide = false,
                        short,
                        actions,
                        Modifier.weight(1f).windowInsetsPadding(WindowInsets.safeDrawing),
                    )
                    NavigationBar {
                        Destination.entries.forEach { d ->
                            NavigationBarItem(
                                selected = destination == d,
                                onClick = { onDestination(d) },
                                icon = { Icon(d.icon, contentDescription = null) },
                                label = { Text(d.label, maxLines = 1) },
                            )
                        }
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
)

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun Brand() {
    Surface(
        modifier = Modifier.padding(top = 12.dp, bottom = 8.dp).size(56.dp),
        shape = MaterialShapes.Cookie9Sided.toShape(),
        color = MaterialTheme.colorScheme.primaryContainer,
    ) {
        Box(contentAlignment = Alignment.Center) {
            Text(
                "H",
                style = MaterialTheme.typography.titleLarge,
                fontWeight = FontWeight.Black,
                color = MaterialTheme.colorScheme.onPrimaryContainer,
            )
        }
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
                    Destination.Settings -> SettingsScreen(state, wide, actions)
                }
            }
        }
    }
}

@Composable
private fun Header(page: Destination, state: Snapshot, short: Boolean, actions: LauncherActions) {
    Row(
        Modifier.fillMaxWidth().padding(top = if (short) 8.dp else 16.dp, bottom = if (short) 8.dp else 12.dp),
        verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(12.dp),
    ) {
        Text(
            when (page) {
                Destination.Play -> "Horizon"
                Destination.Mods -> "Horizon mods"
                Destination.Packs -> "Resource packs"
                Destination.Shaders -> "Shader packs"
                Destination.Installed -> "Installed"
                Destination.Settings -> "Settings"
            },
            style = if (short) MaterialTheme.typography.headlineSmall else MaterialTheme.typography.headlineMedium,
            fontWeight = FontWeight.SemiBold,
            modifier = Modifier.weight(1f),
            maxLines = 1,
        )
        AccountChip(state.account, actions.openAccount)
    }
}

@Composable
fun Avatar(account: Account, size: androidx.compose.ui.unit.Dp) {
    val letter = account.profile?.firstOrNull()?.uppercaseChar()
    Surface(
        modifier = Modifier.size(size),
        shape = CircleShape,
        color = if (account.signedIn) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.secondaryContainer,
    ) {
        Box(contentAlignment = Alignment.Center) {
            if (letter != null) {
                Text(
                    letter.toString(),
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.Bold,
                    color = if (account.signedIn) MaterialTheme.colorScheme.onPrimary else MaterialTheme.colorScheme.onSecondaryContainer,
                )
            } else {
                Icon(
                    Icons.Rounded.Person,
                    contentDescription = null,
                    modifier = Modifier.size(size * 0.6f),
                    tint = MaterialTheme.colorScheme.onSecondaryContainer,
                )
            }
        }
    }
}

@Composable
private fun AccountChip(account: Account, onClick: () -> Unit) {
    Surface(
        onClick = onClick,
        shape = CircleShape,
        color = MaterialTheme.colorScheme.surfaceContainerHigh,
    ) {
        Row(
            Modifier.padding(start = 6.dp, end = 16.dp, top = 6.dp, bottom = 6.dp).height(36.dp),
            verticalAlignment = Alignment.CenterVertically,
            horizontalArrangement = Arrangement.spacedBy(10.dp),
        ) {
            Avatar(account, 36.dp)
            Text(
                account.profile ?: if (account.signedIn) account.status else "Offline",
                style = MaterialTheme.typography.labelLarge,
                maxLines = 1,
            )
        }
    }
}
