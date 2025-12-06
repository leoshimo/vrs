"use client";
import { changedUniform } from "./uniform-cache";
import { useEffect, useImperativeHandle, useRef, type Ref } from "react";
type Uniforms = Record<string, number | number[]>;
export type NativeShaderHandle = {
  setUniforms: (values: Uniforms) => void;
  getCurrentFrame: () => number;
};

/** A small WebGL2 host for the original signal shader. No graphics dependency. */
export function NativeShader({
  fragment,
  uniforms,
  speed,
  ref,
  onFrame,
  active = true,
}: {
  fragment: string;
  uniforms: Uniforms;
  speed: number;
  ref: Ref<NativeShaderHandle>;
  onFrame?: (delta: number) => Uniforms;
  active?: boolean;
}) {
  const canvas = useRef<HTMLCanvasElement>(null);
  const values = useRef<Uniforms>({ ...uniforms }),
    tick = useRef<() => void>(() => {});
  const frameCallback = useRef(onFrame);
  const enabled = useRef(active);
  const pause = useRef<() => void>(() => {});
  useEffect(() => {
    enabled.current = active;
    if (active) tick.current();
    else pause.current();
  }, [active]);
  useEffect(() => {
    frameCallback.current = onFrame;
  }, [onFrame]);
  const running = useRef(speed),
    clock = useRef(0);
  useImperativeHandle(
    ref,
    () => ({
      setUniforms(next) {
        Object.assign(values.current, next);
        tick.current();
      },
      getCurrentFrame() {
        return clock.current;
      },
    }),
    [],
  );
  useEffect(() => {
    running.current = speed;
    tick.current();
  }, [speed]);
  useEffect(() => {
    const el = canvas.current;
    if (!el) return;
    let dispose = () => {};
    function start() {
      const gl = el!.getContext("webgl2", {
        alpha: true,
        premultipliedAlpha: true,
        antialias: false,
        powerPreference: "low-power",
      });
      if (!gl) {
        el!.dataset.failed = "true";
        return;
      }
      const compile = (type: number, source: string) => {
        const shader = gl.createShader(type)!;
        gl.shaderSource(shader, source);
        gl.compileShader(shader);
        if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
          const message = gl.getShaderInfoLog(shader);
          gl.deleteShader(shader);
          throw new Error(message || "Shader compile failed");
        }
        return shader;
      };
      let vertex: WebGLShader | undefined, pixel: WebGLShader | undefined;
      const program = gl.createProgram()!;
      try {
        vertex = compile(
          gl.VERTEX_SHADER,
          "#version 300 es\nin vec2 position;void main(){gl_Position=vec4(position,0.,1.);}",
        );
        pixel = compile(gl.FRAGMENT_SHADER, fragment);
        gl.attachShader(program, vertex);
        gl.attachShader(program, pixel);
        gl.linkProgram(program);
        if (!gl.getProgramParameter(program, gl.LINK_STATUS))
          throw new Error(gl.getProgramInfoLog(program) || "Shader link failed");
      } catch (error) {
        console.error(error);
        el!.dataset.failed = "true";
        if (vertex) gl.deleteShader(vertex);
        if (pixel) gl.deleteShader(pixel);
        gl.deleteProgram(program);
        return;
      }
      const activateProgram = gl.useProgram.bind(gl);
      activateProgram(program);
      const buffer = gl.createBuffer()!;
      gl.bindBuffer(gl.ARRAY_BUFFER, buffer);
      gl.bufferData(
        gl.ARRAY_BUFFER,
        new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
        gl.STATIC_DRAW,
      );
      const position = gl.getAttribLocation(program, "position");
      gl.enableVertexAttribArray(position);
      gl.vertexAttribPointer(position, 2, gl.FLOAT, false, 0, 0);
      const locations = new Map<string, WebGLUniformLocation | null>();
      const uploaded = new Map<string, number | number[]>();
      function uniform(key: string, value: number | number[]) {
        if (!changedUniform(uploaded, key, value)) return;
        if (!locations.has(key)) locations.set(key, gl!.getUniformLocation(program, key));
        const location = locations.get(key)!;
        if (typeof value === "number") gl!.uniform1f(location, value);
        else if (value.length === 2) gl!.uniform2fv(location, value);
        else if (value.length === 3) gl!.uniform3fv(location, value);
        else if (value.length === 4) gl!.uniform4fv(location, value);
      }
      let request = 0,
        last = performance.now(),
        visible = true;
      let cssWidth = el!.clientWidth,
        cssHeight = el!.clientHeight;
      function render(now: number) {
        request = 0;
        if (document.hidden || !visible) return;
        const delta = enabled.current ? Math.min(now - last, 64) / 1000 : 0;
        clock.current += delta * 1000 * running.current;
        if (frameCallback.current && enabled.current)
          Object.assign(values.current, frameCallback.current(delta));
        last = now;
        const ratio = Math.min(devicePixelRatio, 2),
          width = Math.max(1, Math.round(cssWidth * ratio)),
          height = Math.max(1, Math.round(cssHeight * ratio));
        if (el!.width !== width || el!.height !== height) {
          el!.width = width;
          el!.height = height;
          gl!.viewport(0, 0, width, height);
        }
        activateProgram(program);
        for (const [key, value] of Object.entries(values.current)) uniform(key, value);
        uniform("u_resolution", [width, height]);
        uniform("u_pixelRatio", ratio);
        uniform("u_time", clock.current / 1000);
        gl!.clearColor(0, 0, 0, 0);
        gl!.clear(gl!.COLOR_BUFFER_BIT);
        gl!.drawArrays(gl!.TRIANGLES, 0, 6);
        if (el!.dataset.failed !== "false") {
          el!.dataset.failed = "false";
          el!.closest(".activity-command")?.setAttribute("data-signal-ready", "true");
        }
        if (running.current && enabled.current) request = requestAnimationFrame(render);
      }
      function schedule() {
        if (!request && !document.hidden && visible) {
          last = performance.now();
          request = requestAnimationFrame(render);
        }
      }
      const resize = new ResizeObserver((entries) => {
        cssWidth = entries[0].contentRect.width;
        cssHeight = entries[0].contentRect.height;
        schedule();
      });
      resize.observe(el!);
      const intersection = new IntersectionObserver((entries) => {
        visible = entries[0].isIntersecting;
        if (visible) schedule();
        else {
          cancelAnimationFrame(request);
          request = 0;
        }
      });
      intersection.observe(el!);
      const visibility = () => {
        if (document.hidden) {
          cancelAnimationFrame(request);
          request = 0;
        } else schedule();
      };
      document.addEventListener("visibilitychange", visibility);
      tick.current = schedule;
      pause.current = () => {
        cancelAnimationFrame(request);
        request = 0;
      };
      schedule();
      dispose = () => {
        cancelAnimationFrame(request);
        resize.disconnect();
        intersection.disconnect();
        document.removeEventListener("visibilitychange", visibility);
        tick.current = () => {};
        gl!.deleteBuffer(buffer);
        gl!.deleteProgram(program);
        gl!.deleteShader(vertex!);
        gl!.deleteShader(pixel!);
      };
    }
    const lost = (e: Event) => {
      e.preventDefault();
      dispose();
      el.dataset.failed = "true";
      el.closest(".activity-command")?.setAttribute("data-signal-ready", "false");
    };
    const restored = () => start();
    el.addEventListener("webglcontextlost", lost);
    el.addEventListener("webglcontextrestored", restored);
    start();
    return () => {
      dispose();
      el.removeEventListener("webglcontextlost", lost);
      el.removeEventListener("webglcontextrestored", restored);
    };
  }, [fragment]);
  return <canvas ref={canvas} className="native-shader" aria-hidden="true" />;
}
