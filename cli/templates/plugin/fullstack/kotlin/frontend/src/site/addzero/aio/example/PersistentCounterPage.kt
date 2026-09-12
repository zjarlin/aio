package site.addzero.aio.example

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.padding
import androidx.compose.material3.Button
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.unit.dp
import androidx.compose.ui.graphics.Color
import kotlinx.coroutines.launch

@Composable
internal fun PersistentCounterPage() {
    var count by remember { mutableStateOf(0L) }
    var tenant by remember { mutableStateOf("") }
    var pending by remember { mutableStateOf(false) }
    var error by remember { mutableStateOf<String?>(null) }
    val scope = rememberCoroutineScope()
    LaunchedEffect(Unit) {
        pending = true
        try {
            val result = loadCounter()
            count = result.count
            tenant = result.tenantId
        } catch (cause: Throwable) {
            error = cause.message ?: "请求失败"
        } finally {
            pending = false
        }
    }
    MaterialTheme(colorScheme = lightColorScheme(
        primary = Color(0xFF167451), onPrimary = Color.White,
        background = Color.White, surface = Color.White, onSurface = Color(0xFF202124),
    )) {
        Surface(modifier = Modifier.fillMaxSize()) {
            Column(
                modifier = Modifier.padding(24.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp),
            ) {
                Text("KMP Counter", style = MaterialTheme.typography.titleLarge)
                Text(count.toString(), style = MaterialTheme.typography.displayMedium)
                Button(
                    enabled = !pending,
                    onClick = {
                        pending = true
                        error = null
                        scope.launch {
                            try {
                                val result = loadCounter(increment = true)
                                count = result.count
                                tenant = result.tenantId
                            } catch (cause: Throwable) {
                                error = cause.message ?: "请求失败"
                            } finally {
                                pending = false
                            }
                        }
                    },
                ) { Text("+1") }
                if (tenant.isNotEmpty()) Text(tenant, style = MaterialTheme.typography.bodySmall)
                error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            }
        }
    }
}
