/** A lit surface's light, before the air between it and the eye. Pure WGSL;
 *  the environment owner binds `standardPbr` to it, and its `shade` puts the
 *  air over it (`applyAerial`). */
export const lightEnvironmentWgsl = `(base:vec3f,emissive:vec3f,roughness:f32,geomRoughness:f32,metal:f32,ao:f32,normalWorld:vec3f,worldPosition:vec3f,shadow:f32,eye:vec3f,sunDirection:vec3f,sunRadiance:vec3f,environmentIntensity:vec3f,maxMip:f32,pmrem:texture_2d<f32>,dfg:texture_2d<f32>,linear:sampler)->vec3f {
 return standardPbr(base,emissive,roughness,geomRoughness,metal,ao,normalize(normalWorld),normalize(eye-worldPosition),sunDirection,sunRadiance,shadow,environmentIntensity,pmrem,linear,maxMip,dfg,linear);
 }`;
