package site.addzero.aio.example.counter

import kotlinx.serialization.Serializable

@Serializable
data class CounterResponse(val count: Long, val tenantId: String)
