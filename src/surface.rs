use std::io::Read;

use thiserror::Error;

use crate::types::Index;

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
pub struct Surfaces {
    len: usize,
    inner: Vec<Surface>,
}

impl Surfaces {
    pub fn len(&self) -> usize {
        let len = self.inner.len();
        debug_assert!(self.len == len);
        len
    }

    pub fn parse(reader: &mut impl Read, index_size: u8) -> Result<Self> {
        let mut size_bytes = [0; 4];

        reader.read_exact(&mut size_bytes)?;

        let size = i32::from_le_bytes(size_bytes);

        if size.is_negative() {
            Err(Error::NegativeSize)?
        }

        let size = size as usize;

        let mut inner_vec = Vec::with_capacity(size);

        for _ in 0..size {
            let surf = Surface::parse(reader, index_size)?;
            inner_vec.push(surf);
        }

        Ok(Self {
            len: size,
            inner: inner_vec,
        })
    }
}

#[derive(Debug)]
pub struct Surface {
    index: Index,
}

impl Surface {
    pub fn parse(reader: &mut impl Read, index_size: u8) -> Result<Self> {
        let index = Index::parse(reader, index_size.try_into()?, false)?;

        Ok(Self { index })
    }
}
