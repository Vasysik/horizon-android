package dev.horizon.android.launcher

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.items
import androidx.compose.foundation.shape.CircleShape
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.rounded.NavigateBefore
import androidx.compose.material.icons.automirrored.rounded.NavigateNext
import androidx.compose.material.icons.rounded.Add
import androidx.compose.material.icons.rounded.Clear
import androidx.compose.material.icons.rounded.DeleteOutline
import androidx.compose.material.icons.rounded.Download
import androidx.compose.material.icons.rounded.ExpandLess
import androidx.compose.material.icons.rounded.ExpandMore
import androidx.compose.material.icons.rounded.FolderOpen
import androidx.compose.material.icons.rounded.Search
import androidx.compose.material3.Badge
import androidx.compose.material3.Card
import androidx.compose.material3.CardDefaults
import androidx.compose.material3.ExperimentalMaterial3ExpressiveApi
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.ListItem
import androidx.compose.material3.ListItemDefaults
import androidx.compose.material3.LoadingIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SegmentedButton
import androidx.compose.material3.SegmentedButtonDefaults
import androidx.compose.material3.SingleChoiceSegmentedButtonRow
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.TextField
import androidx.compose.material3.TextFieldDefaults
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateMapOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.font.FontWeight
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp

private fun kindOf(page: Destination) = when (page) {
    Destination.Mods -> "mods"
    Destination.Shaders -> "shaders"
    else -> "packs"
}

@OptIn(ExperimentalMaterial3ExpressiveApi::class)
@Composable
fun CatalogScreen(page: Destination, state: Snapshot, wide: Boolean, actions: LauncherActions) {
    val catalog = state.catalog
    var query by rememberSaveable(page) { mutableStateOf(catalog.query) }
    val keyboard = LocalSoftwareKeyboardController.current
    val search = {
        keyboard?.hide()
        act("search", query)
    }
    val kind = kindOf(page)
    val hint = when {
        page == Destination.Mods && state.config.modSource == "modrinth" ->
            "Modrinth Java mods are browse-only: Fabric, Forge and NeoForge cannot run here."
        page == Destination.Mods -> "Horizon content mods (.hmod). Paste an HTTPS .hmod link to import it."
        page == Destination.Shaders && state.config.shaderSource == "modrinth" ->
            "Modrinth shader ZIPs are experimental; some packs may not work."
        page == Destination.Shaders -> "Horizon native .hshader packs."
        else -> "Verified downloads. Enable them in Installed."
    }
    // One list for controls and results: on a phone held sideways the search field and buttons scroll
    // away instead of squeezing the results to nothing.
    LazyColumn(
        Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        contentPadding = PaddingValues(bottom = 8.dp),
    ) {
        item(key = "controls") {
            Column(verticalArrangement = Arrangement.spacedBy(12.dp), modifier = Modifier.padding(bottom = 4.dp)) {
                TextField(
                    value = query,
                    onValueChange = { query = it },
                    modifier = Modifier.fillMaxWidth(),
                    placeholder = {
                        Text(if (page == Destination.Mods) "Search mods or paste an .hmod link" else "Search")
                    },
                    leadingIcon = { Icon(Icons.Rounded.Search, null) },
                    trailingIcon = {
                        if (query.isNotEmpty()) {
                            IconButton(onClick = { query = ""; act("search", "") }) { Icon(Icons.Rounded.Clear, "Clear") }
                        }
                    },
                    singleLine = true,
                    shape = CircleShape,
                    colors = TextFieldDefaults.colors(
                        focusedIndicatorColor = Color.Transparent,
                        unfocusedIndicatorColor = Color.Transparent,
                        disabledIndicatorColor = Color.Transparent,
                    ),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                    keyboardActions = KeyboardActions(onSearch = { search() }),
                )
                Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilledTonalButton(onClick = { act("open_folder", kind) }, modifier = Modifier.weight(1f)) {
                        Icon(Icons.Rounded.FolderOpen, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Folder", maxLines = 1)
                    }
                    FilledTonalButton(onClick = { actions.import(kind) }, modifier = Modifier.weight(1f)) {
                        Icon(Icons.Rounded.Add, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(8.dp))
                        Text("Import", maxLines = 1)
                    }
                }
                if (page == Destination.Mods || page == Destination.Shaders) {
                    // The source picker gets its own full-width row so both labels always fit.
                    val selected = if (page == Destination.Mods) state.config.modSource else state.config.shaderSource
                    SingleChoiceSegmentedButtonRow(Modifier.fillMaxWidth()) {
                        listOf("horizon" to "Horizon", "modrinth" to "Modrinth").forEachIndexed { i, (id, label) ->
                            SegmentedButton(
                                selected = selected == id,
                                onClick = { act("source", id) },
                                shape = SegmentedButtonDefaults.itemShape(i, 2),
                                modifier = Modifier.heightIn(min = 48.dp),
                                label = { Text(label, style = MaterialTheme.typography.labelLarge, maxLines = 1) },
                            )
                        }
                    }
                }
                Text(hint, style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
            }
        }
        when {
            catalog.busy && catalog.results.isEmpty() -> item(key = "loading") {
                Box(Modifier.fillMaxWidth().padding(vertical = 32.dp), contentAlignment = Alignment.Center) { LoadingIndicator() }
            }
            catalog.results.isEmpty() -> item(key = "empty") {
                Text(
                    "Nothing here yet. Search, or change the source.",
                    modifier = Modifier.fillMaxWidth().padding(vertical = 32.dp),
                    textAlign = androidx.compose.ui.text.style.TextAlign.Center,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                )
            }
            else -> {
                items(catalog.results, key = { it.index }) { ProjectCard(page, it) }
                item(key = "pager") {
                    Row(
                        Modifier.fillMaxWidth().padding(top = 4.dp),
                        horizontalArrangement = Arrangement.spacedBy(8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        OutlinedButton(onClick = { act("previous") }, enabled = catalog.offset > 0) {
                            Icon(Icons.AutoMirrored.Rounded.NavigateBefore, null)
                            Text("Previous")
                        }
                        OutlinedButton(onClick = { act("next") }) {
                            Text("Next")
                            Icon(Icons.AutoMirrored.Rounded.NavigateNext, null)
                        }
                        Spacer(Modifier.weight(1f))
                        Text(
                            "Page ${catalog.offset / 12 + 1}",
                            style = MaterialTheme.typography.labelLarge,
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
            }
        }
    }
}

@Composable
private fun ProjectCard(page: Destination, project: Project) {
    Card(
        shape = RoundedCornerShape(24.dp),
        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
    ) {
        ListItem(
            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
            leadingContent = {
                Surface(
                    shape = RoundedCornerShape(16.dp),
                    color = MaterialTheme.colorScheme.primaryContainer,
                    modifier = Modifier.size(44.dp),
                ) {
                    Box(contentAlignment = Alignment.Center) {
                        Icon(page.icon, null, tint = MaterialTheme.colorScheme.onPrimaryContainer)
                    }
                }
            },
            headlineContent = {
                Text(project.title, fontWeight = FontWeight.SemiBold, maxLines = 1, overflow = TextOverflow.Ellipsis)
            },
            supportingContent = {
                Column {
                    Text(project.summary, maxLines = 2, overflow = TextOverflow.Ellipsis)
                    if (project.author.isNotBlank()) {
                        Text(
                            "by ${project.author}",
                            style = MaterialTheme.typography.labelMedium,
                            color = MaterialTheme.colorScheme.outline,
                        )
                    }
                }
            },
            trailingContent = {
                when {
                    project.state != null -> Text(
                        project.state,
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.primary,
                    )
                    project.note != null -> Text(
                        project.note,
                        style = MaterialTheme.typography.labelLarge,
                        color = MaterialTheme.colorScheme.onSurfaceVariant,
                    )
                    else -> FilledTonalButton(onClick = { act("install", project.index.toString()) }) {
                        Icon(Icons.Rounded.Download, null, Modifier.size(18.dp))
                        Spacer(Modifier.width(6.dp))
                        Text(project.button)
                    }
                }
            },
        )
    }
}

@Composable
fun InstalledScreen(state: Snapshot, actions: LauncherActions) {
    val open = remember { mutableStateMapOf("mods" to true, "packs" to true, "shaders" to true) }
    val sections = listOf("mods" to "Mods", "packs" to "Resource packs", "shaders" to "Shaders")
    LazyColumn(
        Modifier.fillMaxSize(),
        verticalArrangement = Arrangement.spacedBy(8.dp),
        contentPadding = PaddingValues(bottom = 16.dp),
    ) {
        sections.forEach { (kind, label) ->
            val items = state.local.filter { it.kind == kind }
            val expanded = open[kind] == true
            item(key = "header-$kind") {
                Surface(
                    onClick = { open[kind] = !expanded },
                    shape = RoundedCornerShape(28.dp),
                    color = MaterialTheme.colorScheme.surfaceContainerHigh,
                ) {
                    Row(
                        Modifier.padding(start = 20.dp, end = 8.dp, top = 8.dp, bottom = 8.dp),
                        verticalAlignment = Alignment.CenterVertically,
                    ) {
                        Icon(if (expanded) Icons.Rounded.ExpandLess else Icons.Rounded.ExpandMore, null)
                        Text(
                            label,
                            style = MaterialTheme.typography.titleMedium,
                            modifier = Modifier.padding(start = 12.dp, end = 8.dp),
                        )
                        Badge(containerColor = MaterialTheme.colorScheme.primaryContainer) {
                            Text(items.size.toString(), color = MaterialTheme.colorScheme.onPrimaryContainer)
                        }
                        Spacer(Modifier.weight(1f))
                        FilledTonalButton(onClick = { actions.import(kind) }) {
                            Icon(Icons.Rounded.Add, null, Modifier.size(18.dp))
                            Spacer(Modifier.width(6.dp))
                            Text("Import")
                        }
                    }
                }
            }
            if (expanded) {
                if (items.isEmpty()) {
                    item(key = "empty-$kind") {
                        Text(
                            "Nothing installed yet.",
                            modifier = Modifier.padding(horizontal = 24.dp, vertical = 8.dp),
                            color = MaterialTheme.colorScheme.onSurfaceVariant,
                        )
                    }
                }
                items(items, key = { "item-${it.index}-${it.file}" }) { item ->
                    Card(
                        shape = RoundedCornerShape(20.dp),
                        colors = CardDefaults.cardColors(containerColor = MaterialTheme.colorScheme.surfaceContainer),
                    ) {
                        ListItem(
                            colors = ListItemDefaults.colors(containerColor = Color.Transparent),
                            headlineContent = {
                                Text(item.title, maxLines = 1, overflow = TextOverflow.Ellipsis, fontWeight = FontWeight.Medium)
                            },
                            supportingContent = { Text(item.file, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                            trailingContent = {
                                Row(verticalAlignment = Alignment.CenterVertically) {
                                    if (item.toggle) {
                                        HorizonSwitch(
                                            checked = item.enabled,
                                            onCheckedChange = { act("toggle_local", item.index.toString()) },
                                        )
                                    } else {
                                        Text("64-bit required", style = MaterialTheme.typography.labelMedium)
                                    }
                                    IconButton(onClick = { actions.remove(item) }) {
                                        Icon(Icons.Rounded.DeleteOutline, "Remove", tint = MaterialTheme.colorScheme.error)
                                    }
                                }
                            },
                        )
                    }
                }
            }
        }
    }
}
