package __PACKAGE_NAME__.contract

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PluginContractTest {
    @Test
    fun exposesAnActionsPage() {
        val page = Json.parseToJsonElement(KmpComponentContract.definition()).jsonArray.single().jsonObject

        assertEquals("__PLUGIN_ID__", page["id"]?.jsonPrimitive?.content)
        assertEquals("actions", page["body"]?.jsonObject?.get("kind")?.jsonPrimitive?.content)
        assertEquals(0, page["body"]?.jsonObject?.get("state")?.jsonObject?.get("count")?.jsonPrimitive?.content?.toLong())
    }

    @Test
    fun incrementsOnlyTheRequestedState() {
        val response = KmpComponentContract.handle(
            """{"kind":"page_action","page_id":"__PLUGIN_ID__","action_id":"increment","tenant_id":"tenant-a","user_id":"user-a","body":{"kind":"actions","title":__TITLE_LITERAL__,"content":"计数：4","state":{"count":4},"actions":[{"id":"increment","label":"Kotlin +1"}]}}""",
        )
        val body = Json.parseToJsonElement(response).jsonObject["body"]?.jsonPrimitive?.content.orEmpty()
        val count = Json.parseToJsonElement(body).jsonObject["body"]
            ?.jsonObject?.get("state")?.jsonObject?.get("count")?.jsonPrimitive?.content?.toLong()

        assertEquals(5, count)
    }

    @Test
    fun echoesTheTenantScopedServiceRequest() {
        val response = KmpComponentContract.handle(
            """{"kind":"service_request","method":"POST","path":"/echo","query":null,"body":"hello","tenant_id":"tenant-a","user_id":"user-a"}""",
        )

        assertTrue(response.contains("tenant-a"))
        assertTrue(response.contains("kotlin-wasm-component"))
    }
}
