use auto_struct_macros::auto_struct;
use reader_writer::generic_array::GenericArray;
use reader_writer::{IteratorArray, LazyArray, RoArray, RoArrayIter};
use reader_writer::typenum::{U3, U6};
use crate::{CmdlDataSection, CmdlMaterialSet};

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct AreaCollision<'r> {
  #[auto_struct(expect = 0x01000000)]
  unknown: u32,

  section_size: u32,

  #[auto_struct(expect = 0xDEAFBABE)]
  magic: u32,

  #[auto_struct(expect = 3)]
  version: u32,

  pub aabb: GenericArray<f32, U6>,

  pub root_node_type: u32,
  pub octree_size: u32,

  // TODO: parse maybe?
  #[auto_struct(init = (octree_size as usize, ()))]
  pub octtree: RoArray<'r, u8>,

  pub collision: CollisionIndexData<'r>
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct CollisionIndexData<'r> {
  collision_material_count: u32,

  #[auto_struct(init = (collision_material_count as usize, ()))]
  pub collision_materials: LazyArray<'r, u32>,

  pub vertex_material_index_count: u32,

  #[auto_struct(init = (vertex_material_index_count as usize, ()))]
  pub vertex_material_indices: LazyArray<'r, u8>,

  pub edge_material_index_count: u32,

  #[auto_struct(init = (edge_material_index_count as usize, ()))]
  pub edge_material_indices: LazyArray<'r, u8>,

  pub tri_material_index_count: u32,

  #[auto_struct(init = (tri_material_index_count as usize, ()))]
  pub tri_material_indices: LazyArray<'r, u8>,

  pub edge_count: u32,

  #[auto_struct(init = (edge_count as usize, ()))]
  pub edges: LazyArray<'r, EdgeData>,

  pub tri_count: u32,

  #[auto_struct(init = ((tri_count / 3) as usize, ()))]
  pub tris: LazyArray<'r, TriData>,

  pub vert_count: u32,

  #[auto_struct(init = (vert_count as usize, ()))]
  pub verts: LazyArray<'r, GenericArray<f32, U3>>,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct EdgeData {
  a: u16,
  b: u16,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone)]
pub struct TriData {
  a: u16,
  b: u16,
  c: u16,
}