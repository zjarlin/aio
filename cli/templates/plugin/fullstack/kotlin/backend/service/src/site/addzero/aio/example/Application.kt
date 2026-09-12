package site.addzero.aio.example

import io.ktor.http.HttpStatusCode
import io.ktor.serialization.kotlinx.json.json
import io.ktor.server.application.Application
import io.ktor.server.application.install
import io.ktor.server.cio.CIO
import io.ktor.server.engine.embeddedServer
import io.ktor.server.plugins.BadRequestException
import io.ktor.server.plugins.contentnegotiation.ContentNegotiation
import io.ktor.server.plugins.statuspages.StatusPages
import io.ktor.server.response.respond
import io.ktor.server.response.respondText
import io.ktor.server.routing.get
import io.ktor.server.routing.routing
import io.ktor.http.ContentType
import site.addzero.aio.example.tasks.ServiceError
import site.addzero.aio.example.tasks.TaskStore
import site.addzero.aio.example.tasks.taskRoutes

fun main() {
    val port = System.getenv("AIO_PLUGIN_PORT")?.toInt() ?: 8080
    embeddedServer(CIO, host = "0.0.0.0", port = port) { application() }.start(wait = true)
}

internal fun Application.application() {
    install(ContentNegotiation) { json() }
    install(StatusPages) {
        exception<IllegalArgumentException> { call, cause ->
            call.respond(HttpStatusCode.BadRequest, ServiceError(cause.message ?: "Invalid request"))
        }
        exception<BadRequestException> { call, _ ->
            call.respond(HttpStatusCode.BadRequest, ServiceError("Invalid JSON request"))
        }
    }
    val store = TaskStore()
    routing {
        get("/health") { call.respondText("ok") }
        get("/aio/definition") {
            val definition = checkNotNull(javaClass.getResourceAsStream("/pages.json"))
                .bufferedReader().use { it.readText() }
            call.respondText(definition, ContentType.Application.Json)
        }
        taskRoutes(store)
    }
}
