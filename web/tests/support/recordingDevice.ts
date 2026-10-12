// A WebGPU device with no GPU behind it that records every shader module,
// bind group layout and pipeline the renderer creates. TypeGPU resolves its
// WGSL in JavaScript, so the text recorded here is the text the browser hands
// its own device, byte for byte (Vitest runs TypeGPU through the same naming
// plugin Vite does). Everything else is an inert stand-in.

export interface RecordedLayout {
  label: string;
  entries: GPUBindGroupLayoutEntry[];
}

export interface RecordedStage {
  module: number;
  entryPoint: string | undefined;
}

export interface RecordedPipeline {
  kind: "render" | "compute";
  label: string;
  /** Indices into `layouts`, per group; `"auto"` for an implicit layout. */
  layout: number[] | "auto";
  vertex?: RecordedStage & { buffers: (GPUVertexBufferLayout | null)[] };
  fragment?: RecordedStage & { targets: (GPUColorTargetState | null)[] };
  compute?: RecordedStage;
  primitive?: GPUPrimitiveState;
  depthStencil?: GPUDepthStencilState;
  multisample?: GPUMultisampleState;
}

export interface Recording {
  modules: { label: string; code: string }[];
  layouts: RecordedLayout[];
  pipelines: RecordedPipeline[];
}

/** WebGPU's flag namespaces, which Node lacks. */
const FLAGS = {
  GPUBufferUsage: {
    MAP_READ: 1,
    MAP_WRITE: 2,
    COPY_SRC: 4,
    COPY_DST: 8,
    INDEX: 16,
    VERTEX: 32,
    UNIFORM: 64,
    STORAGE: 128,
    INDIRECT: 256,
    QUERY_RESOLVE: 512,
  },
  GPUTextureUsage: {
    COPY_SRC: 1,
    COPY_DST: 2,
    TEXTURE_BINDING: 4,
    STORAGE_BINDING: 8,
    RENDER_ATTACHMENT: 16,
  },
  GPUShaderStage: { VERTEX: 1, FRAGMENT: 2, COMPUTE: 4 },
  GPUMapMode: { READ: 1, WRITE: 2 },
  GPUColorWrite: { RED: 1, GREEN: 2, BLUE: 4, ALPHA: 8, ALL: 15 },
};

/** Install WebGPU's flags and an image loader that hands back blank sheets
 *  of `imageSide` pixels (the effect flipbooks), for a Node test. */
export function installGpuGlobals(imageSide: number) {
  Object.assign(globalThis, FLAGS, {
    fetch: async () => ({ ok: true, blob: async () => ({}) }),
    createImageBitmap: async (_: unknown, options?: { resizeWidth?: number }) => ({
      width: options?.resizeWidth ?? imageSide,
      height: options?.resizeWidth ?? imageSide,
      close() {},
    }),
  });
}

/** Any call answers itself; property reads of symbols and `then` are absent,
 *  so TypeGPU's type guards and `await` see a plain encoder. */
function inert(): object {
  const self: object = new Proxy(
    {},
    { get: (_, key) => (typeof key === "symbol" || key === "then" ? undefined : () => self) },
  );
  return self;
}

export function recordingGpuDevice(): { device: GPUDevice; recording: Recording } {
  const recording: Recording = { modules: [], layouts: [], pipelines: [] };
  const ids = new WeakMap<object, number>();
  const groups = new WeakMap<object, number[]>();
  const handle = (extra: object = {}) => ({ label: "", destroy() {}, ...extra });
  const stage = (s: GPUProgrammableStage): RecordedStage => ({
    module: ids.get(s.module)!,
    entryPoint: s.entryPoint,
  });
  const layoutOf = (layout: GPUPipelineLayout | "auto") =>
    layout === "auto" ? ("auto" as const) : groups.get(layout)!;
  const render = (d: GPURenderPipelineDescriptor) => {
    recording.pipelines.push({
      kind: "render",
      label: d.label ?? "",
      layout: layoutOf(d.layout),
      vertex: { ...stage(d.vertex), buffers: [...(d.vertex.buffers ?? [])] },
      fragment: d.fragment && { ...stage(d.fragment), targets: [...d.fragment.targets] },
      primitive: d.primitive,
      depthStencil: d.depthStencil,
      multisample: d.multisample,
    });
    return handle();
  };
  const compute = (d: GPUComputePipelineDescriptor) => {
    recording.pipelines.push({
      kind: "compute",
      label: d.label ?? "",
      layout: layoutOf(d.layout),
      compute: stage(d.compute),
    });
    return handle();
  };
  const device = {
    features: new Set(["timestamp-query"]),
    limits: new Proxy({}, { get: () => 2 ** 30 }),
    lost: new Promise(() => {}),
    queue: {
      writeBuffer() {},
      writeTexture() {},
      submit() {},
      copyExternalImageToTexture() {},
      onSubmittedWorkDone: async () => {},
    },
    createBuffer(d: GPUBufferDescriptor) {
      const buffer = {
        ...handle(),
        size: d.size,
        usage: d.usage,
        mapState: d.mappedAtCreation ? "mapped" : "unmapped",
        getMappedRange: (offset = 0, size?: number) => new ArrayBuffer(size ?? d.size - offset),
        unmap: () => void (buffer.mapState = "unmapped"),
        mapAsync: async () => void (buffer.mapState = "mapped"),
      };
      return buffer;
    },
    createTexture(d: GPUTextureDescriptor) {
      const size = d.size as number[] | GPUExtent3DDict;
      const [width, height = 1, depthOrArrayLayers = 1] = Array.isArray(size)
        ? size
        : [size.width, size.height, size.depthOrArrayLayers];
      return handle({
        ...d,
        width,
        height,
        depthOrArrayLayers,
        mipLevelCount: d.mipLevelCount ?? 1,
        sampleCount: d.sampleCount ?? 1,
        dimension: d.dimension ?? "2d",
        createView: () => handle(),
      });
    },
    createSampler: () => handle(),
    createShaderModule(d: GPUShaderModuleDescriptor) {
      const module = handle({ getCompilationInfo: async () => ({ messages: [] }) });
      ids.set(module, recording.modules.length);
      recording.modules.push({ label: d.label ?? "", code: d.code });
      return module;
    },
    createBindGroupLayout(d: GPUBindGroupLayoutDescriptor) {
      const layout = handle();
      ids.set(layout, recording.layouts.length);
      recording.layouts.push({ label: d.label ?? "", entries: [...d.entries] });
      return layout;
    },
    createPipelineLayout(d: GPUPipelineLayoutDescriptor) {
      const layout = handle();
      groups.set(
        layout,
        [...d.bindGroupLayouts].map((g) => ids.get(g!)!),
      );
      return layout;
    },
    createBindGroup: () => handle(),
    createRenderPipeline: render,
    createComputePipeline: compute,
    createRenderPipelineAsync: async (d: GPURenderPipelineDescriptor) => render(d),
    createComputePipelineAsync: async (d: GPUComputePipelineDescriptor) => compute(d),
    createCommandEncoder: inert,
    createQuerySet: () => handle(),
    pushErrorScope() {},
    popErrorScope: async () => null,
    addEventListener() {},
    removeEventListener() {},
    destroy() {},
  };
  return { device: device as unknown as GPUDevice, recording };
}
