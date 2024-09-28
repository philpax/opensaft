use num_enum::TryFromPrimitiveError;
use saft_sdf::Opcode;
use std::collections::hash_map::DefaultHasher;
use std::hash::Hash;
use std::hash::Hasher;

type UnknownOpcodeError = TryFromPrimitiveError<Opcode>;

pub fn constants_hash(constants: &[f32]) -> u64 {
    let mut s = DefaultHasher::new();
    for &c in constants {
        let v: u32 = c.to_bits();
        v.hash(&mut s);
    }

    s.finish()
}

pub fn opcodes_hash(opcodes: &[Opcode]) -> u64 {
    let mut s = DefaultHasher::new();
    opcodes.hash(&mut s);
    s.finish()
}

/// Represents a signed distance field function as a program with a constant pool and opcodes.
#[derive(Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "with_serde", derive(serde::Serialize, serde::Deserialize))]
#[cfg_attr(feature = "with_speedy", derive(speedy::Writable, speedy::Readable))]
#[cfg_attr(feature = "with_arbitrary", derive(arbitrary::Arbitrary))]
pub struct Program {
    pub constants: Vec<f32>,
    pub opcodes: Vec<Opcode>,
}

impl Program {
    #[must_use]
    pub fn with_constants(&self, constants: Vec<f32>) -> Self {
        Self {
            constants,
            opcodes: self.opcodes.clone(),
        }
    }

    pub fn constant_hash(&self) -> u64 {
        constants_hash(&self.constants)
    }

    pub fn program_hash(&self) -> u64 {
        opcodes_hash(&self.opcodes)
    }

    pub fn full_hash(&self) -> u64 {
        self.program_hash() ^ self.constant_hash()
    }

    #[cfg(feature = "with_bincode")]
    pub fn as_bytes(&self) -> Result<Vec<u8>, std::boxed::Box<bincode::ErrorKind>> {
        bincode::serialize(self)
    }

    #[cfg(feature = "with_bincode")]
    pub fn from_bytes(bytes: &[u8]) -> Result<Self, std::boxed::Box<bincode::ErrorKind>> {
        bincode::deserialize(bytes)
    }

    pub fn from_raw(opcodes: &[u32], constants: &[f32]) -> Result<Self, UnknownOpcodeError> {
        // We use collect to convert from a Vec<Result<..>> to a Result<Vec<..>>. Neat!
        let opcodes = opcodes
            .iter()
            .map(|opcode| Opcode::try_from(*opcode))
            .collect::<Result<Vec<Opcode>, _>>()?;
        Ok(Self {
            opcodes,
            constants: constants.to_vec(),
        })
    }

    pub fn as_raw(&self) -> (Vec<u32>, Vec<f32>) {
        let opcodes = self
            .opcodes
            .iter()
            .map(|&opcode| opcode.into())
            .collect::<Vec<u32>>();
        (opcodes, self.constants.clone())
    }

    pub(crate) fn constant_push_vec2(&mut self, v: impl Into<[f32; 2]>) {
        self.constants.extend(v.into());
    }

    pub(crate) fn constant_push_vec3(&mut self, v: impl Into<[f32; 3]>) {
        self.constants.extend(v.into());
    }

    pub(crate) fn constant_push_vec4(&mut self, v: impl Into<[f32; 4]>) {
        self.constants.extend(v.into());
    }

    pub fn disassemble(&self) -> String {
        // Moved it to the compiler file, fits better there.
        crate::compiler::disassemble(&self.opcodes, &self.constants)
            .unwrap_or_else(|e| format!("(failed to disassemble: {:?}", e))
    }
}
