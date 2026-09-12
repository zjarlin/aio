(() => {
  "use strict";
  for (const name of ["compileStreaming", "instantiateStreaming"]) {
    const compile = globalThis.WebAssembly?.[name];
    if (typeof compile !== "function") continue;
    WebAssembly[name] = async (source, ...options) => {
      const response = await source;
      // 隔离页面的流式编译会反压公网下载；先排空副本，保留原响应的原生校验。
      if (response instanceof Response && response.ok && !response.bodyUsed) {
        await response.clone().arrayBuffer();
      }
      return compile.call(WebAssembly, response, ...options);
    };
  }
})();
