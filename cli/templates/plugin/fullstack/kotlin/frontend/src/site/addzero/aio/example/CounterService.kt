@file:OptIn(kotlin.js.ExperimentalWasmJsInterop::class)

package site.addzero.aio.example

import kotlin.js.js
import kotlin.js.JsString
import kotlin.js.Promise
import kotlin.js.toJsString
import kotlinx.coroutines.await
import kotlinx.serialization.json.Json
import site.addzero.aio.example.counter.CounterResponse

private fun requestHost(method: JsString): Promise<JsString> =
    js("window.aioPlugin.json(method, '/counter').then(result => JSON.stringify(result))")

internal suspend fun loadCounter(increment: Boolean = false): CounterResponse {
    val response = requestHost((if (increment) "POST" else "GET").toJsString()).await<JsString>()
    return Json.decodeFromString(response.toString())
}
