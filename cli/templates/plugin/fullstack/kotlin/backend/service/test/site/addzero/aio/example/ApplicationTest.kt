package site.addzero.aio.example

import io.ktor.client.request.*
import io.ktor.client.statement.bodyAsText
import io.ktor.http.*
import io.ktor.server.testing.testApplication
import kotlinx.serialization.json.Json
import site.addzero.aio.example.tasks.TaskItem
import site.addzero.aio.example.tasks.TaskPage
import kotlin.test.Test
import kotlin.test.assertEquals
import kotlin.test.assertTrue

class ApplicationTest {
    private fun HttpRequestBuilder.context(tenant: String = "alpha") {
        header("x-aio-tenant-id", tenant)
        header("x-aio-user-id", "tester")
        contentType(ContentType.Application.Json)
    }

    @Test
    fun tasksHaveRealHttpMutationsAndTenantIsolation() = testApplication {
        application { application() }
        assertEquals(HttpStatusCode.OK, client.get("/health").status)
        assertTrue(client.get("/aio/definition").bodyAsText().contains("index.html"))
        assertEquals(HttpStatusCode.BadRequest, client.get("/tasks").status)
        val initial: TaskPage = Json.decodeFromString(client.get("/tasks") { context() }.bodyAsText())
        assertEquals(3, initial.total)
        assertEquals("alpha", initial.tenantId)
        assertEquals("tester", initial.userId)
        assertEquals(HttpStatusCode.BadRequest, client.post("/tasks") { context(); setBody("{\"title\":\"  \"}") }.status)
        assertEquals(HttpStatusCode.BadRequest, client.post("/tasks") { context(); setBody("not json") }.status)
        val created = client.post("/tasks") { context(); setBody("{\"title\":\"Fullstack verification\"}") }
        assertEquals(HttpStatusCode.Created, created.status)
        val task: TaskItem = Json.decodeFromString(created.bodyAsText())
        assertEquals("Fullstack verification", task.title)
        val other: TaskPage = Json.decodeFromString(client.get("/tasks") { context("beta") }.bodyAsText())
        assertEquals(3, other.total)
        val updated: TaskItem = Json.decodeFromString(client.patch("/tasks/${task.id}") { context(); setBody("{\"done\":true}") }.bodyAsText())
        assertTrue(updated.done)
        val filtered: TaskPage = Json.decodeFromString(client.get("/tasks?search=Fullstack") { context() }.bodyAsText())
        assertEquals(listOf(updated), filtered.items)
        assertEquals(HttpStatusCode.NoContent, client.delete("/tasks/${task.id}") { context() }.status)
        assertEquals(HttpStatusCode.NotFound, client.delete("/tasks/${task.id}") { context() }.status)
    }
}
