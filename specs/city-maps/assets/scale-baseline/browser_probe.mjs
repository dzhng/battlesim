import { createRequire } from 'node:module';
const { chromium } = createRequire(new URL('../../web/package.json', import.meta.url))('playwright');
import { createServer } from 'node:http';
import { readFile, readdir, writeFile } from 'node:fs/promises';
import path from 'node:path';
const root=process.cwd();
const fixture=JSON.parse(await readFile(path.join(root,'fixtures/village.json'),'utf8'));
async function docs(dir) {
 let paths=[]; for (const entry of await readdir(dir,{withFileTypes:true})) { const p=path.join(dir,entry.name); if (entry.isDirectory()) paths.push(...await docs(p)); else if(p.endsWith('.json')) paths.push(p); } return paths.sort();
}
fixture.catalog=[]; for(const p of (await docs(path.join(root,'fixtures/units'))).concat(await docs(path.join(root,'fixtures/props'))).sort()) fixture.catalog.push(JSON.parse(await readFile(p,'utf8')));
const server=createServer(async(req,res)=>{try { const name=req.url?.slice(1); if(name==='game_wasm.js'||name==='game_wasm_bg.wasm'){res.setHeader('Content-Type',name.endsWith('.js')?'text/javascript':'application/wasm');res.end(await readFile(path.join(root,'web/src/wasm',name)));}else{res.setHeader('Content-Type','text/html');res.end('<!doctype html><title>S0 resource probe</title>');} } catch(e){res.statusCode=500;res.end(String(e));}});
await new Promise(resolve=>server.listen(0,'127.0.0.1',resolve));
let browser;
try {
 browser=await chromium.launch({headless:true,channel:'chrome',args:['--enable-unsafe-webgpu','--enable-features=WebGPU','--use-angle=metal']});
 const page=await browser.newPage(); await page.goto(`http://127.0.0.1:${server.address().port}`);
 const result=await page.evaluate(async(rules)=>{
  const adapter=await navigator.gpu?.requestAdapter({powerPreference:'high-performance'}); const keys=['maxBufferSize','maxStorageBufferBindingSize','maxTextureDimension2D','maxTextureDimension3D'];
  const adapterLimits=Object.fromEntries(keys.map(k=>[k,adapter?.limits[k]])); let device=null;
  if(adapter) device=await adapter.requestDevice({requiredLimits:{maxBufferSize:adapter.limits.maxBufferSize,maxStorageBufferBindingSize:adapter.limits.maxStorageBufferBindingSize}});
  const gpu={info:adapter?.info?{vendor:adapter.info.vendor,architecture:adapter.info.architecture,description:adapter.info.description,isFallbackAdapter:adapter.isFallbackAdapter}:null,adapterLimits,deviceLimits:Object.fromEntries(keys.map(k=>[k,device?.limits[k]]))};
  const wasm=await import('/game_wasm.js'); const exports=await wasm.default(); const retained=[];const rows=[];
  function stage(name,fn){const before=exports.memory.buffer.byteLength; const t=performance.now();const output=fn();rows.push({stage:name,ms:performance.now()-t,wasm_before_bytes:before,wasm_after_bytes:exports.memory.buffer.byteLength,output_bytes:output?.byteLength??0});return output;}
  const map={size:[1600,1600],height_grid_m:4,slope_cutoff_deg:50};
  const view=stage('main_world',()=>new wasm.WorldView(JSON.stringify(map),JSON.stringify(rules)));
  for(const method of ['terrain_positions','terrain_indices','terrain_triangle_surfaces','foliage']) retained.push(stage(method,()=>view[method]()));
  stage('ground_client_arrays',()=>{const marks=new Uint8Array(1600*1600*4),cleared=new Uint8Array(1600*1600);retained.push(marks,cleared);return {byteLength:marks.byteLength+cleared.byteLength};});
  const workerCode=`self.onmessage=async({data:{rules,map,moduleUrl}})=>{try{const wasm=await import(moduleUrl);const exports=await wasm.default();const rows=[];function stage(name,fn){const before=exports.memory.buffer.byteLength;const t=performance.now();const output=fn();rows.push({stage:name,ms:performance.now()-t,wasm_before_bytes:before,wasm_after_bytes:exports.memory.buffer.byteLength,output_bytes:output?.byteLength??0});return output;}const battle=stage('worker_battle',()=>new wasm.Battle(JSON.stringify({map,rules,units:[]}),1));for(const [name,side]of[['initial','blue'],['steady','blue'],['side_switch','red']]){if(name==='steady')stage('tick',()=>battle.step());stage(name,()=>{const len=battle.publish(side);return new Float32Array(new Float32Array(exports.memory.buffer,battle.publication_ptr(),len));});}const digest=battle.digest();const highwater=exports.memory.buffer.byteLength;battle.free();self.postMessage({rows,digest,wasm_heap_highwater_bytes:highwater});}catch(error){self.postMessage({error:String(error)});}};`;
  const workerUrl=URL.createObjectURL(new Blob([workerCode],{type:'text/javascript'}));const worker=new Worker(workerUrl,{type:'module'});
  let workerResult;try{workerResult=await new Promise((resolve,reject)=>{worker.onmessage=({data})=>data.error?reject(new Error(data.error)):resolve(data);worker.onerror=reject;worker.postMessage({rules,map,moduleUrl:new URL('/game_wasm.js',location.href).href});});}finally{worker.terminate();URL.revokeObjectURL(workerUrl);}
  const sw=view.surface_at(8,8),ne=view.surface_at(1592,1592);
  view.free();device?.destroy();
  return {gpu,rows,sw:Array.from(sw),ne:Array.from(ne),worker:workerResult,retained_js_bytes:retained.reduce((n,a)=>n+a.byteLength,0),wasm_heap_highwater_bytes:exports.memory.buffer.byteLength};
 },fixture);
 await writeFile(path.join(root,'throwaway/city-spike/browser.json'),JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}finally {await browser?.close();await new Promise(resolve=>server.close(resolve));}
