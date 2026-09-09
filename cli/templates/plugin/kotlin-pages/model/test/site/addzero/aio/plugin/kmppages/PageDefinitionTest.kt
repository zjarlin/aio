package __PACKAGE_NAME__

import kotlinx.serialization.json.Json
import kotlinx.serialization.json.jsonArray
import kotlinx.serialization.json.jsonObject
import kotlinx.serialization.json.jsonPrimitive
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class PageDefinitionTest {
    @Test
    fun exposesUniqueCounterAndTextPages() {
        val pages = PagePlugin.pages

        assertEquals(2, pages.size)
        assertEquals(pages.size, pages.map(PageDefinition::id).distinct().size)
        assertTrue(pages.first().body is CounterBody)
        assertTrue(pages.last().body is TextBody)
    }

    @Test
    fun encodesTheHostPageDefinitionContract() {
        val pages = Json.parseToJsonElement(PagePlugin.definitionJson()).jsonArray

        assertEquals(__PRIMARY_ID__, pages.first().jsonObject["id"]?.jsonPrimitive?.content)
        assertTrue(pages.first().jsonObject.containsKey("required_permission"))
        assertEquals(
            "counter",
            pages.first().jsonObject["body"]?.jsonObject?.get("kind")?.jsonPrimitive?.content,
        )
        assertEquals(
            "text",
            pages.last().jsonObject["body"]?.jsonObject?.get("kind")?.jsonPrimitive?.content,
        )
    }
}
