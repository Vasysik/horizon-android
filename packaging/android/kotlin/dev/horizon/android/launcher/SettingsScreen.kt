package dev.horizon.android.launcher

import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
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
    LazyColumn(
        Modifier.fillMaxSize(),
        contentPadding = PaddingValues(bottom = 24.dp),
        verticalArrangement = Arrangement.spacedBy(2.dp),
    ) {
        state.settings.forEach { group ->
            item(key = "group-${group.name}") {
                Text(
                    group.name,
                    style = MaterialTheme.typography.titleSmall,
                    color = MaterialTheme.colorScheme.primary,
                    modifier = Modifier.padding(start = 16.dp, top = 16.dp, bottom = 8.dp),
                )
            }
            itemsIndexed(group.rows, key = { _, row -> row.id }) { i, row ->
                // A connected group: large corners at its ends, small ones between rows.
                val top = if (i == 0) 24.dp else 6.dp
                val bottom = if (i == group.rows.lastIndex) 24.dp else 6.dp
                Surface(
                    shape = RoundedCornerShape(topStart = top, topEnd = top, bottomStart = bottom, bottomEnd = bottom),
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
                                modifier = Modifier.size(44.dp),
                            ) {
                                Box(contentAlignment = Alignment.Center) {
                                    Icon(settingIcon(row.id), null, tint = MaterialTheme.colorScheme.onSecondaryContainer)
                                }
                            }
                        },
                        headlineContent = { Text(row.title, fontWeight = FontWeight.Medium) },
                        supportingContent = { Text(row.value) },
                        trailingContent = {
                            when (row.kind) {
                                "switch" -> Switch(checked = row.on, onCheckedChange = { act("toggle", row.id) })
                                "choice" -> Icon(Icons.Rounded.UnfoldMore, null)
                                else -> Icon(Icons.AutoMirrored.Rounded.KeyboardArrowRight, null)
                            }
                        },
                    )
                }
            }
        }
        item(key = "account") {
            Row(
                Modifier.fillMaxWidth().padding(top = 20.dp),
                horizontalArrangement = Arrangement.spacedBy(12.dp),
            ) {
                FilledTonalButton(onClick = { act("repository") }, modifier = Modifier.weight(1f).height(56.dp)) {
                    Icon(Icons.Rounded.OpenInBrowser, null)
                    Text("  Source code")
                }
                Button(onClick = actions.openAccount, modifier = Modifier.weight(1f).height(56.dp)) {
                    Icon(Icons.Rounded.Person, null)
                    Text(if (wide) "  Microsoft / Xbox account" else "  Account")
                }
            }
        }
    }
}

@Composable
fun ChooseDialog(choose: Overlay.Choose, onDismiss: () -> Unit, onCustomScale: () -> Unit) {
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(choose.options.title) },
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
                choose.options.choices.forEach { c ->
                    Row(
                        Modifier
                            .fillMaxWidth()
                            .heightIn(min = 56.dp)
                            .selectable(
                                selected = c.selected,
                                role = Role.RadioButton,
                                onClick = {
                                    act("choose", "${choose.id}=${c.value}")
                                    onDismiss()
                                },
                            )
                            .padding(vertical = 4.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        RadioButton(selected = c.selected, onClick = null)
                        Column(Modifier.padding(start = 12.dp)) {
                            Text(c.label, style = MaterialTheme.typography.bodyLarge)
                            if (c.detail.isNotBlank()) {
                                Text(
                                    c.detail,
                                    style = MaterialTheme.typography.bodySmall,
                                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                                )
                            }
                        }
                    }
                }
                if (choose.id == "render_scale") {
                    TextButton(onClick = onCustomScale) { Text("Custom percentage…") }
                }
            }
        },
        confirmButton = { TextButton(onClick = onDismiss) { Text("Close") } },
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
        confirmButton = { Button(onClick = save) { Text("Save") } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
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
    val account = state.account
    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(
            Modifier.fillMaxWidth().padding(horizontal = 24.dp).padding(bottom = 32.dp).verticalScroll(rememberScrollState()),
            verticalArrangement = Arrangement.spacedBy(12.dp),
            horizontalAlignment = Alignment.CenterHorizontally,
        ) {
            Avatar(account, 88.dp)
            Text(
                account.profile ?: state.config.name.ifBlank { "Player" },
                style = MaterialTheme.typography.headlineSmall,
                fontWeight = FontWeight.Bold,
            )
            Text(
                account.status,
                style = MaterialTheme.typography.bodyLarge,
                color = if (account.signedIn) MaterialTheme.colorScheme.primary else MaterialTheme.colorScheme.onSurfaceVariant,
            )
            account.code?.let { code ->
                Surface(
                    shape = RoundedCornerShape(24.dp),
                    color = MaterialTheme.colorScheme.primaryContainer,
                    modifier = Modifier.fillMaxWidth(),
                ) {
                    Column(Modifier.padding(20.dp), horizontalAlignment = Alignment.CenterHorizontally) {
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
            Text(message, textAlign = TextAlign.Center, color = MaterialTheme.colorScheme.onSurfaceVariant)
            if (account.needsReview) {
                OutlinedButton(onClick = { act("review_app") }) { Text("Request Minecraft API approval") }
            }
            HorizontalDivider(Modifier.padding(vertical = 4.dp))
            if (account.pending) {
                LinearWavyProgressIndicatorCompat()
                OutlinedButton(onClick = { act("cancel_login") }, modifier = Modifier.fillMaxWidth().height(52.dp)) {
                    Text("Cancel sign-in")
                }
            } else {
                Button(onClick = { act("sign_in") }, modifier = Modifier.fillMaxWidth().height(56.dp)) {
                    Text(if (account.signedIn) "Switch Microsoft account" else "Sign in with Microsoft")
                }
                FilledTonalButton(onClick = { act("device_sign_in") }, modifier = Modifier.fillMaxWidth().height(52.dp)) {
                    Text("Use a device code instead")
                }
                if (account.profile != null) {
                    OutlinedButton(onClick = { act("sign_out") }, modifier = Modifier.fillMaxWidth().height(52.dp)) {
                        Icon(Icons.AutoMirrored.Rounded.Logout, null, Modifier.size(18.dp))
                        Text("  Sign out")
                    }
                }
                TextButton(onClick = {
                    actions.edit("name", "Offline player name", state.config.name, "Used when signed out of Microsoft")
                }) {
                    Text(
                        if (state.config.name.isBlank()) "Set an offline player name"
                        else "Offline name: ${state.config.name}",
                    )
                }
            }
            Spacer(Modifier.height(8.dp))
        }
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun LinearWavyProgressIndicatorCompat() {
    LinearWavyProgressIndicator(Modifier.fillMaxWidth())
}
