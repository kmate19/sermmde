use std::{io::Read, vec};

use thiserror::Error;

use crate::types::{Index, IndexSize, Vec2, Vec3, Vec4};

#[derive(Debug, Error)]
pub enum Error {
    #[error("The index size mismatched")]
    IndexSizeMismatch,
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Negative size encountered where positive expected")]
    NegativeSize,
    #[error("Invalid weight deform type encountered")]
    InvalidWeightDeformType,
    #[error(transparent)]
    TypeError(#[from] crate::types::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Vertices {
    inner: Vec<Vertex>,
    size: usize,
}

impl Vertices {
    pub fn len(&self) -> usize {
        let len = self.inner.len();
        debug_assert!(self.size == len);
        len
    }

    pub fn vertices(&self) -> &[Vertex] {
        &self.inner
    }

    pub fn parse(reader: &mut impl Read, extra_vec4_count: u8, index_size: u8) -> Result<Self> {
        let mut size = [0; 4];

        reader.read_exact(&mut size)?;

        let size = i32::from_le_bytes(size);

        if size.is_negative() {
            Err(Error::NegativeSize)?
        }

        let size = size as usize;

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
        let mut pos_bytes = [0; std::mem::size_of::<Vec3>()];

        reader.read_exact(&mut pos_bytes)?;

        let chunks = pos_bytes.as_chunks::<4>().0;

        let pos_floats = [
            f32::from_le_bytes(chunks[0]),
            f32::from_le_bytes(chunks[1]),
            f32::from_le_bytes(chunks[2]),
        ];

        let pos = pos_floats.into();

        let mut normal_bytes = [0; std::mem::size_of::<Vec3>()];

        reader.read_exact(&mut normal_bytes)?;

        let chunks = normal_bytes.as_chunks::<4>().0;

        let normal_floats = [
            f32::from_le_bytes(chunks[0]),
            f32::from_le_bytes(chunks[1]),
            f32::from_le_bytes(chunks[2]),
        ];

        let normal = normal_floats.into();

        let mut uv_bytes = [0; std::mem::size_of::<Vec2>()];

        reader.read_exact(&mut uv_bytes)?;

        let chunks = uv_bytes.as_chunks::<4>().0;

        let uv_floats = [f32::from_le_bytes(chunks[0]), f32::from_le_bytes(chunks[1])];

        let uv = uv_floats.into();

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

        let weight_deform = WeightDeform::parse(reader, weight_deform_type[0], size, true)?;

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

#[derive(Debug)]
pub enum WeightDeform {
    // ver 2.0
    BDEF1 {
        index: Index,
    },
    // ver 2.0
    BDEF2 {
        indices: [Index; 2],
        // Only 1 actual weight is stored in the file, the other is calculated from it
        weights: [f32; 2],
    },
    // ver 2.0
    BDEF4 {
        indices: [Index; 4],
        weights: [f32; 4],
    },
    /// Spherical deform blending
    // ver 2.0
    SDEF {
        indices: [Index; 2],
        // Only 1 actual weight is stored in the file, the other is calculated from it
        weights: [f32; 2],
        // these fields are unsure?
        c: Vec3,
        r0: Vec3,
        r1: Vec3,
    },
    /// Dual quaternion deform blending
    // unsure if this is correct also
    // ver 2.1
    QDEF {
        indices: [Index; 4],
        weights: [f32; 4],
    },
}

impl WeightDeform {
    pub fn parse(
        reader: &mut impl Read,
        typ: u8,
        size: IndexSize,
        index_sign: bool,
    ) -> Result<Self> {
        match typ {
            0 => {
                let index = Index::parse(reader, size, index_sign)?;

                Ok(WeightDeform::BDEF1 { index })
            }
            1 => {
                let indices = [
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                ];

                let mut weights = [0.0; 2];

                // We only need to read 4 bytes here, because the 2nd weight is not in the file
                // but calculated from the first weight
                let mut weights_bytes = [0; std::mem::size_of::<f32>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = 1.0 - weights[0];

                Ok(WeightDeform::BDEF2 { indices, weights })
            }
            2 => {
                let indices = [
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                ];

                let mut weights = [0.0; 4];

                let mut weights_bytes = [0; std::mem::size_of::<[f32; 4]>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = f32::from_le_bytes(chunks[1]);
                weights[2] = f32::from_le_bytes(chunks[2]);
                weights[3] = f32::from_le_bytes(chunks[3]);

                Ok(WeightDeform::BDEF4 { indices, weights })
            }
            3 => {
                let indices = [
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                ];

                let mut weights = [0.0; 2];
                // We only need to read 4 bytes here, because the 2nd weight is not in the file
                // but calculated from the first weight
                let mut weights_bytes = [0; std::mem::size_of::<f32>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = 1.0 - weights[0];

                let mut c = Vec3::default();
                let mut r0 = Vec3::default();
                let mut r1 = Vec3::default();

                for i in 0..3 {
                    let mut vec_bytes = [0; std::mem::size_of::<Vec3>()];

                    reader.read_exact(&mut vec_bytes)?;

                    let chunks = vec_bytes.as_chunks::<4>().0;

                    let vec_floats = [
                        f32::from_le_bytes(chunks[0]),
                        f32::from_le_bytes(chunks[1]),
                        f32::from_le_bytes(chunks[2]),
                    ];

                    let vec: Vec3 = vec_floats.into();

                    match i {
                        0 => c = vec,
                        1 => r0 = vec,
                        2 => r1 = vec,
                        _ => unreachable!(),
                    }
                }

                Ok(WeightDeform::SDEF {
                    indices,
                    weights,
                    c,
                    r0,
                    r1,
                })
            }
            4 => {
                let indices = [
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                    Index::parse(reader, size, index_sign)?,
                ];

                let mut weights = [0.0; 4];

                let mut weights_bytes = [0; std::mem::size_of::<[f32; 4]>()];
                reader.read_exact(&mut weights_bytes)?;

                let chunks = weights_bytes.as_chunks::<4>().0;

                weights[0] = f32::from_le_bytes(chunks[0]);
                weights[1] = f32::from_le_bytes(chunks[1]);
                weights[2] = f32::from_le_bytes(chunks[2]);
                weights[3] = f32::from_le_bytes(chunks[3]);

                Ok(WeightDeform::QDEF { indices, weights })
            }
            _ => Err(Error::InvalidWeightDeformType)?,
        }
    }
}
