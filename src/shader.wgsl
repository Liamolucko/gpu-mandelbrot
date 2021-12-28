[[stage(vertex)]]
fn vs_main([[builtin(vertex_index)]] idx: u32) -> [[builtin(position)]] vec4<f32> {
    // Return the four points necessary to cover the entire screen with a triangle strip (forming a rectangle).
    if (idx == 0u) {
        return vec4<f32>(-1.0, 1.0, 0.0, 1.0);
    } else if (idx == 1u) {
        return vec4<f32>(1.0, 1.0, 0.0, 1.0);
    } else if (idx == 2u) {
        return vec4<f32>(-1.0, -1.0, 0.0, 1.0);
    } else {
        return vec4<f32>(1.0, -1.0, 0.0, 1.0);
    }
}

// This will be fed into a format string, which will then set this.
// let comp_size: u32 = {comp_size};
let comp_size: u32 = 1u; // just for type checking.

struct Component {
    // The integer portion of this fixed-point number, which also controls the sign.
    int: i32;
    // The sub-integer portion. This is _always positive_; so even if `int` is negative, it's added on top.
    subint: array<u32, comp_size>;
};

// Multiplies the sub-integer portions of two components together.
// Overflow isn't a concern, because the product of two values which are less than one will always be less than one.
fn mul_subint(a: ptr<private, array<u32, comp_size>>, b: ptr<private, array<u32, comp_size>>) -> array<u32, comp_size> {
    var out: array<u32, comp_size>;

    // Multiply the numbers in 16-bit segments using primary-school style multiplication.
    // This probably isn't the most efficient way of doing it, but it's the simplest.
    for (var dest_index = 2u * comp_size - 1u; dest_index >= 0u; dest_index = dest_index - 1u) {
        for (var i = 0u; i < 2u * comp_size; i = i + 1u) {
            let j = dest_index - i - 1u;

            var dig_a = (*a)[i / 2u];
            if (i % 2u == 0u) {
                dig_a = dig_a >> 16u;
            } else {
                dig_a = dig_a & 0x0000ffffu;
            }

            var dig_b = (*b)[j / 2u];
            if (j % 2u == 0u) {
                dig_b = dig_b >> 16u;
            } else {
                dig_b = dig_b & 0x0000ffffu;
            }

            // The product of two 16-bit integers will always fit in a 32-bit integer,
            // because the highest possible value, 2^16 - 1, multiplied by itself does - (2^16 - 1)^2 = 2^32 - 2^17 + 1.
            let result = dig_a * dig_b;

            if (dest_index % 2u == 0u) {
                var digit = dest_index / 2u;
                var value = result;

                loop {
                    out[digit] = out[digit] + value;
                    if (out[digit] < value) {
                        // It overflowed, move on to the next one.
                        digit = digit - 1u;
                        // Addition can only overflow to a 1 on the next digit up.
                        value = 1u;
                    } else {
                        break;
                    }
                }
            } else {
                var digit = dest_index / 2u + 1u;
                var value = result & 0x0000ffffu;

                loop {
                    out[digit] = out[digit] + value;
                    if (out[digit] < value) {
                        // It overflowed, move on to the next one.
                        digit = digit - 1u;
                        // Addition can only overflow to a 1 on the next digit up.
                        value = 1u;
                    } else {
                        break;
                    }
                }

                digit = digit - 1u;
                value = result >> 16u;

                loop {
                    out[digit] = out[digit] + value;
                    if (out[digit] < value) {
                        // It overflowed, move on to the next one.
                        digit = digit - 1u;
                        // Addition can only overflow to a 1 on the next digit up.
                        value = 1u;
                    } else {
                        break;
                    }
                }
            }
        }
    }
    
    return out;
}

fn mul(a: ptr<private, Component>, b: ptr<private, Component>) -> Component {
    var out: Component;

    // We don't need to bother checking for overflow, because the highest these will ever be is 2 anyway.
    // Even if the camera is super far away or super zoomed out,
    // the point will be disqualified on the first iteration before any real arithmetic is done to it.
    out.int = (*a).int * (*b).int;
    out.subint = mul_subint(&(*a).subint, &(*b).subint);

    // TODO: we also need to multiply each integer component by the other sub-integer component.

    return out;
}

// Adds `a` and `b` and stores the result in `a`.
fn add(a: ptr<private, Component>, b: ptr<private, Component>) {
    for (var i = comp_size - 1u; i >= 0u; i = i - 1u) {
        
    }
}

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