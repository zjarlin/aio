package site.addzero.aio.plugin.kmpservice

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private val wireJson = Json {
    encodeDefaults = true
}

@Serializable
data class SceneDefinition(
    val id: String,
    val label: String,
)

@Serializable
data class ActionDefinition(
    val id: String,
    val label: String,
)

@Serializable
data class ActionState(
    val count: Long,
)

@Serializable
data class ActionsBody(
    val kind: String = "actions",
    val title: String,
    val content: String,
    val state: ActionState,
    val actions: List<ActionDefinition>,
)

@Serializable
data class PageDefinition(
    val id: String,
    val label: String,
    val icon: String?,
    val scene: SceneDefinition,
    @SerialName("required_permission")
    val requiredPermission: String?,
    val body: ActionsBody,
)

@Serializable
data class PageActionRequest(
    val kind: String,
    @SerialName("page_id")
    val pageId: String,
    @SerialName("action_id")
    val actionId: String,
    @SerialName("tenant_id")
    val tenantId: String,
    @SerialName("user_id")
    val userId: String,
    val body: ActionsBody,
)

@Serializable
data class PageActionResult(
    val body: ActionsBody,
)

data class RequestContext(
    val tenantId: String,
    val userId: String,
)

@Serializable
data class EchoResponse(
    val runtime: String = "kotlin-jvm",
    val version: Int = 2,
    val method: String,
    val path: String,
    val query: String?,
    val body: String,
    @SerialName("tenant_id")
    val tenantId: String,
    @SerialName("user_id")
    val userId: String,
) {
    fun encode(): String = wireJson.encodeToString(this)
}

@Serializable
data class ErrorResponse(
    val error: String,
) {
    fun encode(): String = wireJson.encodeToString(this)
}

object KmpProcessPlugin {
    fun pages(count: Long = 0): List<PageDefinition> = listOf(
        PageDefinition(
            id = "kmp-process",
            label = "KMP 服务",
            icon = "server",
            scene = SceneDefinition("community", "社区插件"),
            requiredPermission = null,
            body = actionBody(count),
        ),
    )

    fun definitionJson(count: Long = 0): String = wireJson.encodeToString(pages(count))

    fun actionResultJson(eventJson: String): String {
        val event = wireJson.decodeFromString<PageActionRequest>(eventJson)
        require(event.kind == "page_action") { "plugin request kind is not supported" }
        require(event.pageId == "kmp-process" && event.actionId == "increment") {
            "page action is not declared"
        }
        require(event.tenantId.isNotBlank() && event.userId.isNotBlank()) {
            "trusted request context is required"
        }
        val count = event.body.state.count
        require(count in 0 until Long.MAX_VALUE) { "page state count must be non-negative" }
        return wireJson.encodeToString(PageActionResult(actionBody(count + 1)))
    }

    private fun actionBody(count: Long): ActionsBody = ActionsBody(
        title = "Kotlin 进程插件 v2 已在线",
        content = "计数：$count",
        state = ActionState(count),
        actions = listOf(ActionDefinition("increment", "Kotlin +1")),
    )
}
