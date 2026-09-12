const fs = require('node:fs');
const { randomBytes } = require('node:crypto');
const { parseEnv } = require('node:util');

if (process.getuid() !== 0) throw new Error('必须由 root 配置构建服务');
const images = process.argv.slice(2);
if (images.length !== 3 || images.some(value => !/^sha256:[a-f0-9]{64}$/.test(value))) {
  throw new Error('用法: node configure.cjs <Rust image ID> <Kotlin image ID> <TypeScript image ID>');
}
const hostFile = '/opt/aio-idea/runtime.env';
const current = fs.readFileSync(hostFile, 'utf8');
const token = parseEnv(current).AIO_DELIVERY_TOKEN || randomBytes(32).toString('hex');
if (!/^[a-zA-Z0-9_-]{32,256}$/.test(token)) throw new Error('已有交付凭据格式无效');
const servicePath = '/opt/aio-delivery/git/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin';
const retained = current.split('\n').filter(line => !/^(AIO_DELIVERY_(TOKEN|OWNER)|PATH)=/.test(line));
fs.writeFileSync(hostFile, retained.join('\n').trimEnd() + `\nAIO_DELIVERY_TOKEN=${token}\nAIO_DELIVERY_OWNER=zjarlin\nPATH=${servicePath}\n`);
fs.mkdirSync('/opt/aio-delivery/bin', { recursive: true });
const values = {
  AIO_DELIVERY_TOKEN: token,
  AIO_DELIVERY_URL: 'http://127.0.0.1:3080',
  AIO_CLI: '/opt/aio-delivery/bin/aio',
  PATH: servicePath,
  AIO_BUILD_IMAGE_RUST: images[0],
  AIO_BUILD_IMAGE_KOTLIN: images[1],
  AIO_BUILD_IMAGE_TYPESCRIPT: images[2],
};
fs.writeFileSync('/opt/aio-delivery/worker.env', Object.entries(values).map(([key,value])=>`${key}=${value}`).join('\n')+'\n', { mode: 0o600 });
fs.chmodSync('/opt/aio-delivery/worker.env',0o600);
console.log('已配置宿主与独立构建服务，凭据仅保存在服务端。');
