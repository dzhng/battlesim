import {mkdir,writeFile,copyFile,readdir} from 'node:fs/promises';
import {spawnSync} from 'node:child_process';
import {fileURLToPath} from 'node:url';
const base=fileURLToPath(new URL('./',import.meta.url));
const out=base+'review/';await mkdir(out,{recursive:true});
const ff=(args)=>{const r=spawnSync('/opt/homebrew/bin/ffmpeg',['-hide_banner','-loglevel','error','-y','-threads','2','-filter_threads','1',...args],{encoding:'utf8'});if(r.status)throw new Error(r.stderr);};
const stories=[['floor3',[0,10,17,20,29]],['collapse',[0,220,312,313,333]],['gutted',[0,220,312,313,333]],['floor3-modular-wall',[0,7,43,48,59]],['collapse-no-spotter',[0,100,200,300,399]]];
const metadata=[];
for(const [name,frames] of stories){
 const dir=base+name+'/';const files=(await readdir(dir)).filter(f=>/^frame-\d+.png$/.test(f)).sort();
 const sequence=['standing.png',...files,'final.png'];
 const concat=out+name+'-sequence.txt';await writeFile(concat,sequence.map((f,i)=>`file '${dir+f}'\nduration ${i===0?.7:i===sequence.length-1?.8:.1}\n`).join('')+`file '${dir}final.png'\n`);
 ff(['-f','concat','-safe','0','-i',concat,'-vf','fps=10,scale=1280:720:flags=lanczos,split[s0][s1];[s0]palettegen=max_colors=128:stats_mode=diff[p];[s1][p]paletteuse=dither=bayer:bayer_scale=3','-loop','0',out+name+'.gif']);
 const chosen=['standing.png',...frames.map(i=>`frame-${String(i).padStart(3,'0')}.png`),'final.png'];
 const sheet=out+name+'-sheet-input.txt';await writeFile(sheet,chosen.map(f=>`file '${dir+f}'\n`).join(''));
 ff(['-f','concat','-safe','0','-i',sheet,'-vf','scale=640:360,tile=3x3:margin=8:padding=8:color=0x202020','-frames:v','1',out+name+'-contact.png']);
 for(const f of chosen)await copyFile(dir+f,out+name+'-'+f);
 metadata.push({name,gif:out+name+'.gif',fullFrames:dir,frames:files.length,contactFrameOrder:chosen,gifTiming:'First/full endpoint holds0.7/0.8s; otherframes0.1s. Floor3 simstep3ticks; destruction step15ticks, settled tail6ticks.'});
 console.log(name,files.length);
}
const crops=[
 ['floor3-seats-2x','floor3/frame-017.png',[548,260,340,260],2],
 ['floor3-path-2x','floor3/frame-017.png',[548,260,740,550],2],
 ['floor3-path-second-2x','floor3/frame-020.png',[548,260,740,550],2],
 ['collapse-standing-2x','collapse/standing.png',[680,315,590,460],2],
 ['collapse-before-2x','collapse/frame-312.png',[680,315,670,500],2],
 ['collapse-transition-2x','collapse/frame-313.png',[680,315,670,500],2],
 ['collapse-final-2x','collapse/final.png',[680,315,590,500],2],
 ['gutted-standing-2x','gutted/standing.png',[650,100,650,680],2],
 ['gutted-before-2x','gutted/frame-312.png',[650,100,700,715],2],
 ['gutted-transition-2x','gutted/frame-313.png',[650,100,700,715],2],
 ['gutted-final-2x','gutted/final.png',[650,100,650,680],2],
 ['modular-path-2x','floor3-modular-wall/frame-048.png',[548,260,780,550],2],
 ['no-spotter-final-2x','collapse-no-spotter/final.png',[680,315,590,460],2],
];
for(const [label,source,[x,y,w,h],scale] of crops)ff(['-i',base+source,'-vf',`crop=${w}:${h}:${x}:${y},scale=${w*scale}:${h*scale}:flags=neighbor`,'-frames:v','1',out+label+'.png']);
await copyFile(base+'failed-internal-bays/collapse-startup-failed.png',out+'invalid-internal-bays-full.png');
await copyFile(base+'floor3-startup-failed.png',out+'earlier-startup-full.png');
await writeFile(out+'index.json',JSON.stringify({stories:metadata,crops:crops.map(([label,source,bounds,scale])=>({label,source,bounds,scale}))},null,2)+'\n');
