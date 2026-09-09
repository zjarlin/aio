package __PACKAGE_NAME__.component

import __PACKAGE_NAME__.bindings.PageRootFunctions
import __PACKAGE_NAME__.contract.KmpComponentContract

internal object PageRootFunctionsExportsImpl : PageRootFunctions.Exports {
    override fun definition(): String = KmpComponentContract.definition()

    override fun handle(request: String): String = KmpComponentContract.handle(request)
}
