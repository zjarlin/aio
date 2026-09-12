package site.addzero.aio.example.tasks

import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import site.addzero.aio.example.counter.CounterResponse
import java.time.Instant

internal class TaskStore {
    private data class TenantData(
        val tasks: MutableList<TaskItem> = mutableListOf(
            TaskItem(1, "Review the plugin contract", true),
            TaskItem(2, "Build the Compose screen", false),
            TaskItem(3, "Verify tenant isolation", false),
        ),
        var nextId: Long = 4,
        var count: Long = 0,
    )
    private val tenants = mutableMapOf<String, TenantData>()
    private val mutex = Mutex()

    private fun tenant(id: String): TenantData {
        require(id.isNotBlank() && id.length <= 128) { "Missing tenant context" }
        require(id in tenants || tenants.size < 64) { "Tenant quota reached" }
        return tenants.getOrPut(id) { TenantData() }
    }

    suspend fun page(tenantId: String, userId: String, search: String): TaskPage = mutex.withLock {
        require(search.length <= 120) { "Search is too long" }
        val data = tenant(tenantId)
        TaskPage(
            items = data.tasks.filter { it.title.contains(search.trim(), ignoreCase = true) },
            total = data.tasks.size,
            completed = data.tasks.count { it.done },
            tenantId = tenantId,
            userId = userId,
            serverTime = Instant.now().toString(),
        )
    }

    suspend fun create(tenantId: String, request: CreateTask): TaskItem = mutex.withLock {
        val title = request.title.trim()
        require(title.isNotEmpty() && title.length <= 120) { "Title must contain 1 to 120 characters" }
        val data = tenant(tenantId)
        require(data.tasks.size < 200) { "Task quota reached" }
        TaskItem(data.nextId++, title, false).also { data.tasks.add(it) }
    }

    suspend fun update(tenantId: String, id: Long, request: UpdateTask): TaskItem? = mutex.withLock {
        val data = tenant(tenantId)
        val index = data.tasks.indexOfFirst { it.id == id }
        if (index < 0) null else data.tasks[index].copy(done = request.done).also { data.tasks[index] = it }
    }

    suspend fun remove(tenantId: String, id: Long): Boolean = mutex.withLock {
        tenant(tenantId).tasks.removeIf { it.id == id }
    }

    suspend fun counter(tenantId: String, increment: Boolean): CounterResponse = mutex.withLock {
        val data = tenant(tenantId)
        if (increment) data.count = Math.addExact(data.count, 1)
        CounterResponse(data.count, tenantId)
    }
}
