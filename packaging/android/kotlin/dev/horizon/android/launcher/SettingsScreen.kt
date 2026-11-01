package dev.horizon.android.launcher

import androidx.compose.foundation.layout.IntrinsicSize
import androidx.compose.foundation.layout.fillMaxHeight
import androidx.compose.foundation.lazy.staggeredgrid.LazyVerticalStaggeredGrid
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridCells
import androidx.compose.foundation.lazy.staggeredgrid.StaggeredGridItemSpan
import androidx.compose.foundation.lazy.staggeredgrid.items
import androidx.compose.material3.VerticalDivider
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.material3.Slider
import androidx.compose.runtime.mutableFloatStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import kotlin.math.roundToInt
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.width
import androidx.compose.material3.ButtonDefaults
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectable
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.KeyboardArrowRight
import androidx.compose.material.icons.automirrored.rounded.Login
import androidx.compose.material.icons.automirrored.rounded.Logout
import androidx.compose.material.icons.rounded.Archive
import androidx.compose.material.icons.rounded.AspectRatio
import androidx.compose.material.icons.rounded.Badge
import androidx.compose.material.icons.rounded.BugReport
import androidx.compose.material.icons.rounded.ContentCopy
import androidx.compose.material.icons.rounded.Dns
import androidx.compose.material.icons.rounded.Fullscreen
import androidx.compose.material.icons.rounded.Gamepad
import androidx.compose.material.icons.rounded.HighQuality
import androidx.compose.material.icons.rounded.History
import androidx.compose.material.icons.rounded.Insights
import androidx.compose.material.icons.rounded.Memory
import androidx.compose.material.icons.rounded.Monitor
import androidx.compose.material.icons.rounded.OpenInBrowser
import androidx.compose.material.icons.rounded.Person
import androidx.compose.material.icons.rounded.ScreenRotation
import androidx.compose.material.icons.rounded.Smartphone
import androidx.compose.material.icons.rounded.Speed
import androidx.compose.material.icons.rounded.Timer
import androidx.compose.material.icons.rounded.UnfoldMore
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.LinearWavyProgressIndicator
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.RadioButton
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.vector.ImageVector
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.unit.dp

private fun settingIcon(id: String): ImageVector = when (id) {
    "renderer" -> Icons.Rounded.Monitor
    "render_scale" -> Icons.Rounded.HighQuality
    "display_hz" -> Icons.Rounded.Speed
    "performance_hints" -> Icons.Rounded.Memory
    "orientation" -> Icons.Rounded.ScreenRotation
    "aspect" -> Icons.Rounded.AspectRatio
    "immersive" -> Icons.Rounded.Fullscreen
    "cutout" -> Icons.Rounded.Smartphone
    "profile" -> Icons.Rounded.Person
    "input_host" -> Icons.Rounded.Gamepad
    "name" -> Icons.Rounded.Badge
    "server" -> Icons.Rounded.Dns
    "gpu_profile" -> Icons.Rounded.BugReport
    "record_performance" -> Icons.Rounded.Timer
    "export_diagnostics" -> Icons.Rounded.Archive
    "export_performance" -> Icons.Rounded.Insights
    else -> Icons.Rounded.History
}

@Composable
fun SettingsScreen(state: Snapshot, wide: Boolean, actions: LauncherActions) {
    // Each group is one block; sideways the blocks sit in two columns instead of one long list.
    LazyVerticalStaggeredGrid(
        columns = StaggeredGridCells.Fixed(if (wide) 2 else 1),
        modifier = Modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 24.dp),
        horizontalArrangement = Arrangement.spacedBy(16.dp),
        verticalItemSpacing = 4.dp,
    ) {
        items(state.settings, key = { "group-${it.name}" }) { group -> SettingGroup(group, state, actions) }
        item(key = "source", span = StaggeredGridItemSpan.FullLine) {
            FilledTonalButton(
                onClick = { act("repository") },
                modifier = Modifier.fillMaxWidth().padding(top = 16.dp).height(48.dp),
            ) {
                Icon(Icons.Rounded.OpenInBrowser, null, Modifier.size(18.dp))
                Spacer(Modifier.width(8.dp))
                Text("Source code")
            }
        }
    }
}

@Composable
private fun SettingGroup(group: SettingGroup, state: Snapshot, actions: LauncherActions) {
    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
        Text(
            group.name,
            style = MaterialTheme.typography.titleSmall,
            fontWeight = FontWeight.Bold,
            color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(start = 16.dp, top = 12.dp, bottom = 6.dp),
        )
        group.rows.forEachIndexed { i, row ->
            // A connected group: large corners at its ends, small ones between rows.
            Surface(
                shape = connectedShape(i, group.rows.size, outer = 24),
                color = MaterialTheme.colorScheme.surfaceContainer,
                onClick = {
                    when (row.kind) {
                        "switch" -> act("toggle", row.id)
                        "choice" -> actions.choose(row.id)
                        "text" -> actions.edit(
                            row.id,
                            row.title,
                            if (row.id == "name") state.config.name else state.config.server,
                            if (row.id == "server") "Leave empty for singleplayer" else "Up to 16 characters",
                        )
                        else -> act(row.id)
                    }
                },
            ) {
                ListItem(
                    colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                    leadingContent = {
                        Surface(
                            shape = RoundedCornerShape(14.dp),
                            color = MaterialTheme.colorScheme.secondaryContainer,
                            modifier = Modifier.size(40.dp),
                        ) {
                            Box(contentAlignment = Alignment.Center) {
                                Icon(settingIcon(row.id), null, tint = MaterialTheme.colorScheme.onSecondaryContainer)
                            }
                        }
                    },
                    headlineContent = { Text(row.title, fontWeight = FontWeight.Medium, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    supportingContent = { Text(row.value, maxLines = 2, overflow = TextOverflow.Ellipsis) },
                    trailingContent = {
                        when (row.kind) {
                            "switch" -> HorizonSwitch(checked = row.on, onCheckedChange = { act("toggle", row.id) })
                            "choice" -> Icon(Icons.Rounded.UnfoldMore, null)
                            else -> Icon(Icons.AutoMirrored.Rounded.KeyboardArrowRight, null)
                        }
                    },
                )
            }
        }
    }
}

/** Numeric settings: a slider reads better than a list of presets. */
private val SLIDER_ROWS = setOf("render_scale", "display_hz")

/**
 * Numeric settings open a slider instead of a preset list. Render scale is continuous (5% steps);
 * refresh rates stop at the launcher's supported values.
 */
@Composable
private fun SliderDialog(choose: Overlay.Choose, renderScale: Int, onDismiss: () -> Unit) {
    val choices = choose.options.choices
    val continuous = choose.id == "render_scale"
    var value by remember {
        mutableFloatStateOf(
            if (continuous) renderScale.toFloat() else choices.indexOfFirst { it.selected }.coerceAtLeast(0).toFloat()
        )
    }
    val label = if (continuous) "${value.roundToInt()}%" else choices.getOrNull(value.roundToInt())?.label.orEmpty()
    val apply = {
        val arg = if (continuous) value.roundToInt().toString() else choices.getOrNull(value.roundToInt())?.value
        if (arg != null) act("choose", "${choose.id}=$arg")
        onDismiss()
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(choose.options.title, fontWeight = FontWeight.Bold, modifier = Modifier.padding(bottom = 4.dp)) },
        text = {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
                if (choose.options.help.isNotBlank()) {
                    Text(
                        choose.options.help,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                }
                Text(
                    label,
                    style = MaterialTheme.typography.headlineMedium,
                    fontWeight = FontWeight.SemiBold,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.align(Alignment.CenterHorizontally),
                )
                if (continuous) {
                    Slider(value = value, onValueChange = { value = it }, valueRange = 25f..100f, steps = 14)
                } else if (choices.size > 1) {
                    Slider(
                        value = value,
                        onValueChange = { value = it },
                        valueRange = 0f..(choices.size - 1).toFloat(),
                        steps = choices.size - 2,
                    )
                }
                Row(Modifier.fillMaxWidth()) {
                    val ends = if (continuous) "25%" to "100%" else choices.first().label to choices.last().label
                    Text(ends.first, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    Spacer(Modifier.weight(1f))
                    Text(ends.second, style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
            }
        },
        confirmButton = { SaveButton(onClick = apply) },
        dismissButton = { DiscardButton(onDismiss) },
    )
}

@Composable
fun ChooseDialog(choose: Overlay.Choose, renderScale: Int, onDismiss: () -> Unit) {
    if (choose.id in SLIDER_ROWS && choose.options.choices.isNotEmpty()) {
        SliderDialog(choose, renderScale, onDismiss)
        return
    }
    val choices = choose.options.choices
    var picked by remember { mutableStateOf(choices.firstOrNull { it.selected }?.value) }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(choose.options.title, fontWeight = FontWeight.Bold, modifier = Modifier.padding(bottom = 4.dp)) },
        text = {
            Column(Modifier.verticalScroll(rememberScrollState())) {
                if (choose.options.help.isNotBlank()) {
                    Text(
                        choose.options.help,
                        style = MaterialTheme.typography.bodyMedium,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                        modifier = Modifier.padding(bottom = 12.dp),
                    )
                }
                if (isLandscape() && choices.size > 2) {
                    // Sideways the dialog is short: tiles pair up so most lists fit without scrolling.
                    Column(verticalArrangement = Arrangement.spacedBy(6.dp)) {
                        choices.chunked(2).forEach { pair ->
                            Row(horizontalArrangement = Arrangement.spacedBy(6.dp), modifier = Modifier.height(IntrinsicSize.Min)) {
                                pair.forEach { c ->
                                    Box(Modifier.weight(1f).fillMaxHeight()) {
                                        ChoiceTile(c.label, c.detail, picked == c.value, RoundedCornerShape(16.dp)) { picked = c.value }
                                    }
                                }
                                if (pair.size == 1) Spacer(Modifier.weight(1f))
                            }
                        }
                    }
                } else {
                    Column(verticalArrangement = Arrangement.spacedBy(2.dp)) {
                        choices.forEachIndexed { i, c ->
                            ChoiceTile(c.label, c.detail, picked == c.value, connectedShape(i, choices.size)) {
                                picked = c.value
                            }
                        }
                    }
                }
            }
        },
        confirmButton = {
            SaveButton(enabled = picked != null && picked != choices.firstOrNull { it.selected }?.value) {
                act("choose", "${choose.id}=$picked")
                onDismiss()
            }
        },
        dismissButton = { DiscardButton(onDismiss) },
    )
}

@Composable
fun EditDialog(edit: Overlay.Edit, onDismiss: () -> Unit) {
    var value by remember { mutableStateOf(edit.value) }
    val save = {
        act("text", "${edit.id}=$value")
        onDismiss()
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(edit.title) },
        text = {
            OutlinedTextField(
                value = value,
                onValueChange = { value = if (edit.id == "name") it.take(16) else it },
                singleLine = true,
                supportingText = { Text(edit.hint) },
                keyboardOptions = KeyboardOptions(
                    keyboardType = if (edit.id == "render_scale") KeyboardType.Number else KeyboardType.Text,
                ),
                shape = RoundedCornerShape(16.dp),
            )
        },
        confirmButton = { SaveButton(onClick = save) },
        dismissButton = { DiscardButton(onDismiss) },
    )
}

@Composable
fun RemoveDialog(item: LocalItem, onDismiss: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text("Remove add-on?") },
        text = { Text("Deletes only ${item.file} from this profile. This cannot be undone.") },
        confirmButton = {
            Button(onClick = {
                act("remove_local", item.index.toString())
                onDismiss()
            }) { Text("Remove") }
        },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ImportDialogs(documents: Documents) {
    val conflict = documents.conflict
    if (conflict != null) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text("File already exists") },
            text = { Text("${conflict.name} is already installed.") },
            confirmButton = {
                Button(onClick = { act("import_conflict", "replace") }) { Text("Replace") }
            },
            dismissButton = {
                Row {
                    if (conflict.keepBoth) {
                        TextButton(onClick = { act("import_conflict", "keep_both") }) { Text("Keep both") }
                    }
                    TextButton(onClick = { act("import_conflict", "cancel") }) { Text("Cancel") }
                }
            },
        )
    } else if (documents.working) {
        AlertDialog(
            onDismissRequest = {},
            title = { Text("Importing") },
            text = {
                Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                    LinearWavyProgressIndicator(Modifier.fillMaxWidth())
                    Text(documents.text ?: "Choose a file")
                }
            },
            confirmButton = { TextButton(onClick = { act("import_cancel") }) { Text("Cancel") } },
        )
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AccountSheet(state: Snapshot, actions: LauncherActions, onDismiss: () -> Unit) {
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        val scroll = Modifier.verticalScroll(rememberScrollState())
        if (isLandscape()) {
            // Sideways: who you are on the left, how to sign in on the right.
            // The whole sheet scrolls; the identity column centres against the sign-in column's height.
            Row(
                Modifier.fillMaxWidth().then(scroll).padding(horizontal = 24.dp).padding(bottom = 24.dp).height(IntrinsicSize.Min),
                horizontalArrangement = Arrangement.spacedBy(20.dp),
            ) {
                Column(
                    Modifier.weight(0.85f).fillMaxHeight(),
                    verticalArrangement = Arrangement.spacedBy(10.dp, Alignment.CenterVertically),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) { AccountIdentity(state, actions) }
                VerticalDivider()
                Column(
                    Modifier.weight(1.15f),
                    verticalArrangement = Arrangement.spacedBy(10.dp),
                    horizontalAlignment = Alignment.CenterHorizontally,
                ) { AccountSignIn(state.account) }
            }
        } else {
            Column(
                Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(bottom = 32.dp).then(scroll),
                verticalArrangement = Arrangement.spacedBy(12.dp),
                horizontalAlignment = Alignment.CenterHorizontally,
            ) {
                AccountIdentity(state, actions)
                HorizontalDivider(Modifier.padding(vertical = 4.dp))
                AccountSignIn(state.account)
            }
        }
    }
}

/** Avatar, name and status, with Log out (or Log in offline) right under them. */
@Composable
private fun AccountIdentity(state: Snapshot, actions: LauncherActions) {
    val account = state.account
    Avatar(account, 64.dp)
    Text(
        account.profile ?: state.config.name.ifBlank { "Player" },
        style = MaterialTheme.typography.headlineSmall,
        fontWeight = FontWeight.Bold,
        maxLines = 1,
        overflow = TextOverflow.Ellipsis,
    )
    Text(
        account.status,
        style = MaterialTheme.typography.bodyLarge,
        textAlign = TextAlign.Center,
        color = if (account.signedIn) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
    )
    // Someone is logged in with a Microsoft account or an offline name; logging out of either
    // turns the button into "Log in offline".
    val microsoft = account.profile != null
    var confirm by remember { mutableStateOf(false) }
    if (confirm) {
        AlertDialog(
            onDismissRequest = { confirm = false },
            icon = { Icon(Icons.AutoMirrored.Rounded.Logout, null) },
            title = { Text("Log out?", fontWeight = FontWeight.Bold) },
            text = {
                Text(
                    if (microsoft) "${account.profile} is removed from this phone. You can sign in with Microsoft again at any time."
                    else "The offline name ${state.config.name} is forgotten. Your worlds stay where they are.",
                )
            },
            confirmButton = {
                Button(
                    onClick = {
                        if (microsoft) act("sign_out") else act("text", "name=")
                        confirm = false
                    },
                    contentPadding = ButtonDefaults.ButtonWithIconContentPadding,
                ) {
                    Icon(Icons.AutoMirrored.Rounded.Logout, null, Modifier.size(18.dp))
                    Spacer(Modifier.width(8.dp))
                    Text("Log out")
                }
            },
            dismissButton = { DiscardButton { confirm = false } },
        )
    }
    if (microsoft || state.config.name.isNotBlank()) {
        OutlinedButton(
            onClick = { confirm = true },
            contentPadding = ButtonDefaults.ButtonWithIconContentPadding,
        ) {
            Icon(Icons.AutoMirrored.Rounded.Logout, null, Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text("Log out")
        }
    } else {
        FilledTonalButton(
            onClick = { actions.edit("name", "Offline player name", "", "Up to 16 characters") },
            contentPadding = ButtonDefaults.ButtonWithIconContentPadding,
        ) {
            Icon(Icons.AutoMirrored.Rounded.Login, null, Modifier.size(18.dp))
            Spacer(Modifier.width(8.dp))
            Text("Log in offline")
        }
    }
}

/** Microsoft sign-in: the device code when one is open, the message, and the sign-in buttons. */
@Composable
private fun AccountSignIn(account: Account) {
    account.code?.let { code ->
        Surface(
            shape = RoundedCornerShape(24.dp),
            color = MaterialTheme.colorScheme.primaryContainer,
            modifier = Modifier.fillMaxWidth(),
        ) {
            Column(Modifier.padding(16.dp), horizontalAlignment = Alignment.CenterHorizontally) {
                Text("Your sign-in code", style = MaterialTheme.typography.labelLarge)
                Text(
                    code.userCode,
                    style = MaterialTheme.typography.displaySmall,
                    fontFamily = FontFamily.Monospace,
                    fontWeight = FontWeight.Bold,
                    color = MaterialTheme.colorScheme.onPrimaryContainer,
                )
                Row(horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilledTonalButton(onClick = { act("copy_code") }) {
                        Icon(Icons.Rounded.ContentCopy, null, Modifier.size(18.dp))
                        Text("  Copy")
                    }
                    Button(onClick = { act("open_link") }) {
                        Icon(Icons.Rounded.OpenInBrowser, null, Modifier.size(18.dp))
                        Text("  Open Microsoft")
                    }
                }
            }
        }
    }
    val message = account.message.ifBlank {
        if (account.signedIn) "Account saved securely. Sessions renew automatically."
        else "Your password is entered in Microsoft's browser, never here."
    }
    Text(message, textAlign = TextAlign.Center, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
    if (account.needsReview) {
        OutlinedButton(onClick = { act("review_app") }) { Text("Request Minecraft API approval") }
    }
    if (account.pending) {
        LinearWavyProgressIndicatorCompat()
        OutlinedButton(onClick = { act("cancel_login") }, modifier = Modifier.fillMaxWidth().height(48.dp)) {
            Text("Cancel sign-in")
        }
    } else {
        Button(onClick = { act("sign_in") }, modifier = Modifier.fillMaxWidth().height(48.dp)) {
            Text(if (account.signedIn) "Switch Microsoft account" else "Sign in with Microsoft")
        }
        FilledTonalButton(onClick = { act("device_sign_in") }, modifier = Modifier.fillMaxWidth().height(48.dp)) {
            Text("Use a device code instead")
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LinearWavyProgressIndicatorCompat() {
    LinearWavyProgressIndicator(Modifier.fillMaxWidth())
}
