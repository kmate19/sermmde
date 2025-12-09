use std::io::Read;

use thiserror::Error;

use crate::types::{Flag, Index, PmxText, TextEncoding, Vec3, Vec4, vec_from_bytes};

#[derive(Debug, Error)]
pub enum Error {
    #[error("Negative size encountered where positive expected")]
    NegativeSize,
    #[error(transparent)]
    Type(#[from] crate::types::Error),
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

type Result<T> = std::result::Result<T, Error>;

#[derive(Debug)]
pub struct Materials {
    len: usize,
    inner: Vec<Material>,
}

impl Materials {
    pub fn len(&self) -> usize {
        let len = self.inner.len();
        debug_assert!(self.len == len);
        len
    }

    pub fn parse(reader: &mut impl Read, index_size: u8, encoding: TextEncoding) -> Result<Self> {
        let mut size_bytes = [0; 4];

        reader.read_exact(&mut size_bytes)?;

        let size = i32::from_le_bytes(size_bytes);

        if size.is_negative() {
            Err(Error::NegativeSize)?
        }

        let size = size as usize;

        let mut inner_vec = Vec::with_capacity(size);

        for _ in 0..size {
            let mat = Material::parse(reader, index_size, encoding)?;
            inner_vec.push(mat);
        }

        Ok(Self {
            len: size,
            inner: inner_vec,
        })
    }
}

#[derive(Debug)]
pub struct Material {
    name: Name,
    diffuse: Vec4,
    specular: Vec3,
    specular_strength: f32,
    ambient: Vec3,
    flags: Flag,
    edge_color: Vec4,
    edge_scale: f32,
    tex_idx: Index,
    env_idx: Index,
    env_blend: EnvironmentBlend,
    toon: Toon,
    meta: PmxText,
    surface_count: i32,
}

#[derive(Debug)]
pub enum Toon {
    Texture(Index),
    Internal(u8),
}

#[derive(Debug)]
pub enum EnvironmentBlend {
    None,
    Multiply,
    Add,
    Additional(Vec4),
}

#[derive(Debug)]
struct Name {
    local: PmxText,
    universal: PmxText,
}

impl Material {
    pub fn parse(reader: &mut impl Read, index_size: u8, encoding: TextEncoding) -> Result<Self> {
        let name = {
            let local = PmxText::from_bytes(reader, encoding)?;
            let universal = PmxText::from_bytes(reader, encoding)?;
            Name { local, universal }
        };

        let diffuse: Vec4 = vec_from_bytes!(Vec4, reader);
        let specular: Vec3 = vec_from_bytes!(Vec3, reader);

        unimplemented!();
    }
}
