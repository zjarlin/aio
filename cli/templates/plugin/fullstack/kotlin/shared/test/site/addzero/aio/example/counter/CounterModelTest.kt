package site.addzero.aio.example.counter

import kotlinx.serialization.encodeToString
import kotlinx.serialization.json.Json
import kotlin.test.Test
import kotlin.test.assertEquals

class CounterModelTest {
    @Test
    fun preservesCountAndTenantAcrossTheWire() {
        val model = CounterResponse(123456789L, "tenant-a")
        assertEquals(model, Json.decodeFromString<CounterResponse>(Json.encodeToString(model)))
    }
}
