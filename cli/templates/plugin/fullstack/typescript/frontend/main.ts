import { increment, type CounterResponse } from '../shared/counter';

declare global { interface Window { aioPlugin: { json(method:string,path:string,value:unknown):Promise<CounterResponse> } } }
let count = 0;
let backend = 0;
const output = document.querySelector<HTMLOutputElement>('#count')!;
document.querySelector<HTMLButtonElement>('#increment')!.onclick = () => { output.value = String(count = increment(count)); };
const request = document.querySelector<HTMLButtonElement>('#request')!;
request.onclick = async () => {
  request.disabled = true;
  try {
    const result = await window.aioPlugin.json('POST','/counter',{value:backend});
    document.querySelector<HTMLOutputElement>('#backend')!.value = String(backend = result.value);
    document.querySelector('#result')!.textContent = result.tenant_id;
  } catch (error) { document.querySelector('#result')!.textContent = String(error); }
  finally { request.disabled = false; }
};
