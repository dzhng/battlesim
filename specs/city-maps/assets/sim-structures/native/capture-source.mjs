// Temporary existing-route staging; restores every source/fixture in finally.
import { readFile, writeFile, mkdir } from 'node:fs/promises';
import { createRequire } from 'node:module';
import { createHash } from 'node:crypto';
import { startServer, WEBGPU_FLAGS } from '../../web/scene.mjs';
import { advance as labAdvance, obs as labObs, presented as labPresented } from '../../web/scenes/_lab.mjs';
const require = createRequire(new URL('../../web/package.json', import.meta.url));
const { chromium } = require('playwright');
const bounded = async (promise,label) => {
 let timer;
 try { return await Promise.race([promise,new Promise((_,reject)=>{
  timer=setTimeout(()=>reject(new Error(`${label}: timed out after 30 seconds`)),30000);
 })]); } finally {clearTimeout(timer);}
};
const advance=(...args)=>bounded(labAdvance(...args),'advance');
const obs=(...args)=>bounded(labObs(...args),'observation');
const presented=(...args)=>bounded(labPresented(...args),'presentation');
const root = new URL('../../',import.meta.url);
const fixturePath = new URL('fixtures/garrison-lab.json',root);
const routePath = new URL('apps/battle-lab/src/routes/garrison.tsx',root);
const [fixtureOriginal,routeOriginal] = await Promise.all([readFile(fixturePath),readFile(routePath,'utf8')]);
await writeFile(new URL('original-garrison-lab.json',import.meta.url),fixtureOriginal);
await writeFile(new URL('original-garrison.tsx',import.meta.url),routeOriginal);
const wasmSha256=createHash('sha256').update(await readFile(new URL('web/src/wasm/game_wasm_bg.wasm',root))).digest('hex');
let browser,server;
const reports = [];
try {
 for (const moment of process.argv.slice(2).length ? process.argv.slice(2) : ['floor3','collapse','gutted']) {
  const stage = JSON.parse(await readFile(new URL(`${moment}-map.json`,import.meta.url),'utf8'));
  const units = [{side:'blue',kind:'rifle',position:[320,250],engagement:'return_fire_only'},
   {side:'red',kind:'tank',position:[580,250],yaw:Math.PI,engagement:'return_fire_only'}];
  if(moment!=='floor3')units.push({side:'red',kind:'rifle',position:[475,250],engagement:'return_fire_only'});
  const scripted = [{tick:1,side:'blue',order:{kind:'garrison',units:[0],building:0}}];
  if (moment !== 'floor3') scripted.push({tick:1500,side:'red',order:{kind:'set_engagement',units:[1],policy:'fire_at_will'}});
  const start=routeOriginal.indexOf('const staged = labScenario('),end=routeOriginal.indexOf('// Keep direct fire',start);
  const staged = `const staged = labScenario(garrisonMap, ${JSON.stringify(units)}, [], ${JSON.stringify(scripted)});\n`;
  await writeFile(fixturePath,JSON.stringify(stage,null,2)+'\n');
  await writeFile(routePath,routeOriginal.slice(0,start)+staged+routeOriginal.slice(end));
  server ??= await startServer();
  browser ??= await chromium.launch({channel:'chromium',args:WEBGPU_FLAGS});
  const context=await browser.newContext({viewport:{width:1920,height:1080},deviceScaleFactor:1});
  const page=await context.newPage();
  const errors=[];
  page.on('pageerror',e=>errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error')errors.push(m.text())});
  await page.goto(server.url+'/lab/garrison');
  try {
   await page.waitForFunction(()=>window.__lab?.ready||window.__lab?.error,undefined,{timeout:30000});
   const error=await page.evaluate(()=>window.__lab.error);if(error)throw new Error(error);
   await page.waitForFunction(()=>window.__lab.route?.tick()>3,undefined,{timeout:30000});
  }
  catch(error) {
   await writeFile(new URL(`${moment}-startup-failed.png`,import.meta.url),await page.screenshot({timeout:30000}));
   const state=await page.evaluate(()=>({labError:window.__lab?.error,text:document.body.innerText}));
   await writeFile(new URL(`${moment}-startup-error.json`,import.meta.url),JSON.stringify({errors,state,error:String(error)},null,2));
   console.log(JSON.stringify({errors,state}));throw error;
  }
  await page.evaluate(()=>window.__lab.route.pause());
  let o=await obs(page);
  for(let k=0;k<1000&&o.own[0]?.garrison?.phase!=='inside';k+=15){await advance(page,15);o=await obs(page)}
  if(o.own[0]?.garrison?.phase!=='inside')throw new Error(`${moment}: entry failed at ${o.tick}`);
  const entryObservedTick=o.tick;
  await page.evaluate(v=>window.__lab.setCamera({...window.__lab.camera(),...v}),{
   target:moment==='floor3'?[402,250,4]:[360,250,0],distance:moment==='floor3'?145:140,pitch:.85,yaw:-.85});
  const dir=new URL(`${moment}/`,import.meta.url);await mkdir(dir,{recursive:true});
  const capture=async name=>{await presented(page);await page.evaluate(()=>window.__lab.frame());await writeFile(new URL(name,dir),await page.screenshot({timeout:30000}))};
  await capture('standing.png');
  const snapshots=[];let terminal=false;let highShots=0;let crossingShots=0;
  if(moment==='floor3')await page.evaluate(()=>window.__lab.route.command({kind:'attack',units:[0],target:{kind:'ground',point:[580,250,0]}}));
  else await advance(page,Math.max(0,1450-o.tick));
  for(let frame=0;frame<(moment==='floor3'?30:400);frame++){
   await advance(page,moment==='floor3'?3:15);o=await obs(page);snapshots.push(o);
   for(const p of o.projectiles??[])if(p.own){
    highShots+=Number(p.path?.[0]?.[2]>6);
    crossingShots+=Number(p.path?.some(q=>q[0]>423&&q[2]>5));
   }
   const state=o.knownProps?.filter(p=>p.kind===(moment==='gutted'?'gutted':'ruin'))??[];
   terminal=moment!=='floor3'&&state.length>=2;
   await capture(`frame-${String(frame).padStart(3,'0')}.png`);
   if(terminal&&frame>30){for(let f=1;f<=20;f++){await advance(page,6);snapshots.push(await obs(page));await capture(`frame-${String(frame+f).padStart(3,'0')}.png`)}break}
  }
  await capture('final.png');o=await obs(page);
  const report={moment,scope:'Explicit systems prototype massing; no source/art acceptance',sourceFixture:stage,stagedUnits:units,scriptedOrders:scripted,
   scenarioSha256:createHash('sha256').update(JSON.stringify({stage,units,scripted})).digest('hex'),adapter:await page.evaluate(()=>window.__lab.adapter),
   browser:browser.version(),wasmSha256,integratedSimCommit:process.env.CAPTURE_SIM_COMMIT??'unspecified',entryObservedTick,firstFrameTick:snapshots[0]?.tick,finalTick:o.tick,highShots,crossingShots,terminal,
   own:o.own,knownProps:o.knownProps,errors};
  reports.push(report);await writeFile(new URL('report.json',dir),JSON.stringify(report,null,2)+'\n');
  await writeFile(new URL('observations.json',dir),JSON.stringify(snapshots)+'\n');
  console.log(JSON.stringify({moment,highShots,crossingShots,terminal,finalTick:o.tick,errors}));
  await context.close();
  if(moment==='floor3' ? highShots===0||crossingShots===0 : !terminal)throw new Error(`${moment}: expected native postcondition not reached`);
 }
}catch(error){
 await writeFile(new URL('producer-failure.json',import.meta.url),JSON.stringify({error:String(error),stack:error.stack,completed:reports.map(r=>r.moment)},null,2)+'\n');
 throw error;
}finally{
 await writeFile(fixturePath,fixtureOriginal);await writeFile(routePath,routeOriginal);
 await browser?.close();await server?.close();
 await writeFile(new URL('report.json',import.meta.url),JSON.stringify(reports,null,2)+'\n');
}
