#!/usr/bin/env node

import net from "node:net";

const options = parseOptions(process.argv.slice(2));
const listenHost = requiredOption(options, "listen-host");
const listenPort = parsePort(requiredOption(options, "listen-port"), "listen-port");
const proxyHost = requiredOption(options, "proxy-host");
const proxyPort = parsePort(requiredOption(options, "proxy-port"), "proxy-port");
const targetHost = requiredOption(options, "target-host");
const targetPort = parsePort(requiredOption(options, "target-port"), "target-port");

const server = net.createServer((client) => {
  client.pause();
  forwardClient(client).catch((error) => {
    console.error(`PostgreSQL SOCKS 转发失败: ${error.message}`);
    client.destroy();
  });
});

server.on("error", (error) => {
  console.error(`PostgreSQL SOCKS relay 监听失败: ${error.message}`);
  process.exitCode = 1;
});

server.listen(listenPort, listenHost, () => {
  console.log(`PostgreSQL SOCKS relay 已监听 ${listenHost}:${listenPort}`);
});

process.on("SIGTERM", () => server.close());
process.on("SIGINT", () => server.close());

async function forwardClient(client) {
  const proxy = await connect(proxyHost, proxyPort);
  const reader = createSocketReader(proxy);
  await write(proxy, Buffer.from([0x05, 0x01, 0x00]));

  const greeting = await reader.read(2);
  if (greeting[0] !== 0x05 || greeting[1] !== 0x00) {
    throw new Error("SOCKS5 代理不支持无认证连接");
  }

  const targetAddress = parseIpv4(targetHost);
  const request = Buffer.from([
    0x05,
    0x01,
    0x00,
    0x01,
    ...targetAddress,
    targetPort >> 8,
    targetPort & 0xff,
  ]);
  await write(proxy, request);

  const response = await reader.read(4);
  if (response[0] !== 0x05 || response[1] !== 0x00) {
    throw new Error(`SOCKS5 连接目标失败，状态码 ${response[1]}`);
  }

  await consumeSocksAddress(reader, response[3]);

  const remainder = reader.detach();
  proxy.pause();
  client.pipe(proxy);
  proxy.pipe(client);
  if (remainder.length > 0) {
    client.write(remainder);
  }
  client.resume();
}

function parseOptions(args) {
  const options = new Map();

  for (let index = 0; index < args.length; index += 2) {
    const key = args[index];
    const value = args[index + 1];
    if (!key?.startsWith("--") || value === undefined) {
      throw new Error("参数必须以 --名称 值 的形式提供");
    }
    options.set(key.slice(2), value);
  }

  return options;
}

function requiredOption(options, key) {
  const value = options.get(key);
  if (!value) {
    throw new Error(`缺少 --${key}`);
  }
  return value;
}

function parsePort(value, key) {
  const port = Number.parseInt(value, 10);
  if (!Number.isInteger(port) || port < 1 || port > 65535) {
    throw new Error(`--${key} 必须是有效端口`);
  }
  return port;
}

function parseIpv4(value) {
  const segments = value.split(".").map((segment) => Number.parseInt(segment, 10));
  if (segments.length !== 4 || segments.some((segment) => !Number.isInteger(segment) || segment < 0 || segment > 255)) {
    throw new Error("当前 SOCKS relay 只支持 IPv4 PostgreSQL 目标");
  }
  return segments;
}

async function consumeSocksAddress(reader, addressType) {
  if (addressType === 0x01) {
    await reader.read(6);
    return;
  }
  if (addressType === 0x04) {
    await reader.read(18);
    return;
  }
  if (addressType === 0x03) {
    const domainLength = (await reader.read(1))[0];
    await reader.read(domainLength + 2);
    return;
  }
  throw new Error(`SOCKS5 返回未知地址类型 ${addressType}`);
}

function connect(host, port) {
  return new Promise((resolve, reject) => {
    const socket = net.connect({ host, port });
    socket.once("connect", () => resolve(socket));
    socket.once("error", reject);
  });
}

function write(socket, payload) {
  return new Promise((resolve, reject) => {
    socket.write(payload, (error) => (error ? reject(error) : resolve()));
  });
}

function createSocketReader(socket) {
  let buffer = Buffer.alloc(0);
  let pending;

  const onData = (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    resolvePending();
  };
  const onError = (error) => rejectPending(error);
  const onClose = () => rejectPending(new Error("SOCKS5 代理在握手完成前关闭连接"));

  socket.on("data", onData);
  socket.once("error", onError);
  socket.once("close", onClose);

  return {
    read(expectedLength) {
      if (buffer.length >= expectedLength) {
        return Promise.resolve(take(expectedLength));
      }

      return new Promise((resolve, reject) => {
        pending = { expectedLength, resolve, reject };
      });
    },
    detach() {
      socket.off("data", onData);
      socket.off("error", onError);
      socket.off("close", onClose);
      return take(buffer.length);
    },
  };

  function resolvePending() {
    if (!pending || buffer.length < pending.expectedLength) {
      return;
    }

    const { expectedLength, resolve } = pending;
    pending = undefined;
    resolve(take(expectedLength));
  }

  function rejectPending(error) {
    if (!pending) {
      return;
    }

    const { reject } = pending;
    pending = undefined;
    reject(error);
  }

  function take(length) {
    const value = buffer.subarray(0, length);
    buffer = buffer.subarray(length);
    return value;
  }
}
