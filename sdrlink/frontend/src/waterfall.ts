// WebGL2 ring-buffer waterfall.
//
// A single R8 texture of size `pixels × HISTORY` stores the most recent
// `HISTORY` frames; each new frame writes one row via `texSubImage2D` at
// `writeRow = frameCount % HISTORY`. The fragment shader samples
// `fract(writeRow/HISTORY + screenV)` so the oldest row always renders at the
// top visually — no per-frame memcpy of the history.
//
// Colors come from a 256×1 RGB colormap texture; intensity is the raw u8
// from the server.

const HISTORY = 1024;

const VERTEX_SHADER = `#version 300 es
in vec2 aPos;
out vec2 vUv;
void main() {
  vUv = aPos * 0.5 + 0.5;       // [-1,1] -> [0,1]
  gl_Position = vec4(aPos, 0.0, 1.0);
}
`;

const FRAGMENT_SHADER = `#version 300 es
precision highp float;
in vec2 vUv;
out vec4 fragColor;
uniform sampler2D uSpectrum;
uniform sampler2D uColormap;
uniform float uWriteRow;
void main() {
  // Screen v=0 at bottom (newest), v=1 at top (oldest).
  // Oldest row in texture = writeRow (mod 1).
  float screenTop = 1.0 - vUv.y;
  float ty = fract(uWriteRow + screenTop);
  float intensity = texture(uSpectrum, vec2(vUv.x, ty)).r;
  fragColor = texture(uColormap, vec2(intensity, 0.5));
}
`;

function compile(gl: WebGL2RenderingContext, type: number, src: string): WebGLShader {
  const shader = gl.createShader(type);
  if (!shader) throw new Error("createShader failed");
  gl.shaderSource(shader, src);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const log = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(`shader compile: ${log}`);
  }
  return shader;
}

function link(gl: WebGL2RenderingContext, vs: WebGLShader, fs: WebGLShader): WebGLProgram {
  const p = gl.createProgram();
  if (!p) throw new Error("createProgram failed");
  gl.attachShader(p, vs);
  gl.attachShader(p, fs);
  gl.linkProgram(p);
  if (!gl.getProgramParameter(p, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(p);
    gl.deleteProgram(p);
    throw new Error(`program link: ${log}`);
  }
  return p;
}

export class Waterfall {
  private gl: WebGL2RenderingContext;
  private program: WebGLProgram;
  private specTex: WebGLTexture;
  private colormapTex: WebGLTexture;
  private uWriteRow: WebGLUniformLocation;
  private pixels = 0;
  private writeIndex = 0;

  constructor(canvas: HTMLCanvasElement, colormap: Uint8Array) {
    const gl = canvas.getContext("webgl2", {
      antialias: false,
      preserveDrawingBuffer: false,
    });
    if (!gl) throw new Error("WebGL2 not supported");
    this.gl = gl;

    const vs = compile(gl, gl.VERTEX_SHADER, VERTEX_SHADER);
    const fs = compile(gl, gl.FRAGMENT_SHADER, FRAGMENT_SHADER);
    this.program = link(gl, vs, fs);

    // Fullscreen quad (two triangles).
    const vbo = gl.createBuffer();
    gl.bindBuffer(gl.ARRAY_BUFFER, vbo);
    gl.bufferData(
      gl.ARRAY_BUFFER,
      new Float32Array([-1, -1, 1, -1, -1, 1, -1, 1, 1, -1, 1, 1]),
      gl.STATIC_DRAW,
    );
    const aPos = gl.getAttribLocation(this.program, "aPos");
    gl.enableVertexAttribArray(aPos);
    gl.vertexAttribPointer(aPos, 2, gl.FLOAT, false, 0, 0);

    const uWriteRow = gl.getUniformLocation(this.program, "uWriteRow");
    const uSpectrum = gl.getUniformLocation(this.program, "uSpectrum");
    const uColormap = gl.getUniformLocation(this.program, "uColormap");
    if (!uWriteRow || !uSpectrum || !uColormap) {
      throw new Error("missing uniform locations");
    }
    this.uWriteRow = uWriteRow;

    gl.useProgram(this.program);
    gl.uniform1i(uSpectrum, 0);
    gl.uniform1i(uColormap, 1);

    // Spectrum texture — allocated lazily once we know `pixels`.
    const specTex = gl.createTexture();
    if (!specTex) throw new Error("createTexture (spec) failed");
    this.specTex = specTex;

    // Colormap texture: 256×1 RGB.
    const colormapTex = gl.createTexture();
    if (!colormapTex) throw new Error("createTexture (colormap) failed");
    this.colormapTex = colormapTex;
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, colormapTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.RGB,
      256,
      1,
      0,
      gl.RGB,
      gl.UNSIGNED_BYTE,
      colormap,
    );
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  }

  private allocSpectrum(pixels: number) {
    const gl = this.gl;
    this.pixels = pixels;
    this.writeIndex = 0;

    (gl.canvas as HTMLCanvasElement).width = pixels;
    (gl.canvas as HTMLCanvasElement).height = HISTORY;
    gl.viewport(0, 0, pixels, HISTORY);

    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.specTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    // Zero-initialize instead of passing `null` so the browser doesn't lazy-
    // clear on every subsequent texSubImage2D (Firefox warns otherwise).
    gl.texImage2D(
      gl.TEXTURE_2D,
      0,
      gl.R8,
      pixels,
      HISTORY,
      0,
      gl.RED,
      gl.UNSIGNED_BYTE,
      new Uint8Array(pixels * HISTORY),
    );
    // Linear on X (looks nicer when stretched to screen width); nearest on Y
    // would avoid row-blending but linear makes the scroll smoother.
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
    gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.REPEAT);
  }

  /** Replace the 256×1 RGB colormap texture. */
  setColormap(colormap: Uint8Array) {
    const gl = this.gl;
    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.colormapTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texImage2D(
      gl.TEXTURE_2D, 0, gl.RGB, 256, 1, 0, gl.RGB, gl.UNSIGNED_BYTE, colormap,
    );
  }

  /** Write one row of `pixels` u8 intensities and redraw. */
  pushRow(row: Uint8Array) {
    if (row.length !== this.pixels) {
      this.allocSpectrum(row.length);
    }
    const gl = this.gl;
    const rowIdx = this.writeIndex % HISTORY;

    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.specTex);
    gl.pixelStorei(gl.UNPACK_ALIGNMENT, 1);
    gl.texSubImage2D(
      gl.TEXTURE_2D,
      0,
      0,
      rowIdx,
      this.pixels,
      1,
      gl.RED,
      gl.UNSIGNED_BYTE,
      row,
    );
    this.writeIndex++;

    // Oldest row (top of screen) = the row that's *about to be overwritten*.
    const writeRow = (this.writeIndex % HISTORY) / HISTORY;
    gl.useProgram(this.program);
    gl.uniform1f(this.uWriteRow, writeRow);

    gl.activeTexture(gl.TEXTURE1);
    gl.bindTexture(gl.TEXTURE_2D, this.colormapTex);
    gl.activeTexture(gl.TEXTURE0);
    gl.bindTexture(gl.TEXTURE_2D, this.specTex);

    gl.drawArrays(gl.TRIANGLES, 0, 6);
  }
}
