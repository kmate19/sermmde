use std::io::Read;

use thiserror::Error;

use crate::types::{PmxText, TextEncoding};

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
pub struct Textures {
    len: usize,
    inner: Vec<Texture>,
}

impl Textures {
    pub fn len(&self) -> usize {
        let len = self.inner.len();
        debug_assert!(self.len == len);
        len
    }

    pub fn parse(reader: &mut impl Read, encoding: TextEncoding) -> Result<Self> {
        let mut size_bytes = [0; 4];

        reader.read_exact(&mut size_bytes)?;

        let size = i32::from_le_bytes(size_bytes);

        if size.is_negative() {
            Err(Error::NegativeSize)?
        }

        let size = size as usize;

        let mut inner_vec = Vec::with_capacity(size);

        for _ in 0..size {
            let tex = Texture::parse(reader, encoding)?;
            inner_vec.push(tex);
        }

        Ok(Self {
            len: size,
            inner: inner_vec,
        })
    }
}

#[derive(Debug)]
pub struct Texture {
    path: PmxText,
}

impl Texture {
    pub fn parse(reader: &mut impl Read, encoding: TextEncoding) -> Result<Self> {
        let path = PmxText::from_bytes(reader, encoding)?;

        Ok(Self { path })
    }
}
