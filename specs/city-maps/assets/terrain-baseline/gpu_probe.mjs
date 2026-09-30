import {createRequire} from 'node:module';
import {readFile,writeFile,readdir} from 'node:fs/promises';
import path from 'node:path';
const cwd=process.cwd();
const require=createRequire(path.join(cwd,'web/package.json'));
const {chromium}=require('playwright');
const {createServer}=await import(require.resolve('vite'));
const fixture=JSON.parse(await readFile('fixtures/village.json','utf8'));
async function documents(dir){let out=[];for(const e of await readdir(dir,{withFileTypes:true})){const p=path.join(dir,e.name);if(e.isDirectory())out.push(...await documents(p));else if(p.endsWith('.json'))out.push(p);}return out.sort();}
fixture.catalog=[];for(const p of (await documents('fixtures/units')).concat(await documents('fixtures/props')).sort())fixture.catalog.push(JSON.parse(await readFile(p,'utf8')));
const server=await createServer({root:path.join(cwd,'web'),configFile:path.join(cwd,'web/vite.config.ts'),server:{port:0,host:'127.0.0.1'},logLevel:'error'});
let browser;
try {
 await server.listen();
 const port=server.httpServer.address().port;
 browser=await chromium.launch({channel:'chrome',headless:true,args:['--enable-unsafe-webgpu','--enable-features=WebGPU','--use-angle=metal']});
 const page=await browser.newPage();const errors=[];page.on('pageerror',e=>errors.push(String(e)));page.on('console',m=>{if(m.type()==='error'){errors.push(m.text());console.error(m.text());}});
 await page.goto(`http://127.0.0.1:${port}/`);
 const result=await page.evaluate(async({cwd,rules})=>{
  const helperUrl='/@fs/'+cwd+'/packages/battle-renderer/src/frame/terrainHeights.ts';
  const helperSource=await (await fetch(helperUrl)).text();
  const typegpuUrl=helperSource.match(/from ["']([^"']*typegpu[^"']*)["']/)[1];
  const [{tgpu,d},{GpuRegistry},{createTerrainHeights,terrainSample},{triangleRuleHeight},{groundHeight},wasm]=await Promise.all([
   import(typegpuUrl),
   import('/@fs/'+cwd+'/packages/battle-renderer/src/frame/registry.ts'),
   import('/@fs/'+cwd+'/packages/battle-renderer/src/frame/terrainHeights.ts'),
   import('/@fs/'+cwd+'/packages/battle-renderer/src/frame/triangleRule.ts'),
   import('/@fs/'+cwd+'/packages/battle-renderer/src/terrain/terrainGrid.ts'),
   import('/src/wasm/game_wasm.js'),
  ]);
  const module=await wasm.default();
  const adapter=await navigator.gpu.requestAdapter({powerPreference:'high-performance'});
  if(adapter.info.vendor!=='apple'||adapter.isFallbackAdapter)throw new Error('Not Apple Metal');
  const device=await adapter.requestDevice();const validation=[];device.addEventListener('uncapturederror',e=>validation.push(e.error.message));
  const registry=new GpuRegistry(device),root=tgpu.initFromDevice({device});registry.adopt(()=>root.destroy());
  const owner=createTerrainHeights(registry);
  const L=tgpu.bindGroupLayout({params:{uniform:d.vec4f},heights:{storage:n=>d.arrayOf(d.u32,n),access:'readonly'},points:{storage:n=>d.arrayOf(d.vec2f,n),access:'readonly'},out:{storage:n=>d.arrayOf(d.f32,n),access:'mutable'}});
  const heightProbeSample=terrainSample(tgpu.fn([d.u32],d.u32)(`(at:u32)->u32 {return L.$.heights[at];}`).$uses({L}));
  const kernel=tgpu.computeFn({in:{gid:d.builtin.globalInvocationId},workgroupSize:[64]})(`{
   let P=L.$.params;if(gid.x>=u32(P.w)){return;}
   let q=L.$.points[gid.x];let fx=clamp(q.x/P.z,0.0,P.x-1.0);let fy=clamp(q.y/P.z,0.0,P.y-1.0);
   let i=min(u32(fx),u32(P.x)-2u);let j=min(u32(fy),u32(P.y)-2u);
   L.$.out[gid.x]=triangleRuleHeight(heightProbeSample(i,j),heightProbeSample(i+1u,j),heightProbeSample(i,j+1u),heightProbeSample(i+1u,j+1u),fx-f32(i),fy-f32(j));
  }`).$uses({L,heightProbeSample,triangleRuleHeight});
  kernel.$name("terrainHeightProbe");
  const pipeline=root.createComputePipeline({compute:kernel});try{await pipeline.initAsync();}catch(e){console.error(validation.join("\n"));registry.release();device.destroy();throw e;}
  const params=registry.own(root.createBuffer(d.vec4f).$usage('uniform'));
  const rows=[];
  try{
   for(const [size,river]of[[12000,false],[15000,false],[18000,false],[18000,true]]){
    const map={size:[size,size],height_grid_m:4,slope_cutoff_deg:35,...(river?{water:[{rect:[size/2-16,0,32,size],surface_z:0,bed_z:-2}]}:{})};
    const view=new wasm.WorldView(JSON.stringify(map),JSON.stringify(rules));
    const grid={...JSON.parse(view.terrain_grid()),pageIds:view.terrain_page_ids(),heights:view.terrain_heights()};
    const points=[];
    for(let k=0;k<1024;k++)points.push(size*((k*0.618034)%1),size*((k*0.754877+0.5)%1));
    for(const x of[0,63.9,64,64.1,size/2-20,size/2-16,size/2-0.25,size/2,size/2+16,size/2+17,size])for(const y of[0,63.9,64,64.1,size/2,size])points.push(x,y);
    const input=Float32Array.from(points),n=input.length/2;
    const scope=registry.scope();
    try{
     const heights=owner.forGrid(grid);if(owner.forGrid({...grid})!==heights)throw new Error('Duplicate shared height buffer');
     const positions=view.terrain_positions(),indices=view.terrain_indices();
     const source=scope.buffer({label:'height-points',size:input.byteLength,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_DST});
     const out=scope.buffer({label:'height-out',size:n*4,usage:GPUBufferUsage.STORAGE|GPUBufferUsage.COPY_SRC});
     const read=scope.buffer({label:'height-readback',size:n*4,usage:GPUBufferUsage.MAP_READ|GPUBufferUsage.COPY_DST});
     device.queue.writeBuffer(source,0,input);params.write(d.vec4f(grid.nx,grid.ny,grid.spacing,n));
     const encoder=root['~unstable'].createCommandEncoder();pipeline.with(root.createBindGroup(L,{params,heights,points:source,out})).with(encoder).dispatchWorkgroups(Math.ceil(n/64));
     root.unwrap(encoder).copyBufferToBuffer(out,0,read,0,n*4);device.queue.submit([encoder.finish()]);await read.mapAsync(GPUMapMode.READ);const got=new Float32Array(read.getMappedRange());
     let nativeError=0,cpuError=0;
     for(let k=0;k<n;k++){const x=input[k*2],y=input[k*2+1];nativeError=Math.max(nativeError,Math.abs(got[k]-view.height_at(x,y)));cpuError=Math.max(cpuError,Math.abs(got[k]-groundHeight(grid,x,y)));}
     if(nativeError>1e-3||cpuError>1e-3)throw new Error(JSON.stringify({size,river,nativeError,cpuError}));read.unmap();
     rows.push({size,river,queries:n,nativeError,cpuError,sampleBytes:grid.pageIds.byteLength+grid.heights.byteLength,meshExportBytes:positions.byteLength+indices.byteLength,packedDrawBytes:indices.length*40,gpuHeightBytes:heights.size,wasmHighWater:module.memory.buffer.byteLength});
    }finally{scope.release();view.free();}
   }
  }finally{registry.release();device.destroy();}
  if(validation.length)throw new Error(validation.join('\n'));
  return{gpu:{vendor:adapter.info.vendor,architecture:adapter.info.architecture,maxStorageBufferBindingSize:device.limits.maxStorageBufferBindingSize,maxStorageBuffersPerShaderStage:device.limits.maxStorageBuffersPerShaderStage},rows,finalGpuCounts:registry.stats(),validation};
 },{cwd,rules:fixture});
 if(errors.length)throw new Error(errors.join('\n'));
 await writeFile('throwaway/city-spike/terrain-gpu.json',JSON.stringify(result,null,2)+'\n');console.log(JSON.stringify(result));
}finally{await browser?.close();await server.close();}
