/** Pure environment composition; resource/binding ownership belongs to each backend. */
export type WorldSurfaceDiagnostic = "albedo" | "normal" | "roughness" | "ao";
export function environmentFunctions(diagnostic?: WorldSurfaceDiagnostic, aerial = true) {
  const shade =
    diagnostic === "ao"
      ? "return vec4f(vec3f(ao),1);"
      : diagnostic === "albedo"
        ? "return vec4f(base,1);"
        : diagnostic === "normal"
          ? "return vec4f(normalize(normalWorld)*0.5+vec3f(0.5),1);"
          : diagnostic === "roughness"
            ? "return vec4f(min(max(roughness,0.0525)+geomRoughness,1.0),geomRoughness,metal,1);"
            : `let lit=standardPbr(base,emissive,roughness,geomRoughness,metal,ao,normalize(normalWorld),normalize(eye-worldPosition),sunDirection,sunRadiance,shadow,environmentIntensity,pmrem,linear,maxMip,dfg,linear);
 return ${aerial ? "applyAerial(vec4f(lit,1),worldPosition,eye,observer,sky,linear)" : "vec4f(lit,1)"};`;
  return {
    geometryRoughnessFromView: `(normalView:vec3f)->f32 {
  let n=normalize(normalView);let d=max(abs(dpdx(n)),abs(dpdy(n)));return max(max(d.x,d.y),d.z);
 }`,
    geometryRoughnessWithView: `(normalWorld:vec3f,worldToView:mat4x4f)->f32 {
  return geometryRoughnessFromView((worldToView*vec4f(normalWorld,0)).xyz);
 }`,
    shadeEnvironment: `(base:vec3f,emissive:vec3f,roughness:f32,geomRoughness:f32,metal:f32,ao:f32,normalWorld:vec3f,worldPosition:vec3f,shadow:f32,eye:vec3f,observer:vec3f,sunDirection:vec3f,sunRadiance:vec3f,environmentIntensity:vec3f,maxMip:f32,sky:texture_2d<f32>,pmrem:texture_2d<f32>,dfg:texture_2d<f32>,linear:sampler)->vec4f {
 ${shade}
 }`,
  };
}
