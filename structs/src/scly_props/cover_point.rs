use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::res_id:: *;
use crate::scly_props::structs::*;
use crate::SclyPropertyData;
use crate::scly_props::structs::*;
use crate::{impl_position, impl_rotation};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CoverPoint<'r>
{
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,

    pub active: u8,
    pub flags: u32,
    pub crouch: u8,
    pub horizontal_angle: f32, // TODO: rotation?
    pub vertical_angle: f32,
    pub cover_time: f32,
}

impl<'r> SclyPropertyData for CoverPoint<'r>
{
    const OBJECT_TYPE: u8 = 0x2A;

    impl_position!();
    impl_rotation!();
}
