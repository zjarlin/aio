package __PACKAGE_NAME__

import java.nio.file.Files
import java.nio.file.Path

fun main(args: Array<String>) {
    val output = Path.of(args.firstOrNull() ?: "dist/pages.json").toAbsolutePath().normalize()
    output.parent?.let(Files::createDirectories)
    Files.writeString(output, PagePlugin.definitionJson() + "\n")
    println(output)
}
