use auto_struct_macros::auto_struct;
use reader_writer::CStr;
use reader_writer::typenum::*;
use reader_writer::generic_array::GenericArray;
use crate::res_id:: *;
use crate::scly_props::structs::*;
use crate::SclyPropertyData;
use crate::scly_props::structs::*;
use crate::{impl_position, impl_rotation, impl_scale};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Debris<'r>
{
    #[auto_struct(expect = 24)]
    pub prop_count: u32,

    pub name: CStr<'r>,

    pub position: GenericArray<f32, U3>,
    pub rotation: GenericArray<f32, U3>,
    pub scale: GenericArray<f32, U3>,

    // TODO: Don't care
}

impl<'r> SclyPropertyData for Debris<'r>
{
    const OBJECT_TYPE: u8 = 0x1B;

    impl_position!();
    impl_rotation!();
    impl_scale!();
}
