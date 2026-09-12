const { readFileSync } = require('node:fs');
const { parseEnv } = require('node:util');
const { Client } = require('/opt/aio-delivery/ops/node_modules/pg');

(async()=>{
  const settings=parseEnv(readFileSync('/opt/aio-idea/runtime.env','utf8'));
  const client=new Client({connectionString:settings.AIO_DATABASE_URL});
  await client.connect();
  try {
    const sources=await client.query('SELECT git,branch,desired_sha,enabled,updated_at FROM delivery_sources ORDER BY updated_at DESC LIMIT 20');
    const jobs=await client.query('SELECT id,git,source_revision,state,package_revision,left(error,1600) AS error,created_at,updated_at FROM delivery_jobs ORDER BY id DESC LIMIT 30');
    const rollouts=await client.query('SELECT tenant_id,source_id,revision,state,left(error,600) AS error,updated_at FROM delivery_rollouts ORDER BY updated_at DESC LIMIT 30');
    console.log(JSON.stringify({sources:sources.rows,jobs:jobs.rows,rollouts:rollouts.rows},null,2));
  }finally{await client.end();}
})().catch(error=>{console.error(error.message);process.exitCode=1;});
