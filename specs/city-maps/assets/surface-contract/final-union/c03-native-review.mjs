import { readFileSync } from "node:fs";
import {vec2} from "../../web/node_modules/math/dist/index.js";
import {segment2} from "../../web/node_modules/math/dist/shapes/index.js";
import { initSync, WorldView } from "../../web/src/wasm/game_wasm.js";
import { VILLAGE_RULES } from "../../apps/battle-lab/src/scenarios.ts";
initSync({module:readFileSync(new URL("../../web/src/wasm/game_wasm_bg.wasm",import.meta.url))});
const poly=(ring,kind="road")=>({kind,shape:{kind:"polygon",ring}});
const box=(x0,y0,x1,y1)=>[[x0,y0],[x1,y0],[x1,y1],[x0,y1]];
function probe(name,surfaces,point,expectedDistance,expectedPerimeter,allRoad=false){
 const view=new WorldView(JSON.stringify({size:[256,256],fog_cell_m:8,height_grid_m:4,slope_cutoff_deg:35,surfaces}),JSON.stringify(VILLAGE_RULES));
 try {const rows=view.surface_boundaries();let perimeter=0,nearest=Infinity;const closest=vec2.create();
  for(let o=0;o<rows.length;o+=5){const a=[rows[o],rows[o+1]],b=[rows[o+2],rows[o+3]];perimeter+=vec2.distance(a,b);segment2.closestPoint(closest,point,a,b);nearest=Math.min(nearest,vec2.distance(point,closest));if(allRoad&&rows[o+4]!==1)throw Error(`${name}: wrong Road priority`);}
  if(nearest!==expectedDistance||perimeter!==expectedPerimeter)throw Error(`${name}: ${JSON.stringify({nearest,perimeter})}`);
  console.log(JSON.stringify({name,nearest,perimeter,boundaryRows:rows.length/5,allRoad}));
 }finally{view.free();}
}
probe("opposite winding touching",[poly(box(2,2,12,12)),poly(box(12,2,22,12).reverse())],[12,7],5,60);
probe("coincident Road wins Sidewalk first",[poly(box(2,2,12,12),"sidewalk"),poly(box(2,2,12,12).reverse())],[7,7],5,40,true);
probe("partial shared edge clips only touching interval",[poly(box(2,2,12,12)),poly(box(12,4,22,10))],[12,7],3,60);
probe("outside indexed map extent still compiles union",[poly(box(300,300,310,310)),poly(box(310,300,320,310))],[310,305],5,60);
