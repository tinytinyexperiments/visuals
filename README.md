## Visuals Workspace

This repo is a Rust playground for different kinds of visuals:

- **Real-time GPU graphics with `wgpu`** (`wgpu-playground`)
- **CPU raytracing renderer** (`raytracer`)
- **ASCII / terminal-based visualizers** (`terminal-visuals`)

Each is a separate crate in a single Cargo workspace.

### Prerequisites

- **Rust toolchain** (via `rustup`)
- On macOS, you should be fine out of the box for `wgpu` (it uses Metal under the hood).

Install Rust (this gives you `cargo`):

```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
```

Then either **restart your terminal** or run:

```bash
. "$HOME/.cargo/env"
```

From the workspace root:

```bash
cd /Users/nicholas/visuals
```

### wgpu-playground (real-time GPU)

- **What it does**: Opens a window and renders a full-screen, animated neon fractal-style shader using `wgpu` and `winit`.
- **Run**:

```bash
cargo run -p wgpu-playground
```

- **Next experiments**:
  - Turn the shader into a proper Mandelbrot/Julia explorer.
  - Add a 2D/3D camera and draw geometry instead of just a full-screen triangle.
  - Build particle systems or simple fluid-like fields on the GPU.

### raytracer (CPU renderer)

- **What it does**: Renders a small ray-traced scene with multiple spheres and a ground plane, with basic anti-aliasing, to a `PPM` image.
- **Output**: `image.ppm` in the `raytracer` crate directory.
- **Run**:

```bash
cargo run -p raytracer
```

- **Viewing the image**:
  - Many image viewers can open `PPM` directly.
  - Or convert it (e.g. with ImageMagick) to PNG.

- **Next experiments**:
  - Add diffuse / metal / glass materials and recursive bounces.
  - Implement depth of field and motion blur.
  - Port more of *Ray Tracing in One Weekend* into this crate.

### terminal-visuals (ASCII art)

- **What it does**: Uses `crossterm` to render an animated ASCII Mandelbrot-style fractal in your terminal in an alternate screen.
- **Controls**:
  - Press **`q`** to quit.
- **Run**:

```bash
cargo run -p terminal-visuals
```

- **Next experiments**:
  - Swap the fractal for a spinning 3D cube or particle field.
  - Hook into audio input for a music visualizer.
  - Add keyboard controls for zooming and panning the fractal.

### References / inspiration

- **wgpu docs**: see the guides and examples at `https://wgpu.rs`
- **Ray tracing**: Peter Shirley’s *Ray Tracing in One Weekend* (and Rust ports on GitHub)
- **Terminal graphics**: `crossterm` crate docs on crates.io


