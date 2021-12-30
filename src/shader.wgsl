// note: i'm assuming here that everything will wrap on overflow, but I can't find anything which specifically says that's the case.

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
let u32_max: u32 = 0xffffffffu;

struct Component {
    // The integer portion of this fixed-point number, which also controls the sign.
    int: i32;
    // The sub-integer portion. This is _always positive_; so even if `int` is negative, it's added on top.
    subint: array<u32, comp_size>;
};

// Add a 32-bit value to `subint` starting at the 16-bit 'digit' `digit`, properly handling overflow.
fn add_at(subint: ptr<function, array<u32, comp_size>>, digit: u32, value: u32) {
    if (digit % 2u == 0u) {
        var i = digit / 2u;
        (*subint)[i] = (*subint)[i] + value;
        if ((*subint)[i] < value) {
            // It overflowed; add 1 to the next digit up, and further if necessary.
            loop {
                i = i - 1u;
                (*subint)[i] = (*subint)[i] + 1u;
                if ((*subint)[i] != 0u) {
                    // It didn't overflow again, we can stop.
                    break;
                }
            }
        }
    } else {
        let upper_i = digit / 2u;
        let lower_i = upper_i + 1u;

        // Whether the digit at `upper_i` has overflowed.
        var overflowed = false;

        let lower = value << 16u;
        (*subint)[lower_i] = (*subint)[lower_i] + lower;
        if ((*subint)[i] < lower) {
            // It overflowed; add 1 to the next digit up.
            (*subint)[upper_i] = (*subint)[upper_i] + 1u;

            // Record if it's overflowed again.
            if ((*subint)[upper_i] == 0u) {
                overflowed = true;
            }
        }

        let upper = value >> 16u;
        (*subint)[upper_i] = (*subint)[upper_i] + upper;
        if ((*subint)[upper_i] < upper) {
            overflowed = true;
        }

        if (overflowed) {
            var j = upper_i - 1u;
            loop {
                (*subint)[j] = (*subint)[j] + 1u;
                if ((*subint)[j] == 0u) {
                    // It must have overflowed.
                    j = j - 1u;
                } else {
                    break;
                }
            }
        }
    }
}

// Subtract a 32-bit value from `subint` starting at the 16-bit 'digit' `digit`, properly handling overflow.
fn sub_at(subint: ptr<function, array<u32, comp_size>>, digit: u32, value: u32) {
    if (digit % 2u == 0u) {
        var i = digit / 2u;
        (*subint)[i] = (*subint)[i] - value;

        // subint = max - x
        // overflow check: max - x - value > max - value
        // subint - value > max - value
        // ^^^^^^^^^^^^^^ - new value of subint
        if ((*subint)[i] > u32_max - value) {
            // It overflowed; we need to subtract 1 from the next digit up, and so on.
            loop {
                i = i - 1u;
                (*subint)[i] = (*subint)[i] - 1u;
                if ((*subint)[i] != u32_max) {
                    // It didn't overflow; we can stop now.
                    break;
                }
            }
        }
    } else {
        // TODO

        let upper_i = digit / 2u;
        let lower_i = upper_i + 1u;

        // Whether the digit at `upper_i` has overflowed.
        var overflowed = false;

        let lower = value << 16u;
        (*subint)[lower_i] = (*subint)[lower_i] - lower;
        if ((*subint)[i] > u32_max - lower) {
            // It overflowed; subtract 1 from the next digit up.
            (*subint)[upper_i] = (*subint)[upper_i] - 1u;

            // Record if it's overflowed again.
            if ((*subint)[upper_i] == u32_max) {
                overflowed = true;
            }
        }

        let upper = value >> 16u;
        (*subint)[upper_i] = (*subint)[upper_i] - upper;
        if ((*subint)[upper_i] > u32_max - upper) {
            overflowed = true;
        }

        if (overflowed) {
            var j = upper_i - 1u;
            loop {
                (*subint)[j] = (*subint)[j] - 1u;
                if ((*subint)[j] == u32_max) {
                    // It must have overflowed.
                    j = j - 1u;
                } else {
                    break;
                }
            }
        }
    }
}

// Multiplies the sub-integer portions of two components together.
// Overflow isn't a concern, because the product of two values which are less than one will always be less than one.
fn mul_subint(a: ptr<function, array<u32, comp_size>>, b: ptr<function, array<u32, comp_size>>) -> array<u32, comp_size> {
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

            add_at(&out, dest_index, result);
        }
    }

    return out;
}

fn mul(a: ptr<function, Component>, b: ptr<function, Component>) -> Component {
    var out: Component;

    // We don't need to bother checking for overflow, because the highest these will ever be is 2 anyway.
    // Even if the camera is super far away or super zoomed out,
    // the point will be disqualified on the first iteration before any real arithmetic is done to it.
    out.int = (*a).int * (*b).int;

    let a_subint = &(*a).subint;
    let b_subint = &(*b).subint;

    out.subint = mul_subint(a_subint, b_subint);

    // Multiply each integer component by the other sub-integer component.
    for (var i = 2u * comp_size - 1u; i >= 0u; i = i - 1u) {
        var a_digit = (*a_subint)[i / 2u];
        var b_digit = (*b_subint)[i / 2u];
        if (i % 2u == 0u) {
            a_digit = a_digit >> 16u;
            b_digit = b_digit >> 16u;
        } else {
            a_digit = a_digit & 0x0000ffffu;
            b_digit = b_digit & 0x0000ffffu;
        }

        let res1 = u32(abs((*b).int)) * a_digit;
        let res2 = u32(abs((*a).int)) * b_digit;

        if ((*b).int > 0) {
            add_at(&out.subint, i, res1);
        } else {
            sub_at(&out.subint, i, res1);
        }

        if ((*a).int > 0) {
            add_at(&out.subint, i, res2);
        } else {
            sub_at(&out.subint, i, res2);
        }
    }


    return out;
}

// Adds `b` to `a`.
fn add(a: ptr<function, Component>, b: ptr<function, Component>) {
    for (var i = comp_size - 1u; i >= 0u; i = i - 1u) {
        // Even though we don't need the unaligned indices support, this is simpler than reimplementing it.
        // Hopefully that branch will get optimised away? I have no idea how much optimisation is done to shaders.
        add_at(&(*a).subint, 2u * i, (*b).subint[i]);
    }
}

// Subtracts `b` from `a`.
fn sub(a: ptr<function, Component>, b: ptr<function, Component>) {
    for (var i = comp_size - 1u; i >= 0u; i = i - 1u) {
        // Even though we don't need the unaligned indices support, this is simpler than reimplementing it.
        // Hopefully that branch will get optimised away? I have no idea how much optimisation is done to shaders.
        sub_at(&(*a).subint, 2u * i, (*b).subint[i]);
    }
}

fn double(num: ptr<function, Component>) {
    (*num).int = 2u * (*num).int + (*num).subint[0] >> 31u;
    for (var i = 0u; i < comp_size; i = i + 1u) {
        if (i + 1u < comp_size) {
            (*num).subint[i] = (*num).subint[i] << 1u + (*num).subint[i + 1u] >> 31u;
        } else {
            (*num).subint[i] = (*num).subint[i] << 1u;
        }
    }
}

struct Complex {
    real: Component;
    imag: Component;
};

/// Squares a complex number.
fn square(num: ptr<function, Complex>) -> Complex {
    var out: Complex;

    var real_fact_1 = (*num).real;
    add(&real_fact_1, &(*num).imag);

    var real_fact_2 = (*num).real;
    sub(&real_fact_1, &(*num).imag);

    out.real = mul(&real_fact_1, &real_fact_2);
    out.imag = mul(&(*num).real, &(*num).imag);
    double(&out.imag);

    return out;
}

struct Settings {
    center: vec2<f32>;

    camera: Complex;
    zoom: f32;

    iterations: u32;
};

[[group(0), binding(0)]] var<uniform> settings: Settings;

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
