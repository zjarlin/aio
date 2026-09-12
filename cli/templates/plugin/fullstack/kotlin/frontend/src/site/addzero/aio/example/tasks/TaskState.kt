package site.addzero.aio.example.tasks

import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import kotlinx.coroutines.CoroutineScope
import kotlinx.coroutines.launch

internal class TaskState(private val scope: CoroutineScope) {
    var page by mutableStateOf<TaskPage?>(null)
        private set
    var busy by mutableStateOf(false)
        private set
    var error by mutableStateOf<String?>(null)
    var creating by mutableStateOf(false)
    var deleting by mutableStateOf<TaskItem?>(null)
    var search by mutableStateOf("")
    var filter by mutableStateOf(0)

    val visible: List<TaskItem>
        get() = page?.items.orEmpty().filter {
            it.title.contains(search, ignoreCase = true) && when (filter) { 1 -> !it.done; 2 -> it.done; else -> true }
        }

    fun reload() = run { }
    fun create(title: String) = run {
        TaskClient.create(title)
        creating = false
    }
    fun update(task: TaskItem, done: Boolean) = run { TaskClient.update(task, done) }
    fun remove(task: TaskItem) = run {
        TaskClient.remove(task)
        deleting = null
    }

    private fun run(operation: suspend () -> Unit) {
        if (busy) return
        busy = true
        error = null
        scope.launch {
            try {
                operation()
                page = TaskClient.page()
            } catch (cause: Throwable) {
                error = cause.message ?: "Request failed"
            } finally {
                busy = false
            }
        }
    }
}
