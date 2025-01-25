use std::io;

use auto_struct_macros::auto_struct;
use reader_writer::{
    generic_array::GenericArray, typenum::*, IteratorArray, LCow, LazyArray, Readable, Reader,
    RoArray, RoArrayIter, Writable,
};
use crate::{AreaCollision, CmdlMaterialSet};
use crate::scly::Scly;

#[auto_struct(Readable, Writable)]
#[derive(Clone, Debug)]
pub struct Mrea<'r> {
    #[auto_struct(expect = 0xDEADBEEF)]
    magic: u32,

    #[auto_struct(expect = 0xF)]
    version: u32,

    pub area_transform: GenericArray<f32, U12>,
    pub world_model_count: u32,

    #[auto_struct(derive = sections.len() as u32)]
    sections_count: u32,

    pub world_geometry_section_idx: u32,
    pub scly_section_idx: u32,
    pub collision_section_idx: u32,
    pub unknown_section_idx: u32,
    pub lights_section_idx: u32,
    pub visibility_tree_section_idx: u32,
    pub path_section_idx: u32,
    pub area_octree_section_idx: u32,

    #[auto_struct(derive_from_iter = sections.iter()
            .map(&|i: LCow<MreaSection>| i.size() as u32))]
    #[auto_struct(init = (sections_count as usize, ()))]
    section_sizes: RoArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    #[auto_struct(init = section_sizes.iter())]
    pub sections: IteratorArray<'r, MreaSection<'r>, RoArrayIter<'r, u32>>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

impl<'r> Mrea<'r> {
    pub fn scly_section<'s>(&'s self) -> LCow<'s, Scly<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.scly_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(()))
            }
            LCow::Owned(MreaSection::Scly(scly)) => LCow::Owned(scly),
            LCow::Borrowed(MreaSection::Scly(scly)) => LCow::Borrowed(scly),
            _ => unreachable!(),
        }
    }

    pub fn scly_section_mut(&mut self) -> &mut Scly<'r> {
        self.sections.as_mut_vec()[self.scly_section_idx as usize].convert_to_scly()
    }

    pub fn collision_section<'s>(&'s self) -> LCow<'s, AreaCollision<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.collision_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(()))
            }
            LCow::Owned(MreaSection::Collision(collision)) => LCow::Owned(collision),
            LCow::Borrowed(MreaSection::Collision(collision)) => LCow::Borrowed(collision),
            _ => unreachable!(),
        }
    }

    pub fn collision_section_mut(&mut self) -> &mut AreaCollision<'r> {
        self.sections.as_mut_vec()[self.collision_section_idx as usize].convert_to_collision()
    }

    pub fn lights_section<'s>(&'s self) -> LCow<'s, Lights<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.lights_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(())),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(()))
            }
            LCow::Owned(MreaSection::Lights(lights)) => LCow::Owned(lights),
            LCow::Borrowed(MreaSection::Lights(lights)) => LCow::Borrowed(lights),
            _ => panic!(),
        }
    }

    pub fn lights_section_mut(&mut self) -> &mut Lights<'r> {
        self.sections.as_mut_vec()[self.lights_section_idx as usize].convert_to_lights()
    }

    pub fn materials_section<'s>(&'s self) -> LCow<'s, CmdlMaterialSet<'r>> {
        let section = self
            .sections
            .iter()
            .nth(self.world_geometry_section_idx as usize)
            .unwrap();
        match section {
            LCow::Owned(MreaSection::Unknown(ref reader)) => LCow::Owned(reader.clone().read(reader.len() as u32)),
            LCow::Borrowed(MreaSection::Unknown(ref reader)) => {
                LCow::Owned(reader.clone().read(reader.len() as u32))
            }
            LCow::Owned(MreaSection::Materials(materials)) => LCow::Owned(materials),
            LCow::Borrowed(MreaSection::Materials(materials)) => LCow::Borrowed(materials),
            _ => panic!(),
        }
    }

    pub fn materials_section_mut(&mut self) -> &mut CmdlMaterialSet<'r> {
        self.sections.as_mut_vec()[self.world_geometry_section_idx as usize].convert_to_materials()
    }
}

#[derive(Debug, Clone)]
pub enum MreaSection<'r> {
    Unknown(Reader<'r>),
    Scly(Scly<'r>),
    Collision(AreaCollision<'r>),
    Materials(CmdlMaterialSet<'r>),
    Lights(Lights<'r>),
}

impl<'r> MreaSection<'r> {
    pub fn convert_to_scly(&mut self) -> &mut Scly<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Scly(reader.clone().read(())),
            MreaSection::Scly(ref mut scly) => return scly,
            _ => panic!(),
        };
        match *self {
            MreaSection::Scly(ref mut scly) => scly,
            _ => panic!(),
        }
    }

    pub fn convert_to_lights(&mut self) -> &mut Lights<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Lights(reader.clone().read(())),
            MreaSection::Lights(ref mut lights) => return lights,
            _ => panic!(),
        };
        match *self {
            MreaSection::Lights(ref mut lights) => lights,
            _ => panic!(),
        }
    }

    pub fn convert_to_collision(&mut self) -> &mut AreaCollision<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Collision(reader.clone().read(())),
            MreaSection::Collision(ref mut collision) => return collision,
            _ => panic!(),
        };
        match *self {
            MreaSection::Collision(ref mut collision) => collision,
            _ => panic!(),
        }
    }

    pub fn convert_to_materials(&mut self) -> &mut CmdlMaterialSet<'r> {
        *self = match *self {
            MreaSection::Unknown(ref reader) => MreaSection::Materials(reader.clone().read(reader.len() as u32)),
            MreaSection::Materials(ref mut materials) => return materials,
            _ => panic!(),
        };
        match *self {
            MreaSection::Materials(ref mut materials) => materials,
            _ => panic!(),
        }
    }
}

impl<'r> Readable<'r> for MreaSection<'r> {
    type Args = u32;
    fn read_from(reader: &mut Reader<'r>, size: u32) -> Self {
        let res = MreaSection::Unknown(reader.truncated(size as usize));
        reader.advance(size as usize);
        res
    }

    fn size(&self) -> usize {
        match *self {
            MreaSection::Unknown(ref reader) => reader.len(),
            MreaSection::Scly(ref scly) => scly.size(),
            MreaSection::Collision(ref collision) => collision.size(),
            MreaSection::Lights(ref lights) => lights.size(),
            MreaSection::Materials(ref materials) => materials.size(),
        }
    }
}

impl<'r> Writable for MreaSection<'r> {
    fn write_to<W: io::Write>(&self, writer: &mut W) -> io::Result<u64> {
        match *self {
            MreaSection::Unknown(ref reader) => {
                writer.write_all(reader)?;
                Ok(reader.len() as u64)
            }
            MreaSection::Scly(ref scly) => scly.write_to(writer),
            MreaSection::Collision(ref collision) => collision.write_to(writer),
            MreaSection::Lights(ref lights) => lights.write_to(writer),
            MreaSection::Materials(ref materials) => materials.write_to(writer),
        }
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Lights<'r> {
    #[auto_struct(expect = 0xBABEDEAD)]
    magic: u32,

    #[auto_struct(derive = light_layers.len() as u32)]
    pub lights_count: u32,
    #[auto_struct(init = (lights_count as usize, ()))]
    pub light_layers: LazyArray<'r, LightLayer>,

    #[auto_struct(pad_align = 32)]
    _pad: (),
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct LightLayer {
    pub light_type: u32,

    pub color: GenericArray<f32, U3>,
    pub position: GenericArray<f32, U3>,
    pub direction: GenericArray<f32, U3>,

    pub brightness: f32,
    pub spot_cutoff: f32,
    pub unknown0: f32,
    pub unknown1: u8,
    pub unknown2: f32,
    pub falloff_type: u32,
    pub unknown3: f32,
}
