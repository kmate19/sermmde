use std::{io::Read, vec};

use thiserror::Error;

use crate::pmx::{Error as PmxError, Result};

pub type Vec2 = [f32; 2];
pub type Vec3 = [f32; 3];
pub type Vec4 = [f32; 4];

#[derive(Debug)]
pub struct Vertices {
    inner: Vec<Vertex>,
    size: usize,
}

impl Vertices {
    pub fn len(&self) -> usize {
        self.size
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.inner
    }

    pub fn parse(reader: &mut impl Read, extra_vec4_count: u8, index_size: u8) -> Result<Self> {
        let mut size = [0; 4];

        reader.read_exact(&mut size)?;

        let size = i32::from_le_bytes(size);

        if size.is_negative() {
            // TODO(mate): better error
            Err(PmxError::InvalidEncoding)?
        }

        let size = size as _;

        let mut inner_vec = Vec::with_capacity(size);

        for _ in 0..size {
            let vert = Vertex::parse(reader, extra_vec4_count, index_size)?;
            inner_vec.push(vert);
        }

        Ok(Self {
            inner: inner_vec,
            size,
        })
    }
}

#[derive(Debug)]
pub struct Vertex {
    pos: Vec3,
    normal: Vec3,
    uv: Vec2,
    extra_vec4: Option<Vec<Vec4>>,
    weight_deform: WeightDeform,
    edge_scale: f32,
}

impl Vertex {
    pub fn parse(reader: &mut impl Read, extra_vec4_count: u8, index_size: u8) -> Result<Self> {
        let mut pos = [0; std::mem::size_of::<Vec3>()];

        reader.read_exact(&mut pos)?;

        let chunks = pos.as_chunks::<4>().0;

        let pos: Vec3 = [
            f32::from_le_bytes(chunks[0]),
            f32::from_le_bytes(chunks[1]),
            f32::from_le_bytes(chunks[2]),
        ];

        let mut normal = [0; std::mem::size_of::<Vec3>()];

        reader.read_exact(&mut normal)?;

        let chunks = normal.as_chunks::<4>().0;

        let normal: Vec3 = [
            f32::from_le_bytes(chunks[0]),
            f32::from_le_bytes(chunks[1]),
            f32::from_le_bytes(chunks[2]),
        ];

        let mut uv = [0; std::mem::size_of::<Vec2>()];

        reader.read_exact(&mut uv)?;

        let chunks = uv.as_chunks::<4>().0;

        let uv: Vec2 = [f32::from_le_bytes(chunks[0]), f32::from_le_bytes(chunks[1])];

        let mut vec4s = if extra_vec4_count != 0 {
            Some(vec![Vec4::default(); extra_vec4_count as _])
        } else {
            None
        };

        if vec4s.is_some() {
            for vec in vec4s.as_mut().unwrap() {
                let mut vec_bytes = [0; std::mem::size_of::<Vec4>()];
                reader.read_exact(&mut vec_bytes)?;

                let chunks = vec_bytes.as_chunks::<4>().0;

                vec[0] = f32::from_le_bytes(chunks[0]);
                vec[1] = f32::from_le_bytes(chunks[1]);
                vec[2] = f32::from_le_bytes(chunks[2]);
                vec[3] = f32::from_le_bytes(chunks[3]);
            }
        }

        let mut weight_deform_type = [0; 1];

        reader.read_exact(&mut weight_deform_type)?;

        let size: IndexSize = index_size.try_into()?;

        let mut weight_deform = match weight_deform_type[0] {
            0 => WeightDeform::BDEF1 {
                index: BoneIndex::new(size),
            },
            1 => WeightDeform::BDEF2 {
                indices: [BoneIndex::new(size); 2],
                weights: [0.0; 2],
            },
            2 => WeightDeform::BDEF4 {
                indices: [BoneIndex::new(size); 4],
                weights: [0.0; 4],
            },
            3 => unimplemented!("SDEF4 is currently unsupported"),
            4 => unimplemented!("QDEF is currently unsupported"),
            _ => {
                dbg!("Invalid weight deform type", weight_deform_type[0]);
                Err(PmxError::InvalidEncoding)?
            }
        };

        weight_deform.parse(reader)?;

        let mut edge_scale = [0; 4];

        reader.read_exact(&mut edge_scale)?;

        let edge_scale = f32::from_le_bytes(edge_scale);

        Ok(Self {
            pos,
            normal,
            uv,
            extra_vec4: vec4s,
            weight_deform,
            edge_scale,
        })
    }
}

#[derive(Debug, Copy, Clone)]
pub enum IndexSize {
    Size1([u8; 1]),
    Size2([u8; 2]),
    Size4([u8; 4]),
}

impl TryFrom<u8> for IndexSize {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            1 => Ok(Self::Size1([0; 1])),
            2 => Ok(Self::Size2([0; 2])),
            4 => Ok(Self::Size4([0; 4])),
            _ => Err(Error::IndexSizeMismatch),
        }
    }
}

#[derive(Debug, Error)]
pub enum Error {
    #[error("The index size mismatched")]
    IndexSizeMismatch,
}

#[derive(Debug, Copy, Clone)]
pub struct BoneIndex {
    size: IndexSize,
}

impl BoneIndex {
    pub fn new(size: IndexSize) -> Self {
        Self { size }
    }

    pub fn fill(&mut self, reader: &mut impl Read) -> Result<()> {
        match &mut self.size {
            IndexSize::Size1(raw) => reader.read_exact(raw)?,
            IndexSize::Size2(raw) => reader.read_exact(raw)?,
            IndexSize::Size4(raw) => reader.read_exact(raw)?,
        }
        Ok(())
    }

    // TODO(mate): probably move this to a field
    pub fn value(&self) -> i32 {
        match &self.size {
            IndexSize::Size1(raw) => i8::from_le_bytes(*raw) as i32,
            IndexSize::Size2(raw) => i16::from_le_bytes(*raw) as i32,
            IndexSize::Size4(raw) => i32::from_le_bytes(*raw),
        }
    }

    pub fn is_nil(&self) -> bool {
        self.value() == -1
    }
}

#[derive(Debug)]
pub enum WeightDeform {
    BDEF1 {
        index: BoneIndex,
    },
    BDEF2 {
        indices: [BoneIndex; 2],
        weights: [f32; 2],
    },
    BDEF4 {
        indices: [BoneIndex; 4],
        weights: [f32; 4],
    },
    // TODO(mate)
    // SDEF
    // QDEF
}

impl WeightDeform {
    pub fn parse(&mut self, reader: &mut impl Read) -> Result<()> {
        match self {
            WeightDeform::BDEF1 { index } => {
                index.fill(reader)?;
            }
            WeightDeform::BDEF2 { indices, weights } => {
                indices[0].fill(reader)?;
                indices[1].fill(reader)?;

                // We only need to read 4 bytes here, because the 2nd weight is not in the file
                // but calculated from the first weight
                let mut weights_bytes = [0; std::mem::size_of::<f32>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = 1.0 - weights[0];
            }
            WeightDeform::BDEF4 { indices, weights } => {
                indices[0].fill(reader)?;
                indices[1].fill(reader)?;
                indices[2].fill(reader)?;
                indices[3].fill(reader)?;

                let mut weights_bytes = [0; std::mem::size_of::<[f32; 4]>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = f32::from_le_bytes(chunks[1]);
                weights[2] = f32::from_le_bytes(chunks[2]);
                weights[3] = f32::from_le_bytes(chunks[3]);
            }
        }
        Ok(())
    }
}
