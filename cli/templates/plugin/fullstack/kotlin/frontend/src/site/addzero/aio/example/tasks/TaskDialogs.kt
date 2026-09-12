package site.addzero.aio.example.tasks

import androidx.compose.foundation.layout.Column
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue

@Composable
internal fun CreateTaskDialog(state: TaskState) {
    var title by remember { mutableStateOf("") }
    AlertDialog(
        onDismissRequest = { if (!state.busy) state.creating = false },
        title = { Text("New task") },
        text = {
            Column {
                OutlinedTextField(value = title, onValueChange = { title = it.take(120) }, label = { Text("Title") }, singleLine = true, enabled = !state.busy)
                state.error?.let { Text(it) }
            }
        },
        confirmButton = { TextButton(onClick = { state.create(title) }, enabled = title.isNotBlank() && !state.busy) { Text("Create") } },
        dismissButton = { TextButton(onClick = { state.creating = false }, enabled = !state.busy) { Text("Cancel") } },
    )
}

@Composable
internal fun DeleteTaskDialog(state: TaskState, task: TaskItem) {
    AlertDialog(
        onDismissRequest = { if (!state.busy) state.deleting = null },
        title = { Text("Delete task?") },
        text = { Text(task.title) },
        confirmButton = { TextButton(onClick = { state.remove(task) }, enabled = !state.busy) { Text("Delete") } },
        dismissButton = { TextButton(onClick = { state.deleting = null }, enabled = !state.busy) { Text("Cancel") } },
    )
}
