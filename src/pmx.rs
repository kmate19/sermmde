use core::fmt;

use std::{
    io::{BufReader, Read},
    path::Path,
};

use thiserror::Error;

use crate::{
    surface, texture,
    types::{self, PmxText, TextEncoding},
    vertex,
};

#[derive(Debug, Error)]
pub enum Error {
    #[error("File had an invalid tag, did you input the correct file?")]
    InvalidTag,
    #[error("IO error: {0}")]
    IO(#[from] std::io::Error),
    #[error("Error parsing vertex: {0}")]
    VertexError(#[from] vertex::Error),
    #[error("PMX type error: {0}")]
    TypeError(#[from] types::Error),
    #[error("Invalid global variable amount, must be at least 8")]
    InvalidGlobalCount,
    #[error("Surface error: {0}")]
    SurfaceError(#[from] surface::Error),
    #[error("Texture error: {0}")]
    TextureError(#[from] texture::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

pub struct Pmx {
    header: Header,
    vertices: vertex::Vertices,
    surfaces: surface::Surfaces,
    textures: texture::Textures,
}

impl fmt::Debug for Pmx {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Pmx")
            .field("header", &self.header)
            .field(
                "vertices",
                &format!(
                    "<truncated, print the field separately if you want to see raw contents> (size: {})",
                    self.vertices.len()
                ),
            )
            .field("surfaces", &format!(
                "<truncated, print the field separately if you want to see raw contents> (size: {})",
                self.surfaces.len()
            ))
            .field("textures", &self.textures)
            .finish()
    }
}

impl Pmx {
    pub fn open(path: &Path) -> Result<Self> {
        let fh = std::fs::File::open(path)?;

        let mut reader = BufReader::new(fh);

        let header = Header::parse(&mut reader)?;

        let vertices = vertex::Vertices::parse(
            &mut reader,
            header.globals.vec4_additional,
            header.globals.bone_idx_size,
        )?;

        let surfaces = surface::Surfaces::parse(&mut reader, header.globals.vert_idx_size)?;

        let textures = texture::Textures::parse(&mut reader, header.globals.encoding)?;

        Ok(Pmx {
            header,
            vertices,
            surfaces,
            textures,
        })
    }
}

#[derive(Debug)]
pub struct Header {
    version: f32,
    globals: Globals,
    name: ModelName,
    comment: Comment,
}

#[derive(Debug)]
pub struct ModelName {
    pub local: PmxText,
    pub universal: PmxText,
}

#[derive(Debug)]
pub struct Comment {
    pub local: PmxText,
    pub universal: PmxText,
}

impl Header {
    pub fn parse(r: &mut impl Read) -> Result<Self> {
        // 4 bytes since there's a space after
        let mut tag = [0; 4];

        r.read_exact(&mut tag)?;

        if &tag[..3] != b"PMX" {
            Err(Error::InvalidTag)?
        }

        let mut ver = [0; 4];

        r.read_exact(&mut ver)?;

        let version = f32::from_le_bytes(ver);

        let globals = Globals::parse(r)?;

        let text_encoding = globals.encoding;

        let local_name = PmxText::from_bytes(r, text_encoding)?;

        let universal_name = PmxText::from_bytes(r, text_encoding)?;

        let name = ModelName {
            local: local_name,
            universal: universal_name,
        };

        let local_comment = PmxText::from_bytes(r, text_encoding)?;

        let universal_comment = PmxText::from_bytes(r, text_encoding)?;

        let comment = Comment {
            local: local_comment,
            universal: universal_comment,
        };

        Ok(Self {
            version,
            globals,
            name,
            comment,
        })
    }
}

#[derive(Debug)]
pub struct Globals {
    encoding: TextEncoding,
    vec4_additional: u8,
    vert_idx_size: u8,
    tex_idx_size: u8,
    material_idx_size: u8,
    bone_idx_size: u8,
    morph_idx_size: u8,
    rb_idx_size: u8,
    /// Store additional fields here that we don't know the specific purpose of right now.
    additional: Option<Vec<u8>>,
}

impl Globals {
    pub fn parse(r: &mut impl Read) -> Result<Self> {
        let mut global_count = [0; 1];

        r.read_exact(&mut global_count)?;

        if global_count[0] < 8 {
            Err(Error::InvalidGlobalCount)?
        }

        // note that global count is actually an i8, but since we already checked it's >= 8 we can assume its not negative.
        let mut globals = vec![0; global_count[0] as usize];

        r.read_exact(&mut globals)?;

        let additional = if global_count[0] > 8 {
            Some(globals.split_off(8))
        } else {
            None
        };

        Ok(Self {
            encoding: globals[0].try_into()?,
            vec4_additional: globals[1],
            vert_idx_size: globals[2],
            tex_idx_size: globals[3],
            material_idx_size: globals[4],
            bone_idx_size: globals[5],
            morph_idx_size: globals[6],
            rb_idx_size: globals[7],
            additional,
        })
    }
}
