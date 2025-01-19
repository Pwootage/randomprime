use crate::{res_id::*, ResId};
use auto_struct_macros::auto_struct;
use reader_writer::byteorder::ReadBytesExt;
use reader_writer::{
    generic_array::GenericArray, typenum::*, IteratorArray, LazyArray, Readable, Reader, RoArray,
    RoArrayIter, Writable,
};
use std::io::Write;

// We don't need to modify CMDLs, so most of the details are left out.
// We only actually care about reading out the TXTR file ids.
#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct Cmdl<'r> {
    #[auto_struct(expect = 0xDEADBABE)]
    magic: u32,

    #[auto_struct(expect = 2)]
    version: u32,

    pub flags: u32,

    pub maab: GenericArray<f32, U6>,

    pub data_section_count: u32,
    pub material_set_count: u32,

    // TODO: Iter derive
    #[auto_struct(init = (material_set_count as usize, ()))]
    pub material_set_sizes: RoArray<'r, u32>,
    #[auto_struct(init = ((data_section_count - material_set_count) as usize, ()))]
    pub data_section_sizes: RoArray<'r, u32>,

    #[auto_struct(pad_align = 32)]
    _pad: (),

    #[auto_struct(init = material_set_sizes.iter())]
    pub material_sets: IteratorArray<'r, CmdlMaterialSet<'r>, RoArrayIter<'r, u32>>,
    #[auto_struct(init = data_section_sizes.iter())]
    pub data_sections: IteratorArray<'r, CmdlDataSection<'r>, RoArrayIter<'r, u32>>,
}



#[derive(Debug, Clone)]
pub struct CmdlMaterialSet<'r> {
    pub texture_ids: LazyArray<'r, ResId<TXTR>>,

    pub material_ends: RoArray<'r, u32>,
    pub materials: LazyArray<'r, CmdlMaterial<'r>>,
}
impl<'r> Readable<'r> for CmdlMaterialSet<'r> {
    type Args = u32;
    fn read_from(reader: &mut Reader<'r>, size: Self::Args) -> Self {
        let texture_count: u32 = reader.read(());
        let texture_ids: LazyArray<'r, ResId<TXTR>> = reader.read((texture_count as usize, ()));
        let material_count: u32 = reader.read(());
        let material_ends: RoArray<'r, u32> = reader.read((material_count as usize, ()));
        let materials: LazyArray<'r, CmdlMaterial<'r>> = reader.read((material_count as usize, ()));
        CmdlMaterialSet { texture_ids, material_ends, materials }
    }
    fn size(&self) -> usize {
        let mut sum = 0;
        sum += 4; // texture_count
        sum += self.texture_ids.size();
        sum += 4; // material_count
        sum += self.material_ends.size();
        sum += self.materials.size();
        sum
    }
}
impl<'r> Writable for CmdlMaterialSet<'r> {
    fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<u64> {
        let mut sum = 0;
        sum += (self.texture_ids.len() as u32).write_to(writer)?;
        sum += self.texture_ids.write_to(writer)?;
        sum += (self.material_ends.len() as u32).write_to(writer)?;
        sum += self.material_ends.write_to(writer)?; // TODO: generate these
        sum += self.materials.write_to(writer)?;
        Ok(sum)
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlDataSection<'r> {
    #[auto_struct(args)]
    size: u32,

    #[auto_struct(init = (size as usize, ()))]
    pub remainder: RoArray<'r, u8>,
}

#[derive(Debug, Clone)]
pub struct CmdlMaterial<'r> {
    pub flags: u32,
    pub texture_indicies: RoArray<'r, u32>,

    pub vertex_attribute_flags: u32,
    pub group_index: u32,

    pub konsts: RoArray<'r, u32>,

    pub blend_destination_factor: u16,
    pub blend_source_factor: u16,

    pub reflection_indirect_texture_slot_index: Option<u32>,

    pub color_channel_flags: RoArray<'r, u32>,

    pub tev_stages: RoArray<'r, CmdlTevStage>,
    pub tev_stage_texture_inputs: RoArray<'r, CmdlTevStageTextureInput>,

    pub texgen_flags: RoArray<'r, u32>,

    pub uv_animations: RoArray<'r, CmdlUvAnimation>,
}

impl<'r> Readable<'r> for CmdlMaterial<'r> {
    type Args = ();

    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self {
        let flags: u32 = reader.read(());
        let texture_count: u32 = reader.read(());
        let texture_indicies: RoArray<'r, u32> = reader.read((texture_count as usize, ()));
        let vertex_attribute_flags: u32 = reader.read(());
        let group_index: u32 = reader.read(());
        // only if konst flag is set
        let konst_count: u32 = if flags & 0x8 != 0 {
            reader.read(())
        } else { 0 };
        let konsts: RoArray<'r, u32> = reader.read((konst_count as usize, ()));
        let blend_destination_factor: u16 = reader.read(());
        let blend_source_factor: u16 = reader.read(());
        // only if indirect texture flag is set
        let reflection_indirect_texture_slot_index: Option<u32> = if flags & 0x400 != 0 {
            Some(reader.read(()))
        } else {
            None
        };
        let color_channel_count: u32 = reader.read(());
        let color_channel_flags: RoArray<'r, u32> = reader.read((color_channel_count as usize, ()));
        let tev_stage_count: u32 = reader.read(());
        let tev_stages: RoArray<'r, CmdlTevStage> = reader.read((tev_stage_count as usize, ()));
        let tev_stage_texture_inputs: RoArray<'r, CmdlTevStageTextureInput> =
            reader.read((tev_stage_count as usize, ()));

        let texgen_count: u32 = reader.read(());
        let texgen_flags: RoArray<'r, u32> = reader.read((texgen_count as usize, ()));

        let _: u32 = reader.read(()); // animation section size
        let uv_animation_count: u32 = reader.read(());
        let uv_animations: RoArray<'r, CmdlUvAnimation> = reader.read((uv_animation_count as usize, ()));

        CmdlMaterial {
            flags,
            texture_indicies,
            vertex_attribute_flags,
            group_index,
            konsts,
            blend_destination_factor,
            blend_source_factor,
            reflection_indirect_texture_slot_index,
            color_channel_flags,
            tev_stages,
            tev_stage_texture_inputs,
            texgen_flags,
            uv_animations,
        }
    }

    fn size(&self) -> usize {
        let mut sum = 0;
        /* flags */ sum += 4;
        /* texture_count */ sum += 4;
        /* texture_indicies */ sum += self.texture_indicies.size();
        /* vertex_attribute_flags */ sum += 4;
        /* group_index */ sum += 4;
        if self.flags & 0x8 != 0 {
            /* konst_count */ sum += 4;
            /* konsts */ sum += self.konsts.size();
        }
        /* blend_destination_factor */ sum += 2;
        /* blend_source_factor */ sum += 2;
        if self.flags & 0x400 != 0 {
            /* reflection_indirect_texture_slot_index */ sum += 4;
        }
        /* color_channel_count */ sum += 4;
        /* color_channel_flags */ sum += self.color_channel_flags.size();
        /* tev_stage_count */ sum += 4;
        /* tev_stages */ sum += self.tev_stages.size();
        /* tev_stage_texture_inputs */ sum += self.tev_stage_texture_inputs.size();
        /* texgen_count */ sum += 4;
        /* texgen_flags */ sum += self.texgen_flags.size();
        /* animation section size */ sum += 4;
        /* uv_animation_count */ sum += 4;
        /* uv_animations */ sum += self.uv_animations.size();
        sum
    }
}

impl<'r> Writable for CmdlMaterial<'r> {
    fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<u64> {
        let mut sum = 0;
        sum += self.flags.write_to(writer)?;
        sum += (self.texture_indicies.len() as u32).write_to(writer)?;
        sum += self.texture_indicies.write_to(writer)?;
        sum += self.vertex_attribute_flags.write_to(writer)?;
        sum += self.group_index.write_to(writer)?;
        sum += (self.konsts.len() as u32).write_to(writer)?;
        sum += self.konsts.write_to(writer)?;
        sum += self.blend_destination_factor.write_to(writer)?;
        sum += self.blend_source_factor.write_to(writer)?;
        if self.flags & 0x400 != 0 {
            sum += self.reflection_indirect_texture_slot_index.unwrap().write_to(writer)?;
        }
        sum += (self.color_channel_flags.len() as u32).write_to(writer)?;
        sum += self.color_channel_flags.write_to(writer)?;
        sum += (self.tev_stages.len() as u32).write_to(writer)?;
        sum += self.tev_stages.write_to(writer)?;
        sum += self.tev_stage_texture_inputs.write_to(writer)?;
        sum += (self.texgen_flags.len() as u32).write_to(writer)?;
        sum += self.texgen_flags.write_to(writer)?;
        sum += ((self.uv_animations.size() + 4) as u32).write_to(writer)?;
        sum += (self.uv_animations.len() as u32).write_to(writer)?;
        sum += self.uv_animations.write_to(writer)?;
        Ok(sum)
    }
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlTevStage {
    pub color_in_flags: u32,
    pub alpha_in_flags: u32,
    pub color_combine_flags: u32,
    pub alpha_combine_flags: u32,
    pub padding: u8,
    pub konst_alpha_input: u8,
    pub konst_color_input: u8,
    pub rasterized_color_input: u8,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CmdlTevStageTextureInput {
    pub padding: u16,
    pub texture_input: u8,
    pub tex_coord_input: u8,
}

#[derive(Debug, Clone)]
pub enum CmdlUvAnimation {
    InvsereModelViewMatrixNoTranslation,
    InverseModelViewMatrix,
    UVScroll { offset_a: f32, offset_b: f32, scale_a: f32, scale_b: f32 },
    Rotation { offset: f32, scale: f32 },
    HorizontalFilmstrip { scale: f32, num_frames: f32, step: f32, offset: f32 },
    VerticalFilmstrip { scale: f32, num_frames: f32, step: f32, offset: f32 },
    ModelMatrix,
    CylinderEnvironment { param_a: f32, param_b: f32 },
}

impl <'r> Readable<'r> for CmdlUvAnimation {
    type Args = ();

    fn read_from(reader: &mut Reader<'r>, args: Self::Args) -> Self {
        let mode: u32 = reader.read(());
        match mode {
            0 => CmdlUvAnimation::InvsereModelViewMatrixNoTranslation,
            1 => CmdlUvAnimation::InverseModelViewMatrix,
            2 => {
                let offset_a = reader.read(());
                let offset_b = reader.read(());
                let scale_a = reader.read(());
                let scale_b = reader.read(());
                CmdlUvAnimation::UVScroll { offset_a, offset_b, scale_a, scale_b }
            },
            3 => {
                let offset = reader.read(());
                let scale = reader.read(());
                CmdlUvAnimation::Rotation { offset, scale }
            },
            4 => {
                let scale = reader.read(());
                let num_frames = reader.read(());
                let step = reader.read(());
                let offset = reader.read(());
                CmdlUvAnimation::HorizontalFilmstrip { scale, num_frames, step, offset }
            },
            5 => {
                let scale = reader.read(());
                let num_frames = reader.read(());
                let step = reader.read(());
                let offset = reader.read(());
                CmdlUvAnimation::VerticalFilmstrip { scale, num_frames, step, offset }
            },
            6 => CmdlUvAnimation::ModelMatrix,
            7 => {
                let param_a = reader.read(());
                let param_b = reader.read(());
                CmdlUvAnimation::CylinderEnvironment { param_a, param_b }
            },
            _ => panic!("Unknown UV animation mode: {}", mode),
        }
    }

    fn size(&self) -> usize {
        4 + match self {
            CmdlUvAnimation::InvsereModelViewMatrixNoTranslation => 0,
            CmdlUvAnimation::InverseModelViewMatrix => 0,
            CmdlUvAnimation::UVScroll { .. } => 16,
            CmdlUvAnimation::Rotation { .. } => 8,
            CmdlUvAnimation::HorizontalFilmstrip { .. } => 16,
            CmdlUvAnimation::VerticalFilmstrip { .. } => 16,
            CmdlUvAnimation::ModelMatrix => 0,
            CmdlUvAnimation::CylinderEnvironment { .. } => 8,
        }
    }
}

impl Writable for CmdlUvAnimation {
    fn write_to<W: Write>(&self, writer: &mut W) -> std::io::Result<u64> {
        let mut sum = 0;
        match self {
            CmdlUvAnimation::InvsereModelViewMatrixNoTranslation => {
                sum += 0u32.write_to(writer)?;
            },
            CmdlUvAnimation::InverseModelViewMatrix => {
                sum += 1u32.write_to(writer)?;
            },
            CmdlUvAnimation::UVScroll { offset_a, offset_b, scale_a, scale_b } => {
                sum += 2u32.write_to(writer)?;
                sum += offset_a.write_to(writer)?;
                sum += offset_b.write_to(writer)?;
                sum += scale_a.write_to(writer)?;
                sum += scale_b.write_to(writer)?;
            },
            CmdlUvAnimation::Rotation { offset, scale } => {
                sum += 3u32.write_to(writer)?;
                sum += offset.write_to(writer)?;
                sum += scale.write_to(writer)?;
            },
            CmdlUvAnimation::HorizontalFilmstrip { scale, num_frames, step, offset } => {
                sum += 4u32.write_to(writer)?;
                sum += scale.write_to(writer)?;
                sum += num_frames.write_to(writer)?;
                sum += step.write_to(writer)?;
                sum += offset.write_to(writer)?;
            },
            CmdlUvAnimation::VerticalFilmstrip { scale, num_frames, step, offset } => {
                sum += 5u32.write_to(writer)?;
                sum += scale.write_to(writer)?;
                sum += num_frames.write_to(writer)?;
                sum += step.write_to(writer)?;
                sum += offset.write_to(writer)?;
            },
            CmdlUvAnimation::ModelMatrix => {
                sum += 6u32.write_to(writer)?;
            },
            CmdlUvAnimation::CylinderEnvironment { param_a, param_b } => {
                sum += 7u32.write_to(writer)?;
                sum += param_a.write_to(writer)?;
                sum += param_b.write_to(writer)?;
            },
        }
        Ok(sum)
    }
}