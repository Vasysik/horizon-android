package dev.horizon.android.launcher

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.height
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.grid.GridCells
import androidx.compose.foundation.lazy.grid.GridItemSpan
import androidx.compose.foundation.lazy.grid.LazyVerticalGrid
import androidx.compose.foundation.lazy.grid.itemsIndexed
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Check
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.Edit
import androidx.compose.material.icons.rounded.MoreVert
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.ButtonDefaults
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialShapes
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.material3.toShape
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

/** A profile action waiting for its dialog: create, rename or delete. */
private sealed interface ProfileEdit {
    data object Create : ProfileEdit
    data class Rename(val profile: Profile) : ProfileEdit
    data class Delete(val profile: Profile) : ProfileEdit
}

/** What a profile holds, for telling profiles apart at a glance. */
private fun Profile.summary(): String {
    fun part(n: Int, one: String, many: String) = if (n == 0) null else "$n ${if (n == 1) one else many}"
    return listOfNotNull(
        part(worlds, "world", "worlds"),
        part(mods, "mod", "mods"),
        part(packs, "pack", "packs"),
        part(shaders, "shader", "shaders"),
    ).joinToString("  ·  ").ifEmpty { "Empty" }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun ProfilesSheet(onDismiss: () -> Unit) {
    var profiles by remember { mutableStateOf(Profile.load()) }
    val reload = { profiles = Profile.load() }
    var edit by remember { mutableStateOf<ProfileEdit?>(null) }

    ModalBottomSheet(onDismissRequest = onDismiss, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        // One scrolling grid, header included. Sideways, profiles pair up in two columns.
        val landscape = isLandscape()
        LazyVerticalGrid(
            columns = GridCells.Fixed(if (landscape) 2 else 1),
            contentPadding = PaddingValues(start = 16.dp, end = 16.dp, bottom = 24.dp),
            verticalArrangement = Arrangement.spacedBy(if (landscape) 8.dp else 2.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp),
        ) {
            item(key = "header", span = { GridItemSpan(maxLineSpan) }) {
                Row(Modifier.fillMaxWidth().padding(start = 8.dp, bottom = 12.dp), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text("Profiles", style = MaterialTheme.typography.headlineSmall, fontWeight = FontWeight.SemiBold)
                        Text(
                            "Each profile keeps its own worlds, settings and add-ons.",
                            style = MaterialTheme.typography.bodyMedium,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                    Spacer(Modifier.width(12.dp))
                    Button(onClick = { edit = ProfileEdit.Create }, contentPadding = ButtonDefaults.ButtonWithIconContentPadding) {
                        Icon(Icons.Rounded.Add, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("New")
                    }
                }
            }
            itemsIndexed(profiles, key = { _, p -> p.id }) { i, profile ->
                ProfileRow(
                    profile,
                    if (landscape) RoundedCornerShape(20.dp) else connectedShape(i, profiles.size, outer = 24),
                    onSelect = {
                        if (!profile.active) {
                            act("choose", "profile=${profile.id}")
                            reload()
                        }
                    },
                    onRename = { edit = ProfileEdit.Rename(profile) },
                    onDelete = { edit = ProfileEdit.Delete(profile) },
                )
            }
        }
    }

    when (val e = edit) {
        ProfileEdit.Create -> NameDialog("New profile", "", "Create", onDismiss = { edit = null }) { name ->
            act("profile_create", name)
            reload()
        }
        is ProfileEdit.Rename -> NameDialog("Rename profile", e.profile.name, "Save", onDismiss = { edit = null }) { name ->
            act("profile_rename", "${e.profile.id}=$name")
            reload()
        }
        is ProfileEdit.Delete -> AlertDialog(
            onDismissRequest = { edit = null },
            icon = { Icon(Icons.Rounded.DeleteOutline, null) },
            title = { Text("Delete ${e.profile.name}?") },
            text = { Text("Its worlds, settings and add-ons (${e.profile.summary().lowercase()}) are deleted from this phone. This cannot be undone.") },
            confirmButton = {
                Button(
                    onClick = {
                        act("profile_delete", e.profile.id.toString())
                        reload()
                        edit = null
                    },
                    colors = ButtonDefaults.buttonColors(
                        containerColor = MaterialTheme.colorScheme.error,
                        contentColor = MaterialTheme.colorScheme.onError,
                    ),
                ) { Text("Delete") }
            },
            dismissButton = { TextButton(onClick = { edit = null }) { Text("Cancel") } },
        )
        null -> {}
    }
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
private fun ProfileRow(
    profile: Profile,
    shape: androidx.compose.ui.graphics.Shape,
    onSelect: () -> Unit,
    onRename: () -> Unit,
    onDelete: () -> Unit,
) {
    val colors = MaterialTheme.colorScheme
    Surface(
        onClick = onSelect,
        shape = shape,
        color = if (profile.active) colors.secondaryContainer else colors.surfaceContainer,
    ) {
        Row(Modifier.padding(start = 12.dp, top = 10.dp, bottom = 10.dp), verticalAlignment = Alignment.CenterVertically) {
            Surface(
                modifier = Modifier.size(40.dp),
                shape = if (profile.active) MaterialShapes.Cookie9Sided.toShape() else MaterialShapes.Circle.toShape(),
                color = if (profile.active) colors.primary else colors.surfaceContainerHighest,
            ) {
                Box(contentAlignment = Alignment.Center) {
                    if (profile.active) {
                        Icon(Icons.Rounded.Check, null, tint = colors.onPrimary)
                    } else {
                        Text(
                            profile.name.firstOrNull()?.uppercase() ?: "?",
                            style = MaterialTheme.typography.titleMedium,
                            color = colors.onSurfaceVariant,
                        )
                    }
                }
            }
            Column(Modifier.padding(start = 14.dp).weight(1f)) {
                Text(
                    profile.name,
                    style = MaterialTheme.typography.titleMedium,
                    fontWeight = FontWeight.SemiBold,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
                Text(
                    if (profile.active) "In use  ·  ${profile.summary()}" else profile.summary(),
                    style = MaterialTheme.typography.bodyMedium,
                    color = colors.onSurfaceVariant,
                    maxLines = 1,
                    overflow = TextOverflow.Ellipsis,
                )
            }
            var menu by remember { mutableStateOf(false) }
            Box {
                IconButton(onClick = { menu = true }) { Icon(Icons.Rounded.MoreVert, "More for ${profile.name}") }
                DropdownMenu(expanded = menu, onDismissRequest = { menu = false }) {
                    DropdownMenuItem(
                        text = { Text("Rename") },
                        leadingIcon = { Icon(Icons.Rounded.Edit, null) },
                        onClick = { menu = false; onRename() },
                    )
                    // The default profile is the app's home folder; the active one is in use.
                    val deletable = profile.id != 0 && !profile.active
                    DropdownMenuItem(
                        text = {
                            Column {
                                Text("Delete")
                                if (!deletable) {
                                    Text(
                                        if (profile.id == 0) "Default profile" else "Switch first",
                                        style = MaterialTheme.typography.bodySmall,
                                    )
                                }
                            }
                        },
                        leadingIcon = { Icon(Icons.Rounded.DeleteOutline, null) },
                        enabled = deletable,
                        onClick = { menu = false; onDelete() },
                    )
                }
            }
        }
    }
}

@Composable
private fun NameDialog(title: String, initial: String, confirm: String, onDismiss: () -> Unit, onSave: (String) -> Unit) {
    var name by remember { mutableStateOf(initial) }
    val save = {
        onSave(name.trim())
        onDismiss()
    }
    AlertDialog(
        onDismissRequest = onDismiss,
        title = { Text(title) },
        text = {
            OutlinedTextField(
                value = name,
                onValueChange = { if (it.length <= 32) name = it },
                label = { Text("Name") },
                placeholder = { Text("Survival, Modded, Creative…") },
                supportingText = { Text("${name.length}/32") },
                singleLine = true,
                modifier = Modifier.fillMaxWidth(),
            )
        },
        confirmButton = { Button(onClick = save, enabled = name.isNotBlank()) { Text(confirm) } },
        dismissButton = { TextButton(onClick = onDismiss) { Text("Cancel") } },
    )
}
