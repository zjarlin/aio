package site.addzero.aio.plugin.kmpservice

import com.sun.net.httpserver.HttpExchange
import com.sun.net.httpserver.HttpServer
import java.net.InetSocketAddress
import java.nio.charset.StandardCharsets
import java.util.concurrent.Executors

private const val DEFAULT_PORT = 8080

fun main() {
    val port = System.getenv("AIO_PLUGIN_PORT")?.toIntOrNull() ?: DEFAULT_PORT
    val server = HttpServer.create(InetSocketAddress("0.0.0.0", port), 0)
    server.executor = Executors.newVirtualThreadPerTaskExecutor()
    server.createContext("/") { exchange -> handle(exchange) }
    Runtime.getRuntime().addShutdownHook(Thread { server.stop(1) })
    server.start()
    println("AIO Kotlin process plugin listening on $port")
}

private fun handle(exchange: HttpExchange) {
    try {
        when {
            exchange.requestMethod == "GET" && exchange.requestURI.path == "/health" ->
                exchange.respond(200, "text/plain; charset=utf-8", "ok")

            exchange.requestMethod == "GET" && exchange.requestURI.path == "/aio/definition" ->
                exchange.respond(
                    200,
                    "application/json; charset=utf-8",
                    KmpProcessPlugin.definitionJson(),
                )

            exchange.requestMethod == "POST" && exchange.requestURI.path == "/aio/action" ->
                exchange.respond(
                    200,
                    "application/json; charset=utf-8",
                    KmpProcessPlugin.actionResultJson(
                        exchange.requestBody.bufferedReader(StandardCharsets.UTF_8).use { it.readText() },
                    ),
                )

            exchange.requestURI.path == "/echo" -> exchange.echo()
            else -> exchange.respond(404, "application/json; charset=utf-8", "{\"error\":\"not found\"}")
        }
    } catch (error: Exception) {
        exchange.respond(
            500,
            "application/json; charset=utf-8",
            ErrorResponse(error.message ?: "internal error").encode(),
        )
    } finally {
        exchange.close()
    }
}

private fun HttpExchange.echo() {
    val context = RequestContext(
        tenantId = requestHeaders.getFirst("x-aio-tenant-id") ?: "",
        userId = requestHeaders.getFirst("x-aio-user-id") ?: "",
    )
    val response = EchoResponse(
        method = requestMethod,
        path = requestURI.path,
        query = requestURI.rawQuery,
        body = requestBody.bufferedReader(StandardCharsets.UTF_8).use { it.readText() },
        tenantId = context.tenantId,
        userId = context.userId,
    )
    respond(200, "application/json; charset=utf-8", response.encode())
}

private fun HttpExchange.respond(status: Int, contentType: String, body: String) {
    val bytes = body.toByteArray(StandardCharsets.UTF_8)
    responseHeaders.set("Content-Type", contentType)
    sendResponseHeaders(status, bytes.size.toLong())
    responseBody.use { it.write(bytes) }
}
