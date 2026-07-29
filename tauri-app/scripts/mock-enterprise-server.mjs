import http from 'http';

const PORT = 9224;

const servers = {
  clientConfig(req, res) {
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      mcp_servers: [
        { id: 'builtin-1c-search', name: '1C:Поиск', enabled: true, transport: 'http' },
        { id: 'builtin-1c-help', name: '1С:Справка', enabled: true, transport: 'http' },
        { id: 'builtin-1c-naparnik', name: '1C:Напарник', enabled: true, transport: 'http' },
        { id: 'builtin-1c-metadata', name: '1C:Метаданные', enabled: true, transport: 'http' },
      ],
      bsl_remote_url: 'ws://localhost:8025/lsp',
      active_llm_profile: '',
      llm: { active_provider_id: '', providers: {} },
      theme: 'dark',
      extra_settings: null,
    }));
  },

  updaterCheck(req, res) {
    const url = new URL(req.url, `http://localhost:${PORT}`);
    const version = url.searchParams.get('version') || '0.0.0';
    res.writeHead(200, { 'Content-Type': 'application/json' });
    res.end(JSON.stringify({
      available: false,
      version: null,
      url: null,
      changelog: null,
    }));
  },

  mcpProxy(req, res) {
    // Echo back the request as if it were an MCP tool call
    let body = '';
    req.on('data', chunk => body += chunk);
    req.on('end', () => {
      try {
        const parsed = JSON.parse(body);
        const result = {
          jsonrpc: '2.0',
          id: parsed.id || 1,
          result: {
            content: [{ type: 'text', text: `[MOCK] ${parsed.method} called on server` }],
            isError: false,
          },
        };
        res.writeHead(200, { 'Content-Type': 'application/json' });
        res.end(JSON.stringify(result));
      } catch (e) {
        res.writeHead(400);
        res.end(JSON.stringify({ error: e.message }));
      }
    });
  },
};

const routes = {
  'GET:/api/client/config': servers.clientConfig,
  'GET:/api/updater/check': servers.updaterCheck,
};

const server = http.createServer((req, res) => {
  const key = `${req.method}:${req.url.split('?')[0]}`;
  const handler = routes[key];

  if (handler) {
    handler(req, res);
  } else if (req.url.startsWith('/api/mcp/')) {
    servers.mcpProxy(req, res);
  } else {
    res.writeHead(404);
    res.end(JSON.stringify({ error: 'Not found' }));
  }
});

server.listen(PORT, '0.0.0.0', () => {
  console.log(`[MOCK] Enterprise server listening on http://0.0.0.0:${PORT}`);
  console.log(`[MOCK] Endpoints:`);
  console.log(`       GET /api/client/config`);
  console.log(`       GET /api/updater/check?version=X`);
  console.log(`       POST /api/mcp/{server_id}`);
});
