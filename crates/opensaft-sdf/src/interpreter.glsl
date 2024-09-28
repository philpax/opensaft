#line 0

#define Opcode_Plane          0 // vec4
#define Opcode_Sphere         1 // center: vec3, radius: f32
#define Opcode_Capsule        2 // p0: vec3, p1: vec3, radius: f32
#define Opcode_TaperedCapsule 3 // p0: vec3, p1: vec3, radius: f32
#define Opcode_Material       4 // rgb: vec3

// Combinators:
#define Opcode_Union           5
#define Opcode_UnionSmooth     6
#define Opcode_Subtract        7
#define Opcode_SubtractSmooth  8
#define Opcode_Intersect       9
#define Opcode_IntersectSmooth 10

// Transforms:
#define Opcode_PushTranslation 11
#define Opcode_PushRotation    12
#define Opcode_PopTransform    13
#define Opcode_PushScale       14
#define Opcode_PopScale        15

#define Opcode_End 16

#define Opcode_RoundedBox      17
#define Opcode_BiconvexLens    18
#define Opcode_RoundedCylinder 19
#define Opcode_Torus           20
#define Opcode_TorusSector     21
#define Opcode_Cone            22

// Using a subset of opcodes here would allow to make a stackless
// batched interpreter that basically runs the vm at a kernel dispatch level
// and runs each instruction for every grid node.

float read_float(inout uint cp) {
    return SDF_CONSTANTS[cp++];
}

vec2 read_vec2(inout uint cp) {
    return vec2(SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++]);
}

vec3 read_vec3(inout uint cp) {
    return vec3(SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++]);
}

vec4 read_vec4(inout uint  cp) {
    return vec4(SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++], SDF_CONSTANTS[cp++]);
}

vec4 sdrgb_interpret(vec3 pos, uint opcodes_offset, uint constants_offset) {
    uint sp = 0;
    uint cp = constants_offset;
    uint pc = opcodes_offset;

    vec4 stack[64];

    uint transform_sp = 0;
    vec3 transform_stack[64];

    vec3 current_position = pos;

    while (true) {
    //while (pc < SDF_OPCODES.length()) {
        uint opcode = SDF_OPCODES[pc++];

        switch (opcode) {
            case Opcode_Plane: {
                vec4 plane = read_vec4(cp);
                stack[sp++] = sdrgb_plane(current_position, plane);
            }
            break;

            case Opcode_Sphere: {
                vec3 center = read_vec3(cp);
                float radius = read_float(cp);
                stack[sp++] = sdrgb_sphere(current_position, center, radius);
            }
            break;

            case Opcode_Capsule: {
                vec3 p0 = read_vec3(cp);
                vec3 p1 = read_vec3(cp);
                float radius = read_float(cp);
                stack[sp++] = sdrgb_capsule(current_position, p0, p1, radius);
            }
            break;

            case Opcode_RoundedCylinder: {
                float radius = read_float(cp);
                float height = read_float(cp);
                float rounding = read_float(cp);
                stack[sp++] = sdrgb_rounded_cylinder(current_position,
                                                   radius,
                                                   height,
                                                   rounding);
            }
            break;

            case Opcode_TaperedCapsule: {
                vec3 p0 = read_vec3(cp);
                float r0 = read_float(cp);
                vec3 p1 = read_vec3(cp);
                float r1 = read_float(cp);
                stack[sp++] = sdrgb_tapered_capsule(current_position, p0, p1, r0, r1);
            }
            break;

            case Opcode_Cone: {
                vec2 q = read_vec2(cp);
                stack[sp++] = sdrgb_cone(current_position, q.x, q.y);
            }
            break;

            case Opcode_RoundedBox: {
                vec3 half_size = read_vec3(cp);
                float radius = read_float(cp);
                stack[sp++] = sdrgb_rounded_box(current_position, half_size, radius);
            }
            break;

            case Opcode_Torus: {
                vec2 q = read_vec2(cp);
                stack[sp++] = sdrgb_torus(current_position, q.x, q.y);
            }
            break;

            case Opcode_TorusSector: {
                vec4 params = read_vec4(cp);
                stack[sp++] = sdrgb_torus_sector(current_position, params[0], params[1], vec2(params[2], params[3]));
            }
            break;

            case Opcode_BiconvexLens: {
                float lower = read_float(cp);
                float upper = read_float(cp);
                float chord = read_float(cp);
                stack[sp++] = sdrgb_biconvex_lens(current_position, lower, upper, chord);
            }
            break;

            case Opcode_Material: {
                vec3 rgb = read_vec3(cp);
                stack[sp - 1].rgb = rgb.rgb;
            }
            break;

            // Combinators:
            case Opcode_Union: {
                sp -= 1;
                stack[sp - 1] = sdrgb_op_union(stack[sp], stack[sp - 1]);
            }
            break;

            case Opcode_UnionSmooth: {
                float smoothness = read_float(cp);
                sp -= 1;
                stack[sp - 1] = sdrgb_op_union_smooth(stack[sp], stack[sp - 1], smoothness);
            }
            break;

            case Opcode_Subtract: {
                sp -= 1;
                stack[sp - 1] = sdrgb_op_subtract(stack[sp], stack[sp - 1]);
            }
            break;

            case Opcode_SubtractSmooth: {
                float smoothness = read_float(cp);
                sp -= 1;
                stack[sp - 1] = sdrgb_op_subtract_smooth(stack[sp], stack[sp - 1], smoothness);
            }
            break;

            case Opcode_Intersect: {
                sp -= 1;
                stack[sp - 1] = sdrgb_op_intersect(stack[sp], stack[sp - 1]);
            }
            break;

            case Opcode_IntersectSmooth: {
                float smoothness = read_float(cp);
                sp -= 1;
                stack[sp - 1] = sdrgb_op_intersect_smooth(stack[sp], stack[sp - 1], smoothness);
            }
            break;

                // Transforms:
            case Opcode_PushTranslation: {
                transform_stack[transform_sp++] = current_position;
                vec3 translation = read_vec3(cp);
                current_position += translation.xyz;
            }
            break;

            case Opcode_PushRotation: {
                transform_stack[transform_sp++] = current_position;
                vec4 quat = read_vec4(cp);
                current_position = mul_quat(quat, current_position);
            }
            break;

            case Opcode_PopTransform: {
                transform_sp -= 1;
                current_position = transform_stack[transform_sp];
            }
            break;

            case Opcode_PushScale: {
                transform_stack[transform_sp++] = current_position;

                float scale = read_float(cp);
                current_position *= scale;
            }
            break;

            case Opcode_PopScale: {
                transform_sp -= 1;
                current_position = transform_stack[transform_sp];

                float inv_scale = read_float(cp);
                stack[sp - 1].w *= inv_scale;
            }
            break;

            default:
            case Opcode_End: {
                return stack[sp - 1];
            }
        }
    }

    return vec4(0.0);
}
