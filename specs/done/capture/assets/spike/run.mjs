import http from 'node:http';import {readFileSync,writeFileSync} from 'node:fs';
import {chromium} from 'playwright';
const [w,h,secs]=[+(process.argv[2]||1920),+(process.argv[3]||1080),+(process.argv[4]||10)];
const html=readFileSync(new URL('./page.html',import.meta.url));
const srv=http.createServer((q,r)=>{r.setHeader('content-type','text/html');r.end(html)}).listen(0);
const port=srv.address().port;
const browser=await chromium.launch({channel:'chromium',headless:false,ignoreDefaultArgs:['--mute-audio'],
  args:['--enable-unsafe-webgpu','--enable-features=WebGPU','--auto-accept-this-tab-capture','--autoplay-policy=no-user-gesture-required',
  '--disable-background-timer-throttling','--disable-backgrounding-occluded-windows','--disable-renderer-backgrounding']});
const page=await browser.newPage({viewport:{width:w,height:h},deviceScaleFactor:1});
page.on('console',m=>console.log(m.text()));page.on('pageerror',e=>console.log('ERR',e.message));
await page.goto(`http://localhost:${port}/`);await page.waitForFunction(()=>window.ready);
const t0=Date.now();
const b64=await Promise.race([page.evaluate(s=>window.record(s),secs),new Promise((_,j)=>setTimeout(()=>j(new Error('timeout: prompt not auto-accepted?')),(secs+20)*1000))]);
console.log('recorded in',Date.now()-t0,'ms');
writeFileSync(new URL(`./out-${h}.webm`,import.meta.url),Buffer.from(b64,'base64'));
await browser.close();srv.close();
