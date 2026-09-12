import { createServer } from 'node:http';
import { increment, type CounterResponse } from '../shared/counter';

const server = createServer(async (request, response) => {
  response.setHeader('content-type', 'application/json');
  const path = new URL(request.url || '/', 'http://localhost').pathname;
  try {
    if (path === '/health') { response.end(JSON.stringify({ok:true})); return; }
    if (path === '/aio/definition') {
      response.end(JSON.stringify([{id:'__NAME__',label:"__TITLE__",scene:{id:'community',label:'社区插件'},menu_path:[],body:{kind:'frontend',entry:'index.html'}}])); return;
    }
    if (path !== '/counter' || request.method !== 'POST') { response.writeHead(404).end(); return; }
    let body = '';
    for await (const chunk of request) { body += chunk; if (body.length > 4096) { response.writeHead(413).end(); return; } }
    const input = JSON.parse(body);
    const result: CounterResponse = {value:increment(input.value),tenant_id:String(request.headers['x-aio-tenant-id'] || '')};
    response.end(JSON.stringify(result));
  } catch (error) { response.writeHead(400).end(JSON.stringify({error:error instanceof Error ? error.message : 'Invalid request'})); }
});
server.listen(Number(process.env.AIO_PLUGIN_PORT || 8080),'0.0.0.0');
process.on('SIGTERM',()=>server.close());
