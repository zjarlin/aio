package __PACKAGE_NAME__

import kotlinx.serialization.SerialName
import kotlinx.serialization.Serializable
import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json

private val wireJson = Json {
    classDiscriminator = "kind"
    encodeDefaults = true
}

@Serializable
data class SceneDefinition(
    val id: String,
    val label: String,
)

@Serializable
sealed interface PageBody

@Serializable
@SerialName("counter")
data class CounterBody(
    val title: String,
    val button: String,
) : PageBody

@Serializable
@SerialName("text")
data class TextBody(
    val title: String,
    val content: String,
) : PageBody

@Serializable
data class PageDefinition(
    val id: String,
    val label: String,
    val icon: String?,
    val scene: SceneDefinition,
    @SerialName("required_permission")
    val requiredPermission: String?,
    val body: PageBody,
)

object PagePlugin {
    val pages: List<PageDefinition> = listOf(
        PageDefinition(
            id = __PRIMARY_ID__,
            label = __TITLE__,
            icon = "binary",
            scene = SceneDefinition("community", "社区插件"),
            requiredPermission = null,
            body = CounterBody("Kotlin Multiplatform", "Kotlin +1"),
        ),
        PageDefinition(
            id = __ABOUT_ID__,
            label = "KMP 说明",
            icon = "info",
            scene = SceneDefinition("community", "社区插件"),
            requiredPermission = null,
            body = TextBody(
                title = "由 Kotlin Toolchain 构建",
                content = "页面模型来自 commonMain，并同时通过 JVM 与 wasmJs 编译。",
            ),
        ),
    )

    fun definitionJson(): String {
        require(pages.isNotEmpty()) { "插件至少需要一个页面" }
        require(pages.map(PageDefinition::id).distinct().size == pages.size) { "页面 ID 不能重复" }
        return wireJson.encodeToString(pages)
    }
}
