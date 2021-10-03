[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] idx: u32) -> [[builtin(position)]] vec4<f32> {
    // Return the four points necessary to cover the entire screen with a triangle strip (forming a rectangle).
    if (idx == 0u) {
        return vec4<f32>(-1.0, 1.0, 0.0, 1.0);
    } elseif (idx == 1u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    } elseif (idx == 2u) {
        return vec4<f32>(-1.0, -1.0, 0.0, 1.0);
    } else {
        return vec4<f32>(1.0, -1.0, 0.0, 1.0);
    }
}

[[block]]
struct Settings {
    center: vec2<f32>;

    camera: vec2<f32>;
    zoom: f32;

    iterations: u32;
};

[[group(0), binding(0)]] var<uniform> settings: Settings;

/// Squares a complex number.
fn square(num: vec2<f32>) -> vec2<f32> {
    return vec2<f32>((num.x + num.y) * (num.x - num.y), 2.0 * num.x * num.y);
}

[[stage(fragment)]]
fn fs_main([[builtin(position)]] pixel: vec4<f32>) -> [[location(0)]] vec4<f32> {
    let offset = pixel.xy - settings.center;
    // Flip around the y, since in pixel space y gets bigger going downwards, whereas on the complex plane it's the reverse.
    let pos = settings.camera + vec2<f32>(offset.x, -offset.y) / settings.zoom;

    var point = vec2<f32>(0.0, 0.0);
    var iters = 0u;
    loop {
        point = square(point) + pos;
        iters = iters + 1u;

        if (dot(point, point) > 4.0 || iters == settings.iterations) { break }
    }

    var l = f32(iters) / f32(settings.iterations);

    if (iters == settings.iterations) {
        l = 0.0;
    }

    return vec4<f32>(l, l, l, 1.0);
}