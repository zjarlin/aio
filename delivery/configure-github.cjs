const fs = require('node:fs');
if(process.getuid()!==0)throw new Error('必须由 root 配置发现凭据');
const token=fs.readFileSync(0,'utf8').trim();
if(!/^[a-zA-Z0-9_]{20,256}$/.test(token))throw new Error('GitHub 服务凭据格式无效');
const path='/opt/aio-idea/runtime.env';
const previous=fs.readFileSync(path,'utf8').split('\n').filter(line=>!line.startsWith('AIO_DELIVERY_GITHUB_TOKEN='));
fs.writeFileSync(path,previous.join('\n').trimEnd()+`\nAIO_DELIVERY_GITHUB_TOKEN=${token}\n`);
console.log('发现凭据已写入宿主配置；构建容器不接收此凭据。');
