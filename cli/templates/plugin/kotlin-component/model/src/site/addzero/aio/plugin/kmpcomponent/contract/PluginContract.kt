package __PACKAGE_NAME__.contract

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.decodeFromString
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive

private val wireJson = Json { encodeDefaults = true }

@Serializable
internal data class SceneDefinition(
    val id: String,
    val label: String,
)

@Serializable
internal data class ActionDefinition(
    val id: String,
    val label: String,
)

@Serializable
internal data class ActionState(val count: Long)

@Serializable
internal data class ActionsBody(
    val kind: String = "actions",
    val title: String,
    val content: String,
    val state: ActionState,
    val actions: List<ActionDefinition>,
)

@Serializable
internal data class PageDefinition(
    val id: String,
    val label: String,
    val icon: String?,
    val scene: SceneDefinition,
    @SerialName("required_permission")
    val requiredPermission: String?,
    val body: ActionsBody,
)

@Serializable
internal data class PageActionRequest(
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
internal data class ServiceRequest(
    val kind: String,
    val method: String,
    val path: String,
    val query: String?,
    val body: String,
    @SerialName("tenant_id")
    val tenantId: String,
    @SerialName("user_id")
    val userId: String,
)

@Serializable
internal data class PageActionResult(val body: ActionsBody)

@Serializable
internal data class EchoResult(
    val runtime: String = "kotlin-wasm-component",
    val request: ServiceRequest,
)

@Serializable
internal data class ComponentResponse(
    val status: Int,
    @SerialName("content_type")
    val contentType: String = "application/json",
    val body: String,
)

private fun actionBody(count: Long) = ActionsBody(
    title = __TITLE_LITERAL__,
    content = "计数：$count",
    state = ActionState(count),
    actions = listOf(ActionDefinition("increment", "Kotlin +1")),
)

object KmpComponentContract {
    fun definition(): String = wireJson.encodeToString(
        listOf(
            PageDefinition(
                id = "__PLUGIN_ID__",
                label = __TITLE_LITERAL__,
                icon = "binary",
                scene = SceneDefinition("community", "社区插件"),
                requiredPermission = null,
                body = actionBody(0),
            ),
        ),
    )

    fun handle(request: String): String {
        val responseBody = when (
            wireJson.parseToJsonElement(request).jsonObject["kind"]?.jsonPrimitive?.content
        ) {
            "page_action" -> reducePageAction(wireJson.decodeFromString(request))
            "service_request" -> wireJson.encodeToString(
                EchoResult(request = wireJson.decodeFromString<ServiceRequest>(request)),
            )
            else -> error("不支持的插件请求类型")
        }
        return wireJson.encodeToString(ComponentResponse(status = 200, body = responseBody))
    }

    private fun reducePageAction(event: PageActionRequest): String {
        require(event.pageId == "__PLUGIN_ID__" && event.actionId == "increment") {
            "不支持的页面动作"
        }
        require(event.tenantId.isNotBlank() && event.userId.isNotBlank()) { "缺少租户或用户上下文" }
        require(event.body.state.count in 0 until Long.MAX_VALUE) { "计数超出范围" }
        return wireJson.encodeToString(
            PageActionResult(actionBody(event.body.state.count + 1)),
        )
    }
}
