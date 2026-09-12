package site.addzero.aio.example.tasks

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.filled.Add
import androidx.compose.material.icons.filled.Delete
import androidx.compose.material.icons.filled.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.Checkbox
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.style.TextDecoration
import androidx.compose.ui.unit.dp

@Composable
internal fun TasksPage() {
    val scope = rememberCoroutineScope()
    val state = remember { TaskState(scope) }
    LaunchedEffect(Unit) { state.reload() }
    Box(Modifier.fillMaxSize().verticalScroll(rememberScrollState()), contentAlignment = Alignment.TopCenter) {
        Column(
            Modifier.widthIn(max = 960.dp).fillMaxWidth().padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp),
        ) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Column(Modifier.weight(1f)) {
                    Text("KMP Workspace", style = MaterialTheme.typography.titleLarge)
                    Text("Tasks", style = MaterialTheme.typography.bodyMedium, color = MaterialTheme.colorScheme.onSurfaceVariant)
                }
                Text("Demo", style = MaterialTheme.typography.labelMedium, color = MaterialTheme.colorScheme.secondary)
            }
            state.page?.let { page ->
                Text("${page.total} tasks  /  ${page.completed} completed", style = MaterialTheme.typography.bodyMedium)
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(
                    value = state.search,
                    onValueChange = { state.search = it.take(120) },
                    modifier = Modifier.weight(1f),
                    singleLine = true,
                    label = { Text("Search tasks") },
                )
                IconButton(onClick = state::reload, enabled = !state.busy) { Icon(Icons.Default.Refresh, "Refresh") }
            }
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
                listOf("All", "Open", "Done").forEachIndexed { index, label ->
                    FilterChip(selected = state.filter == index, onClick = { state.filter = index }, label = { Text(label) })
                }
                Box(Modifier.weight(1f))
                Button(onClick = { state.creating = true }, enabled = !state.busy) {
                    Icon(Icons.Default.Add, "New task")
                }
            }
            Box(Modifier.fillMaxWidth()) {
                if (state.busy) LinearProgressIndicator(Modifier.fillMaxWidth())
                else HorizontalDivider()
            }
            state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            if (!state.busy && state.visible.isEmpty()) Text("No tasks", style = MaterialTheme.typography.bodyLarge)
            state.visible.forEach { task ->
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Checkbox(checked = task.done, onCheckedChange = { state.update(task, it) }, enabled = !state.busy)
                    Column(Modifier.weight(1f).padding(horizontal = 8.dp)) {
                        Text(task.title, textDecoration = if (task.done) TextDecoration.LineThrough else TextDecoration.None)
                        Text(if (task.done) "Done" else "Open", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.onSurfaceVariant)
                    }
                    IconButton(onClick = { state.deleting = task }, enabled = !state.busy) { Icon(Icons.Default.Delete, "Delete ${task.title}") }
                }
                HorizontalDivider()
            }
            state.page?.let { page ->
                Text("Tenant: ${page.tenantId}", style = MaterialTheme.typography.bodySmall)
                Text("User: ${page.userId}", style = MaterialTheme.typography.bodySmall)
                Text("Updated: ${page.serverTime}", style = MaterialTheme.typography.bodySmall)
            }
        }
    }
    if (state.creating) CreateTaskDialog(state)
    state.deleting?.let { DeleteTaskDialog(state, it) }
}
