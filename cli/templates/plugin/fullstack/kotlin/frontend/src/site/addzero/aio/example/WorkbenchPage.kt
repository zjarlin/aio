@file:OptIn(kotlin.js.ExperimentalWasmJsInterop::class)

package site.addzero.aio.example

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Tab
import androidx.compose.material3.PrimaryTabRow
import androidx.compose.material3.Text
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import site.addzero.aio.example.tasks.TasksPage
import kotlin.js.js

private fun selectedCounter(): Boolean = js("location.hash === '#counter'")
private fun selectRoute(counter: Boolean): Unit = js("location.hash = counter ? '#counter' : '#tasks'")

@Composable
internal fun WorkbenchPage() {
    var selected by remember { mutableStateOf(if (selectedCounter()) 1 else 0) }
    var count by remember { mutableStateOf(0L) }
    MaterialTheme(colorScheme = lightColorScheme(
        primary = Color(0xFF167451), onPrimary = Color.White,
        background = Color.White, surface = Color.White, onSurface = Color(0xFF202124),
        secondaryContainer = Color(0xFFE7EFFC), onSecondaryContainer = Color(0xFF254C85),
    )) {
        Surface(modifier = Modifier.fillMaxSize()) {
            Column {
                PrimaryTabRow(selectedTabIndex = selected) {
                    listOf("Tasks", "Counter").forEachIndexed { index, label ->
                        Tab(selected = selected == index, onClick = { selected = index; selectRoute(index == 1) }, text = { Text(label) })
                    }
                }
                when (selected) {
                    0 -> TasksPage()
                    else -> CounterPage(count = count, onIncrement = { count++ })
                }
            }
        }
    }
}
