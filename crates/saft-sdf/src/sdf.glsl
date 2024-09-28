#line 2

float square_vec3(vec3 v) { return dot(v, v); }

float sd_plane(vec3 pos, vec4 plane) { return dot(pos, plane.xyz) + plane.w; }

vec4 sdrgb_plane(vec3 pos, vec4 plane) { return vec4(vec3(1.0), sd_plane(pos, plane)); }

float sd_sphere(vec3 pos, vec3 center, float radius) { return distance(pos, center) - radius; }

vec4 sdrgb_sphere(vec3 pos, vec3 center, float radius) {
    return vec4(vec3(1.0), sd_sphere(pos, center, radius));
}

float sd_rounded_box(vec3 pos, vec3 half_size, float rounding_radius) {
    vec3 q = abs(pos) - half_size + rounding_radius;
    return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0) - rounding_radius;
}

vec4 sdrgb_rounded_box(vec3 pos, vec3 half_size, float radius) {
    return vec4(vec3(1.0), sd_rounded_box(pos, half_size, radius));
}

float sd_torus(vec3 pos, float big_r, float small_r) {
    vec2 q = vec2(length(pos.xz) - big_r, pos.y);
    return length(q) - small_r;
}

vec4 sdrgb_torus(vec3 pos, float big_r, float small_r) {
    return vec4(vec3(1.0), sd_torus(pos, big_r, small_r));
}

float sd_torus_sector(vec3 pos, float big_r, float small_r, vec2 sin_cos_half_angle) {
    pos.x = abs(pos.x);
    float k = (sin_cos_half_angle.y * pos.x > sin_cos_half_angle.x * pos.z)
                  ? dot(pos.xz, sin_cos_half_angle)
                  : length(pos.xz);
    return sqrt(dot(pos, pos) + big_r * big_r - 2.0 * big_r * k) - small_r;
}

vec4 sdrgb_torus_sector(vec3 pos, float big_r, float small_r, vec2 sin_cos_half_angle) {
    return vec4(vec3(1.0), sd_torus_sector(pos, big_r, small_r, sin_cos_half_angle));
}

float sd_capsule(vec3 pos, vec3 p0, vec3 p1, float radius) {
    vec3 pa = pos - p0;
    vec3 ba = p1 - p0;
    float h = clamp(dot(pa, ba) / dot(ba, ba), 0.0, 1.0);
    return distance(pa, ba * h) - radius;
}

vec4 sdrgb_capsule(vec3 pos, vec3 p0, vec3 p1, float radius) {
    return vec4(vec3(1.0), sd_capsule(pos, p0, p1, radius));
}

float sd_rounded_cylinder(vec3 pos,
                          float cylinder_radius,
                          float half_height,
                          float rounding_radius) {
    vec2 d = vec2(length(pos.xz) - cylinder_radius + rounding_radius,
                  abs(pos.y) - half_height + rounding_radius);
    return min(max(d.x, d.y), 0.0) + length(max(d, 0.0)) - rounding_radius;
}

vec4 sdrgb_rounded_cylinder(vec3 pos,
                            float cylinder_radius,
                            float half_height,
                            float rounding_radius) {
    return vec4(vec3(1.0), sd_rounded_cylinder(pos, cylinder_radius, half_height, rounding_radius));
}

float sd_tapered_capsule(vec3 pos, vec3 p0, vec3 p1, float r0, float r1) {
    // Straight from https://www.iquilezles.org/www/articles/distfunctions/distfunctions.htm

    // sampling independent computations (only depend on shape)
    vec3 ba = p1 - p0;
    float l2 = dot(ba, ba);
    float rr = r0 - r1;
    float a2 = l2 - rr * rr;
    float il2 = 1.0 / l2;

    // sampling dependant computations
    vec3 pa = pos - p0;
    float y = dot(pa, ba);
    float z = y - l2;
    float x2 = square_vec3(pa * l2 - ba * y);
    float y2 = y * y * l2;
    float z2 = z * z * l2;

    // single square root!
    float k = sign(rr) * rr * rr * x2;
    if (sign(z) * a2 * z2 > k) {
        return sqrt(x2 + z2) * il2 - r1;
    } else if (sign(y) * a2 * y2 < k) {
        return sqrt(x2 + y2) * il2 - r0;
    } else {
        return (sqrt(x2 * a2 * il2) + y * rr) * il2 - r0;
    }
}

vec4 sdrgb_tapered_capsule(vec3 pos, vec3 p0, vec3 p1, float r0, float r1) {
    return vec4(vec3(1.0), sd_tapered_capsule(pos, p0, p1, r0, r1));
}

/// Base at origin, with height `h` along positive Y.
float sd_cone(vec3 p, float r, float h) {
    vec2 q = vec2(r, h);
    vec2 w = vec2(length(p.xz), h - p.y);
    vec2 a = w - q * clamp(dot(w, q) / dot(q, q), 0.0, 1.0);
    vec2 b = w - vec2(r * clamp(w.x / r, 0.0, 1.0), h);
    float d = min(dot(a, a), dot(b, b));
    float s = max(w.x * h - w.y * r, w.y - h);
    return sqrt(d) * sign(s);
}

vec4 sdrgb_cone(vec3 pos, float r, float h) { return vec4(vec3(1.0), sd_cone(pos, r, h)); }

float sd_material(float sd, vec3 rgb) { return sd; }

vec4 sdrgb_material(vec4 sd, vec3 rgb) { return vec4(rgb, sd.w); }

float sd_op_union(float sd1, float sd2) { return min(sd1, sd2); }

vec4 sdrgb_op_union(vec4 sd1, vec4 sd2) {
    if (sd1.w < sd2.w) {
        return sd1;
    } else {
        return sd2;
    }
}

float sd_op_subtract(float sd1, float sd2) { return max(-sd1, sd2); }

vec4 sdrgb_op_subtract(vec4 sd1, vec4 sd2) {
    if (-sd1.w > sd2.w) {
        return vec4(sd1.rgb, -sd1.w);
    } else {
        return sd2;
    }
}

float sd_op_intersect(float sd1, float sd2) { return max(sd1, sd2); }

vec4 sdrgb_op_intersect(vec4 sd1, vec4 sd2) {
    if (sd1.w > sd2.w) {
        return sd1;
    } else {
        return sd2;
    }
}

float sd_op_union_smooth(float d1, float d2, float size) {
    float h = clamp(0.5 + 0.5 * (d2 - d1) / size, 0.0, 1.0);
    return mix(d2, d1, h) - size * h * (1.0 - h);
}

vec4 sdrgb_op_union_smooth(vec4 d1, vec4 d2, float size) {
    float h = clamp(0.5 + 0.5 * (d2.w - d1.w) / size, 0.0, 1.0);
    vec4 mixed = mix(d2, d1, h);
    float dist = mixed.w - size * h * (1.0 - h);
    return vec4(mixed.rgb, dist);
}

float sd_op_subtract_smooth(float d1, float d2, float size) {
    float h = clamp(0.5 - 0.5 * (d2 + d1) / size, 0.0, 1.0);
    return mix(d2, -d1, h) + size * h * (1.0 - h);
}

vec4 sdrgb_op_subtract_smooth(vec4 d1, vec4 d2, float size) {
    float h = clamp(0.5 - 0.5 * (d2.w + d1.w) / size, 0.0, 1.0);
    vec4 mixed = mix(d2, vec4(d1.rgb, -d1.w), h);
    float dist = mixed.w + size * h * (1.0 - h);
    return vec4(mixed.rgb, dist);
}

float sd_op_intersect_smooth(float d1, float d2, float size) {
    float h = clamp(0.5 - 0.5 * (d2 - d1) / size, 0.0, 1.0);
    return mix(d2, d1, h) + size * h * (1.0 - h);
}

vec4 sdrgb_op_intersect_smooth(vec4 d1, vec4 d2, float size) {
    float h = clamp(0.5 - 0.5 * (d2.w - d1.w) / size, 0.0, 1.0);
    vec4 mixed = mix(d2, d1, h);
    float dist = mixed.w + size * h * (1.0 - h);
    return vec4(mixed.rgb, dist);
}

float sd_biconvex_lens(vec3 pos, float lower_sagitta, float upper_sagitta, float chord) {
    float chord_radius = chord / 2.0;
    float lower_radius =
        (chord_radius * chord_radius + lower_sagitta * lower_sagitta) / (2.0 * lower_sagitta);
    float upper_radius =
        (chord_radius * chord_radius + upper_sagitta * upper_sagitta) / (2.0 * upper_sagitta);
    vec3 lower_center = vec3(0.0, lower_radius - lower_sagitta, 0.0);
    vec3 upper_center = vec3(0.0, -(upper_radius - upper_sagitta), 0.0);
    return sd_op_intersect(sd_sphere(pos, lower_center, lower_radius),
                           sd_sphere(pos, upper_center, upper_radius));
}

vec4 sdrgb_biconvex_lens(vec3 pos, float lower_sagitta, float upper_sagitta, float chord) {
    return vec4(vec3(1.0), sd_biconvex_lens(pos, lower_sagitta, upper_sagitta, chord));
}

vec3 mul_quat(vec4 q, vec3 v) { return v + 2.0 * cross(q.xyz, cross(q.xyz, v) + q.w * v); }

float sd_op_scale_distance(float sd, float scale) { return sd * scale; }

vec4 sdrgb_op_scale_distance(vec4 sd, float scale) { return vec4(sd.rgb, sd.w * scale); }
