struct VertexOut {
  @builtin(position) position: vec4f,
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

struct VertexIn {
  @location(0) position: vec3f,
  @location(1) uv: vec2f,
  @location(2) face: f32,
}

@vertex fn vertex(_arg_0: VertexIn) -> VertexOut {
  return VertexOut(vec4f(_arg_0.position, 1f), _arg_0.uv, _arg_0.face);
}

fn equirectUv(direction: vec3f) -> vec2f {
  return vec2f(atan2(direction.z, direction.x) * 1.5915494309189535e-1f + 0.5,
    asin(clamp(direction.y, -1.0, 1.0)) * 3.1830988618379069e-1f + 0.5);
}

fn cubeDirection(uv: vec2f, face: f32) -> vec3f {
  let p=uv*2.0-vec2f(1);
  if(face==0){return vec3f(1,p.y,p.x);}
  if(face==1){return vec3f(-p.x,1,-p.y);}
  if(face==2){return vec3f(-p.x,p.y,1);}
  if(face==3){return vec3f(-1,p.y,-p.x);}
  if(face==4){return vec3f(-p.x,-1,p.y);}
  return vec3f(p.x,p.y,-1);
}

fn bakeAlgorithm(uv: vec2f, face: f32, source: texture_2d<f32>, linear: sampler) -> vec4f {
      return textureSampleLevel(source,linear,equirectUv(normalize(cubeDirection(uv,face))),0);
    }

@group(0) @binding(0) var source: texture_2d<f32>;

@group(0) @binding(1) var linear: sampler;

fn bakeShade(uv: vec2f, face: f32) -> vec4f {
  return bakeAlgorithm(uv, face, source, linear);
}

struct FragmentIn {
  @location(0) uv: vec2f,
  @location(1) face: f32,
}

@fragment fn fragment(_arg_0: FragmentIn) -> @location(0) vec4f {
  return bakeShade(_arg_0.uv, _arg_0.face);
}