// WGSL owner: shadow.

// @block shadow_SunCascade
struct SunCascade {
  matrix: mat4x4f,
  bias: vec4f,
  interval: vec4f,
}

// @block shadow_SunShadow
struct SunShadow {
  cascades: array<SunCascade, SHADOW_CASCADES>,
  control: vec4f,
}

// @block shadow_inheritedSunShadow
fn inheritedSunShadow(environment: Environment, sunShadow: SunShadow, sunDepth: texture_depth_2d_array, sunCompare: sampler_comparison, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let viewZ=(environment.worldToView*vec4f(world,1.0)).z;
  // Breaks are fractions of the receiver range [control.z, control.x]
  // (split near, capped far); the receiver normalises its depth the same way.
  // The range is sampled on a ray grid, so a sliver of map can sit just
  // nearer than it: the first cascade takes that too.
  let linearDepth=max((-viewZ-sunShadow.control.z)/(sunShadow.control.x-sunShadow.control.z),0.0);
  var shade=1.0;
  {
    let interval=sunShadow.cascades[0].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,0,sunShadow.cascades[0].matrix,sunShadow.cascades[0].bias,world,normal,pixel))*select(1.0,ratio,linearDepth>centre);
    }
  }
  {
    let interval=sunShadow.cascades[1].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,1,sunShadow.cascades[1].matrix,sunShadow.cascades[1].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[2].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y+margin*0.5;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,2,sunShadow.cascades[2].matrix,sunShadow.cascades[2].bias,world,normal,pixel))*ratio;
    }
  }
  {
    let interval=sunShadow.cascades[3].interval;
    let centre=(interval.x+interval.y)*0.5;
    let closest=select(interval.y,interval.x,linearDepth<centre);
    let margin=0.25*closest*closest;
    let low=interval.x-margin*0.5;
    let high=interval.y;
    if (linearDepth>=low&&linearDepth<=high) {
      // A zero-width band is wholly inside, not a 0/0 sample. Unreachable for
      // the shipped split; the guard is what keeps a NaN out of the fragment.
      let ratio=select(1.0,clamp(min(linearDepth-low,high-linearDepth)/margin,0.0,1.0),margin>0.0);
      shade-=(1.0-shadowVisibility(sunDepth,sunCompare,3,sunShadow.cascades[3].matrix,sunShadow.cascades[3].bias,world,normal,pixel))*ratio;
    }
  }
  return shade;
}

// @block shadow_sampleSunShadow_alternate
fn sampleSunShadow(world: vec3f, normal_1: vec3f, pixel: vec2f) -> f32 {
  return inheritedSunShadow(data, sun, sunDepth, sunCompare, world, normal_1, pixel);
}

// @block shadow_shadowPcf
fn shadowPcf(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, uv: vec2f, z: f32, pixel: vec2f, radius: f32) -> f32 {
  let phi=fract(52.9829189*fract(dot(pixel,vec2f(0.06711056,0.00583715))))*6.28318530718;
  let cphi=cos(phi);let sphi=sin(phi);
  let scale=radius/f32(textureDimensions(depth).x);
  var visibility=0.0;
  let b0=vec2f(0.31622776601683794,0);let o0=vec2f(b0.x*cphi-b0.y*sphi,b0.x*sphi+b0.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o0,layer,z);let b1=vec2f(-0.40387356772577226,0.36998127154283905);let o1=vec2f(b1.x*cphi-b1.y*sphi,b1.x*sphi+b1.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o1,layer,z);let b2=vec2f(0.06181932279751069,-0.7043992982171811);let o2=vec2f(b2.x*cphi-b2.y*sphi,b2.x*sphi+b2.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o2,layer,z);let b3=vec2f(0.5090564735709379,0.6639740256331726);let o3=vec2f(b3.x*cphi-b3.y*sphi,b3.x*sphi+b3.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o3,layer,z);let b4=vec2f(-0.934181236883857,-0.16524350714671632);let o4=vec2f(b4.x*cphi-b4.y*sphi,b4.x*sphi+b4.y*cphi)*scale;visibility+=textureSampleCompareLevel(depth,compare,uv+o4,layer,z);
  return visibility*0.2;
}

// @block shadow_shadowVisibility
fn shadowVisibility(depth: texture_depth_2d_array, compare: sampler_comparison, layer: i32, matrix: mat4x4f, settings: vec4f, world: vec3f, normal: vec3f, pixel: vec2f) -> f32 {
  let clip=matrix*vec4f(world+normal*settings.y,1);
  let coord=clip.xyz/clip.w;
  let uv=vec2f(coord.x*0.5+0.5,0.5-coord.y*0.5);
  let z=coord.z-settings.x;
  let inside=all(uv>=vec2f(0))&&all(uv<=vec2f(1))&&z>=0&&z<=1;
  if (!inside) { return 1.0; }
  return shadowPcf(depth,compare,layer,uv,z,pixel,settings.z);
}

// @block shadow_sun_primary
@group(GROUP) @binding(6) var<uniform> sun: SunShadow;
