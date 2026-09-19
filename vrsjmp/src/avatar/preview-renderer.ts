import { signalFragment } from "./signal-field";
import { advancePreview } from "./preview-frame";
import { frameTask } from "./frame-clock";
import { neutralInk } from "./preview-colors";
import { appearanceUniforms } from "./appearance-uniforms";
import { ExpressionPlayer, type Uniforms } from "./expression-player";

type Preview = {
  canvas: HTMLCanvasElement;
  player: ExpressionPlayer;
  dark: boolean;
  resolution: number;
  live: boolean;
  visible: boolean;
  dirty: boolean;
  paused: boolean;
  trackTime: boolean;
  onFrame?: (player: ExpressionPlayer, paint: boolean) => void;
};
const previews = new Set<Preview>();
let render: ((p: Preview) => void) | undefined;
let driver: ReturnType<typeof frameTask> | undefined;
function createRenderer() {
  const source = document.createElement("canvas");
  const gl = source.getContext("webgl2", {
    alpha: true,
    premultipliedAlpha: true,
    antialias: false,
    preserveDrawingBuffer: true,
  });
  if (!gl) throw new Error("WebGL2 unavailable");
  const shader = (type: number, code: string) => {
    const s = gl.createShader(type)!;
    gl.shaderSource(s, code);
    gl.compileShader(s);
    if (!gl.getShaderParameter(s, gl.COMPILE_STATUS))
      throw new Error(gl.getShaderInfoLog(s) ?? "Shader compile failed");
    return s;
  };
  const program = gl.createProgram()!;
  gl.attachShader(
    program,
    shader(
      gl.VERTEX_SHADER,
      "#version 300 es\nin vec2 position;void main(){gl_Position=vec4(position,0.,1.);}",
    ),
  );
  gl.attachShader(program, shader(gl.FRAGMENT_SHADER, signalFragment));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS))
    throw new Error(gl.getProgramInfoLog(program) ?? "Shader link failed");
  gl.useProgram(program);
  const buffer = gl.createBuffer();
  gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
  gl.bufferData(
    gl.ARRAY_BUFFER,
    new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
    gl.STATIC_DRAW,
  );
  const position = gl.getAttribLocation(program, "position");
  gl.enableVertexAttribArray(position);
  gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
  const uniforms = new Map<string, { location: WebGLUniformLocation | null; type: number }>();
  for (let i = 0; i < gl.getProgramParameter(program, gl.ACTIVE_UNIFORMS); i++) {
    const info = gl.getActiveUniform(program, i)!;
    uniforms.set(info.name, {
      location: gl.getUniformLocation(program, info.name),
      type: info.type,
    });
  }
  const waveTexture = gl.createTexture();
  gl.activeTexture(gl.TEXTURE0);
  gl.bindTexture(gl.TEXTURE_2D, waveTexture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.NEAREST);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.R32F, 128, 8, 0, gl.RED, gl.FLOAT, null);
  gl.uniform1i(gl.getUniformLocation(program, "u_waveCurves"), 0);
  const wavePixels = new Float32Array(128 * 8);
  function upload(values: Uniforms) {
    for (const [key, v] of Object.entries(values)) {
      const uniform = uniforms.get(key);
      if (!uniform) continue;
      const { location, type } = uniform;
      if (typeof v === "number") gl!.uniform1f(location, v);
      else if (type === gl!.FLOAT_VEC4) gl!.uniform4fv(location, v);
      else if (type === gl!.FLOAT_VEC3) gl!.uniform3fv(location, v);
      else if (type === gl!.FLOAT_VEC2) gl!.uniform2fv(location, v);
      else gl!.uniform1fv(location, v);
    }
  }
  return (p: Preview) => {
    const size = p.resolution + 80;
    if (source.width !== size) {
      source.width = size;
      source.height = size;
      gl.viewport(0, 0, size, size);
    }
    const config = p.player.setup.appearance;
    const dynamic = p.player.frame();
    wavePixels.set(dynamic.waveTexture as number[]);
    gl.texSubImage2D(gl.TEXTURE_2D, 0, 0, 0, 128, 8, gl.RED, gl.FLOAT, wavePixels);
    upload({
      ...appearanceUniforms(config, p.dark),
      ...dynamic,
      u_resolution: [size, size],
      u_pixelRatio: 1,
      u_signal: [size / 2, size / 2, p.resolution * 0.39, 1],
      u_paperColor: p.dark ? [0.082, 0.086, 0.075] : [0.941, 0.945, 0.921],
      u_inkColor: neutralInk(p.dark),
      u_colorStrength: config.colorStrength ?? 0.5,
    });
    gl.clearColor(0, 0, 0, 0);
    gl.clear(gl.COLOR_BUFFER_BIT);
    gl.drawArrays(gl.TRIANGLES, 0, 6);
    if (p.canvas.width !== size) {
      p.canvas.width = size;
      p.canvas.height = size;
    }
    const ctx = p.canvas.getContext("2d")!;
    ctx.clearRect(0, 0, size, size);
    ctx.drawImage(source, 0, 0);
    p.canvas.dataset.failed = "false";
  };
}
function paint(p: Preview) {
  try {
    render ??= createRenderer();
    render(p);
    p.onFrame?.(p.player, true);
    p.dirty = false;
  } catch (error) {
    p.canvas.dataset.failed = "true";
    p.canvas.dataset.error = String(error);
    console.error(error);
    p.dirty = false;
    p.live = false;
  }
}
function tick(milliseconds: number) {
  const delta = milliseconds / 1000;
  let keepAlive = false;
  for (const p of previews) {
    const activity = advancePreview(p.player, p, delta);
    keepAlive ||= activity.keepAlive;
    if (!p.visible) {
      p.onFrame?.(p.player, false);
      continue;
    }
    if (!activity.draw) {
      p.onFrame?.(p.player, true);
      continue;
    }
    paint(p);
  }
  return keepAlive;
}
export function registerPreview(canvas: HTMLCanvasElement, player: ExpressionPlayer) {
  const preview: Preview = {
    canvas,
    player,
    dark: true,
    resolution: 96,
    live: false,
    visible: true,
    dirty: true,
    paused: false,
    trackTime: false,
  };
  previews.add(preview);
  const observer = new IntersectionObserver(([entry]) => {
    preview.visible = entry.isIntersecting;
    preview.dirty = true;
    driver?.wake();
  });
  observer.observe(canvas);
  driver ??= frameTask(tick);
  driver.wake();
  return {
    paint: () => paint(preview),
    update(patch: Partial<Preview>) {
      Object.assign(preview, patch, { dirty: true });
      driver?.wake();
    },
    dispose() {
      observer.disconnect();
      previews.delete(preview);
      if (!previews.size) {
        driver?.dispose();
        driver = undefined;
      }
    },
  };
}
