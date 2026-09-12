@file:OptIn(kotlin.js.ExperimentalWasmJsInterop::class)

package site.addzero.aio.example.tasks

import kotlin.js.JsString
import kotlin.js.Promise
import kotlin.js.js
import kotlin.js.toJsString
import kotlinx.coroutines.await
import kotlinx.serialization.json.Json

private fun request(method: JsString, path: JsString, body: JsString): Promise<JsString> =
    js("window.aioPlugin.json(method, path, body ? JSON.parse(body) : undefined).then(value => JSON.stringify(value ?? null))")

internal object TaskClient {
    suspend fun page(): TaskPage = Json.decodeFromString(call("GET", "/tasks"))
    suspend fun create(title: String) { call("POST", "/tasks", Json.encodeToString(CreateTask(title))) }
    suspend fun update(task: TaskItem, done: Boolean) {
        call("PATCH", "/tasks/${task.id}", Json.encodeToString(UpdateTask(done)))
    }
    suspend fun remove(task: TaskItem) { call("DELETE", "/tasks/${task.id}") }
    private suspend fun call(method: String, path: String, body: String = ""): String =
        request(method.toJsString(), path.toJsString(), body.toJsString()).await<JsString>().toString()
}
