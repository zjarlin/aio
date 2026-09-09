package site.addzero.aio.plugin.kmpservice

import kotlinx.serialization.SerializationException
import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertFailsWith

private val pageActionJson =
    """
    {
      "kind": "page_action",
      "page_id": "kmp-process",
      "action_id": "increment",
      "tenant_id": "tenant-a",
      "user_id": "user-a",
      "body": {
        "kind": "actions",
        "title": "Kotlin",
        "content": "计数：2",
        "state": { "count": 2 },
        "actions": [{ "id": "increment", "label": "Kotlin +1" }]
      }
    }
    """.trimIndent()

class RuntimeModelTest {
    @Test
    fun exposesDeclaredPageDefinition() {
        val pages = KmpProcessPlugin.pages()

        assertEquals(listOf("kmp-process"), pages.map(PageDefinition::id))
        val definition = Json.parseToJsonElement(KmpProcessPlugin.definitionJson())
        assertEquals(
            "actions",
            definition.jsonArray.single().jsonObject["body"]
                ?.jsonObject
                ?.get("kind")
                ?.jsonPrimitive
                ?.content,
        )
    }

    @Test
    fun reducesTypedHostPageAction() {
        val result = Json.parseToJsonElement(
            KmpProcessPlugin.actionResultJson(pageActionJson),
        ).jsonObject["body"]?.jsonObject

        assertEquals("计数：3", result?.get("content")?.jsonPrimitive?.content)
        assertEquals(3, result?.get("state")?.jsonObject?.get("count")?.jsonPrimitive?.content?.toLong())
    }

    @Test
    fun rejectsAmbiguousOrUnsupportedRequests() {
        assertFailsWith<SerializationException> {
            KmpProcessPlugin.actionResultJson(
                pageActionJson.replace("\"kind\": \"page_action\",", ""),
            )
        }
        assertFailsWith<IllegalArgumentException> {
            KmpProcessPlugin.actionResultJson(
                pageActionJson.replace("\"page_action\"", "\"service_request\""),
            )
        }
    }

    @Test
    fun encodesTrustedContextAndEscapesInput() {
        val response = Json.parseToJsonElement(
            EchoResponse(
                method = "POST",
                path = "/echo",
                query = null,
                body = "say \"hello\"\n",
                tenantId = "tenant-a",
                userId = "user-a",
            ).encode(),
        ).jsonObject

        assertEquals("say \"hello\"\n", response["body"]?.jsonPrimitive?.content)
        assertEquals(2, response["version"]?.jsonPrimitive?.content?.toInt())
        assertEquals("tenant-a", response["tenant_id"]?.jsonPrimitive?.content)
        assertEquals("user-a", response["user_id"]?.jsonPrimitive?.content)
    }
}
