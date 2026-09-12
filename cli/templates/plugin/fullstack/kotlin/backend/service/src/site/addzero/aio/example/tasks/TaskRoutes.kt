package site.addzero.aio.example.tasks

import io.ktor.http.HttpStatusCode
import io.ktor.server.application.ApplicationCall
import io.ktor.server.request.receive
import io.ktor.server.response.respond
import io.ktor.server.routing.Route
import io.ktor.server.routing.delete
import io.ktor.server.routing.get
import io.ktor.server.routing.patch
import io.ktor.server.routing.post

private fun ApplicationCall.tenant(): String = requireNotNull(request.headers["x-aio-tenant-id"]) { "Missing tenant context" }
private fun ApplicationCall.user(): String = requireNotNull(request.headers["x-aio-user-id"]) { "Missing user context" }
private fun ApplicationCall.taskId(): Long = requireNotNull(parameters["id"]?.toLongOrNull()) { "Invalid task id" }

internal fun Route.taskRoutes(store: TaskStore) {
    get("/tasks") {
        call.respond(store.page(call.tenant(), call.user(), call.request.queryParameters["search"].orEmpty()))
    }
    post("/tasks") { call.respond(HttpStatusCode.Created, store.create(call.tenant(), call.receive())) }
    patch("/tasks/{id}") {
        val item = store.update(call.tenant(), call.taskId(), call.receive())
        if (item == null) call.respond(HttpStatusCode.NotFound, ServiceError("Task not found")) else call.respond(item)
    }
    delete("/tasks/{id}") {
        if (store.remove(call.tenant(), call.taskId())) call.respond(HttpStatusCode.NoContent)
        else call.respond(HttpStatusCode.NotFound, ServiceError("Task not found"))
    }
    get("/counter") { call.respond(store.counter(call.tenant(), false)) }
    post("/counter") { call.respond(store.counter(call.tenant(), true)) }
}
