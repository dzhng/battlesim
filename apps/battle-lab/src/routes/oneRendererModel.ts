export interface ModelWorkload {
  vertices: Float32Array;
  indices: Uint32Array;
  instances: Float32Array;
  texture: Uint8Array;
  textureWidth: number;
  textureHeight: number;
}

const MODEL_SHADER = `
struct Camera { view_proj: mat4x4<f32> };
@group(0) @binding(0) var<uniform> camera: Camera;
@group(1) @binding(0) var albedo: texture_2d<f32>;
@group(1) @binding(1) var linear: sampler;
struct Out { @builtin(position) position: vec4<f32>, @location(0) color: vec4<f32>, @location(1) uv: vec2<f32> };
@vertex fn vertex(
  @location(0) position: vec3<f32>, @location(1) color: vec4<f32>,
  @location(2) uv: vec2<f32>, @location(3) placement: vec4<f32>, @location(4) tint: vec4<f32>,
) -> Out {
  let c = cos(placement.w); let s = sin(placement.w);
  let world = vec3<f32>(c * position.x - s * position.y + placement.x,
    s * position.x + c * position.y + placement.y, position.z + placement.z);
  var out: Out; out.position = camera.view_proj * vec4<f32>(world, 1.0);
  out.color = color * tint; out.uv = uv; return out;
}
@fragment fn fragment(in: Out) -> @location(0) vec4<f32> { return in.color * textureSample(albedo, linear, in.uv); }
`;

export function modelPipeline(
  device: GPUDevice,
  format: GPUTextureFormat,
  withDepth = false,
): GPURenderPipeline {
  const module = device.createShaderModule({
    label: "one-renderer-authored-model",
    code: MODEL_SHADER,
  });
  return device.createRenderPipeline({
    label: "one-renderer-authored-model-pipeline",
    layout: "auto",
    vertex: {
      module,
      entryPoint: "vertex",
      buffers: [
        {
          arrayStride: 36,
          attributes: [
            { format: "float32x3", offset: 0, shaderLocation: 0 },
            { format: "float32x4", offset: 12, shaderLocation: 1 },
            { format: "float32x2", offset: 28, shaderLocation: 2 },
          ],
        },
        {
          arrayStride: 32,
          stepMode: "instance",
          attributes: [
            { format: "float32x4", offset: 0, shaderLocation: 3 },
            { format: "float32x4", offset: 16, shaderLocation: 4 },
          ],
        },
      ],
    },
    fragment: { module, entryPoint: "fragment", targets: [{ format }] },
    depthStencil: withDepth
      ? { format: "depth32float", depthWriteEnabled: true, depthCompare: "greater" }
      : undefined,
  });
}

export function modelEncoder(
  device: GPUDevice,
  terrainPipeline: GPURenderPipeline,
  model: GPURenderPipeline,
  terrain: Float32Array,
  workload: ModelWorkload,
  camera: Float32Array,
  width: number,
  height: number,
) {
  const terrainBuffer = device.createBuffer({
    size: terrain.byteLength,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(terrainBuffer, 0, terrain);
  const vertex = device.createBuffer({
    size: workload.vertices.byteLength,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(vertex, 0, workload.vertices);
  const index = device.createBuffer({
    size: workload.indices.byteLength,
    usage: GPUBufferUsage.INDEX | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(index, 0, workload.indices);
  const instance = device.createBuffer({
    size: workload.instances.byteLength,
    usage: GPUBufferUsage.VERTEX | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(instance, 0, workload.instances);
  const uniform = device.createBuffer({
    size: 64,
    usage: GPUBufferUsage.UNIFORM | GPUBufferUsage.COPY_DST,
  });
  device.queue.writeBuffer(uniform, 0, camera.buffer, camera.byteOffset, 64);
  const albedo = device.createTexture({
    label: "one-renderer-authored-model-albedo",
    size: [workload.textureWidth, workload.textureHeight, 1],
    format: "rgba8unorm-srgb",
    usage: GPUTextureUsage.TEXTURE_BINDING | GPUTextureUsage.COPY_DST,
  });
  device.queue.writeTexture(
    { texture: albedo },
    workload.texture,
    { bytesPerRow: workload.textureWidth * 4, rowsPerImage: workload.textureHeight },
    [workload.textureWidth, workload.textureHeight, 1],
  );
  const sampler = device.createSampler({
    magFilter: "linear",
    minFilter: "linear",
    mipmapFilter: "linear",
  });
  const depth = device.createTexture({
    label: "one-renderer-authored-model-depth",
    size: [width, height, 1],
    format: "depth32float",
    usage: GPUTextureUsage.RENDER_ATTACHMENT,
  });
  const depthView = depth.createView();
  const bind = device.createBindGroup({
    layout: terrainPipeline.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniform } }],
  });
  const modelBind = device.createBindGroup({
    layout: model.getBindGroupLayout(0),
    entries: [{ binding: 0, resource: { buffer: uniform } }],
  });
  const materialBind = device.createBindGroup({
    layout: model.getBindGroupLayout(1),
    entries: [
      { binding: 0, resource: albedo.createView() },
      { binding: 1, resource: sampler },
    ],
  });
  return (context: GPUCanvasContext) => {
    const view = context.getCurrentTexture().createView();
    const encoder = device.createCommandEncoder({ label: "one-renderer-authored-model-frame" });
    const pass = encoder.beginRenderPass({
      colorAttachments: [
        {
          view,
          clearValue: { r: 0.02, g: 0.03, b: 0.05, a: 1 },
          loadOp: "clear",
          storeOp: "store",
        },
      ],
      depthStencilAttachment: {
        view: depthView,
        depthClearValue: 0,
        depthLoadOp: "clear",
        depthStoreOp: "store",
      },
    });
    pass.setPipeline(terrainPipeline);
    pass.setBindGroup(0, bind);
    pass.setVertexBuffer(0, terrainBuffer);
    pass.draw(terrain.length / 10);
    pass.end();
    const modelPass = encoder.beginRenderPass({
      colorAttachments: [{ view, loadOp: "load", storeOp: "store" }],
      depthStencilAttachment: {
        view: depthView,
        depthLoadOp: "load",
        depthStoreOp: "store",
      },
    });
    modelPass.setPipeline(model);
    modelPass.setBindGroup(0, modelBind);
    modelPass.setBindGroup(1, materialBind);
    modelPass.setVertexBuffer(0, vertex);
    modelPass.setVertexBuffer(1, instance);
    modelPass.setIndexBuffer(index, "uint32");
    modelPass.drawIndexed(workload.indices.length, workload.instances.length / 8);
    modelPass.end();
    device.queue.submit([encoder.finish()]);
  };
}
