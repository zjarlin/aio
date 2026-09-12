package site.addzero.aio.example.tasks

import kotlinx.serialization.Serializable

@Serializable
data class TaskItem(val id: Long, val title: String, val done: Boolean)

@Serializable
data class TaskPage(
    val items: List<TaskItem>,
    val total: Int,
    val completed: Int,
    val tenantId: String,
    val userId: String,
    val serverTime: String,
    val storage: String = "demo-memory",
)

@Serializable
data class CreateTask(val title: String)

@Serializable
data class UpdateTask(val done: Boolean)

@Serializable
data class ServiceError(val error: String)
