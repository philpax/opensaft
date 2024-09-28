use super::Program;
use std::rc::Rc;

#[derive(PartialEq, Eq)]
#[allow(clippy::upper_case_acronyms)]
pub enum Backend {
    GLSL,
}

#[derive(Copy, Clone)]
pub enum OutputType {
    DistanceOnly,
    DistanceWithRgb,
}

pub struct CodeGenContext<'a> {
    function_name: &'a str,
    dynamic_constants: bool,
    variable_index: usize,
    constant_index: usize,

    variable_stack: Vec<Rc<str>>,
    position_variable_stack: Vec<Rc<str>>,
    current_position: Rc<str>,
}

impl<'a> CodeGenContext<'a> {
    pub fn new(
        initial_position_variable: &str,
        function_name: &'a str,
        dynamic_constants: bool,
    ) -> Self {
        let position: Rc<str> = Rc::from(initial_position_variable);

        Self {
            function_name,
            dynamic_constants,
            variable_index: 0,
            constant_index: 0,
            variable_stack: Vec::new(),
            position_variable_stack: vec![position.clone()],
            current_position: position,
        }
    }

    pub fn push_variable(&mut self) -> Rc<str> {
        let name = format!("sdf{}", self.variable_index);
        self.variable_index += 1;

        let name: Rc<str> = Rc::from(name.as_str());
        self.variable_stack.push(name.clone());
        name
    }

    pub fn uint32(&mut self) -> String {
        let components = &[".x", ".y", ".z", ".w"];

        let s = format!(
            "{}_constants[{}_constants_offset + {}]{}",
            self.function_name,
            self.function_name,
            if self.dynamic_constants {
                self.constant_index >> 2
            } else {
                self.constant_index
            },
            if self.dynamic_constants {
                components[self.constant_index & 0x3]
            } else {
                ""
            }
        );
        self.constant_index += 1;
        s
    }

    pub fn float32(&mut self) -> String {
        format!("uintBitsToFloat({})", self.uint32())
    }

    pub fn vec2(&mut self) -> String {
        format!("vec2({}, {})", self.float32(), self.float32())
    }

    pub fn vec3(&mut self) -> String {
        format!(
            "vec3({}, {}, {})",
            self.float32(),
            self.float32(),
            self.float32()
        )
    }

    pub fn vec4(&mut self) -> String {
        format!(
            "vec4({}, {}, {}, {})",
            self.float32(),
            self.float32(),
            self.float32(),
            self.float32()
        )
    }

    pub fn quat(&mut self) -> String {
        self.vec4()
    }

    pub fn material(&mut self) -> String {
        self.vec3()
    }

    pub fn pop_variable(&mut self) -> Option<Rc<str>> {
        self.variable_stack.pop()
    }

    pub fn pop_transform(&mut self) {
        self.current_position = self.position_variable_stack.pop().unwrap();
    }

    pub fn current_position(&self) -> Rc<str> {
        self.current_position.clone()
    }

    // Returns new_position, old_position variable names.
    pub fn push_transform(&mut self) -> (Rc<str>, Rc<str>) {
        let s = format!("transform{}", self.variable_index);
        let s: Rc<str> = Rc::from(s.as_str());
        self.variable_index += 1;

        self.position_variable_stack
            .push(self.current_position.clone());
        let old_position = self.current_position.clone();
        self.current_position = s.clone();
        (s, old_position)
    }
}

pub struct CodeGen {
    backend: Backend,
}

impl CodeGen {
    pub fn glsl() -> Self {
        Self {
            backend: Backend::GLSL,
        }
    }

    // Generates code that is used by the translated program
    pub fn get_library_code(&self) -> &'static str {
        match self.backend {
            Backend::GLSL => opensaft_sdf::get_glsl_sdf_library_code(),
        }
    }

    fn build_glsl_code(
        program: &Program,
        function_name: &str,
        output_type: OutputType,
        dynamic_constants: bool,
    ) -> String {
        use super::Opcode::*;
        use std::fmt::Write;

        let mut code = String::new();
        code.push_str("// !!! START OF GENERATED CODE !!!\n");

        let output_glsl_type = match output_type {
            OutputType::DistanceOnly => "float",
            OutputType::DistanceWithRgb => "vec4",
        };

        if !dynamic_constants {
            let _ = write!(
                &mut code,
                "\tconst uint {}_constants[{}] = uint[](",
                function_name,
                program.constants.len()
            );

            for (i, c) in program.constants.iter().enumerate() {
                if i != 0 {
                    code.push(',');
                }

                let _ = write!(&mut code, "{}", c.to_bits());
            }

            code.push_str(");\n");
        }

        let mut ctx = CodeGenContext::new("pos", function_name, dynamic_constants);

        let _ = writeln!(
            &mut code,
            "{} {}_base(vec3 pos) {{",
            output_glsl_type, function_name
        );

        let prefix = match output_type {
            OutputType::DistanceOnly => "sd",
            OutputType::DistanceWithRgb => "sdrgb",
        };

        for opcode in &program.opcodes {
            match opcode {
                Plane => {
                    let variable_name = ctx.push_variable();
                    let plane = ctx.vec4();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_plane({}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        plane,
                    );
                }
                Sphere => {
                    let variable_name = ctx.push_variable();
                    let center = ctx.vec3();
                    let radius = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_sphere({}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        center,
                        radius,
                    );
                }
                Capsule => {
                    let variable_name = ctx.push_variable();
                    let points = [ctx.vec3(), ctx.vec3()];
                    let radius = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_capsule({}, {}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        points[0],
                        points[1],
                        radius,
                    );
                }
                RoundedCylinder => {
                    let variable_name = ctx.push_variable();
                    let cylinder_radius = ctx.float32();
                    let half_height = ctx.float32();
                    let rounding_radius = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_rounded_cylinder({}, {}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        cylinder_radius,
                        half_height,
                        rounding_radius,
                    );
                }
                TaperedCapsule => {
                    let variable_name = ctx.push_variable();
                    let p0 = ctx.vec3();
                    let r0 = ctx.float32();
                    let p1 = ctx.vec3();
                    let r1 = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_tapered_capsule({}, {}, {}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        p0,
                        p1,
                        r0,
                        r1,
                    );
                }
                Cone => {
                    let variable_name = ctx.push_variable();
                    let r = ctx.float32();
                    let h = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_cone({}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        r,
                        h,
                    );
                }
                RoundedBox => {
                    let variable_name = ctx.push_variable();
                    let half_size = ctx.vec3();
                    let radius = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_rounded_box({}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        half_size,
                        radius,
                    );
                }
                Torus => {
                    // big_r, small_r
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_torus({}, {}, {});",
                        output_glsl_type,
                        ctx.push_variable(),
                        prefix,
                        ctx.current_position(),
                        ctx.float32(),
                        ctx.float32(),
                    );
                }
                TorusSector => {
                    // big_r, small_r, sin_cos_half_angle
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_torus_sector({}, {}, {}, {});",
                        output_glsl_type,
                        ctx.push_variable(),
                        prefix,
                        ctx.current_position(),
                        ctx.float32(),
                        ctx.float32(),
                        ctx.vec2(),
                    );
                }
                BiconvexLens => {
                    let variable_name = ctx.push_variable();
                    let lower_sagitta = ctx.float32();
                    let upper_sagitta = ctx.float32();
                    let chord = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_biconvex_lens({}, {}, {}, {});",
                        output_glsl_type,
                        variable_name,
                        prefix,
                        ctx.current_position(),
                        lower_sagitta,
                        upper_sagitta,
                        chord,
                    );
                }
                Material => {
                    let sd = ctx.pop_variable().unwrap();
                    let material = ctx.material();
                    let variable_name = ctx.push_variable();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_material({}, {});",
                        output_glsl_type, variable_name, prefix, sd, material
                    );
                }
                Union => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_union({}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2
                    );
                }
                UnionSmooth => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let size = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_union_smooth({}, {}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2, size
                    );
                }
                Subtract => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_subtract({}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2
                    );
                }
                SubtractSmooth => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let size = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_subtract_smooth({}, {}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2, size
                    );
                }
                Intersect => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_intersect({}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2
                    );
                }
                IntersectSmooth => {
                    let sd1 = ctx.pop_variable().unwrap();
                    let sd2 = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let size = ctx.float32();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_intersect_smooth({}, {}, {});",
                        output_glsl_type, variable_name, prefix, sd1, sd2, size
                    );
                }
                PushTranslation => {
                    let translation = ctx.vec3();
                    let (new_position, old_position) = ctx.push_transform();
                    let _ = writeln!(
                        &mut code,
                        "\tvec3 {} = {} + {};",
                        new_position, old_position, translation
                    );
                }
                PopTransform => {
                    ctx.pop_transform();
                }
                PushRotation => {
                    let rotation = ctx.quat();
                    let (new_position, old_position) = ctx.push_transform();
                    let _ = writeln!(
                        &mut code,
                        "\tvec3 {} = mul_quat({}, {});",
                        new_position, rotation, old_position
                    );
                }
                PushScale => {
                    let scale = ctx.float32();
                    let (new_position, old_position) = ctx.push_transform();
                    let _ = writeln!(
                        &mut code,
                        "\tvec3 {} = {} * {};",
                        new_position, old_position, scale
                    );
                }
                PopScale => {
                    ctx.pop_transform();
                    let inv_scale = ctx.float32();
                    let sd = ctx.pop_variable().unwrap();
                    let variable_name = ctx.push_variable();
                    let _ = writeln!(
                        &mut code,
                        "\t{} {} = {}_op_scale_distance({}, {});",
                        output_glsl_type, variable_name, prefix, sd, inv_scale
                    );
                }
                End => {
                    break;
                }
            }
        }

        let ret = ctx.pop_variable().unwrap();
        let _ = writeln!(&mut code, "\treturn {};\n}}", ret);

        let _ = writeln!(
            &mut code,
            "float {}(vec3 pos) {{ return {}_base(pos){}; }}",
            function_name,
            function_name,
            match output_type {
                OutputType::DistanceOnly => "",
                OutputType::DistanceWithRgb => ".w",
            }
        );

        match output_type {
            OutputType::DistanceWithRgb => {
                let _ = writeln!(
                    &mut code,
                    "vec3 {}_color(vec3 pos) {{ return {}_base(pos).rgb; }}",
                    function_name, function_name,
                );
            }
            OutputType::DistanceOnly => {
                let _ = writeln!(
                    &mut code,
                    "vec3 {}_color(vec3 /*pos*/) {{ return vec3(1.0, 1.0, 1.0); }}",
                    function_name,
                );
            }
        }

        code.push_str("// !!! END OF GENERATED CODE !!!\n");
        code
    }

    // Recompiles an 'Program' into the target.
    pub fn to_code(
        &self,
        program: &Program,
        function_name: &str,
        output_type: OutputType,
        dynamic_constants: bool,
    ) -> String {
        match self.backend {
            Backend::GLSL => {
                Self::build_glsl_code(program, function_name, output_type, dynamic_constants)
            }
        }
    }
}
