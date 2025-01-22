use crate::{CmdlDataSection, CmdlMaterialSet};
use auto_struct_macros::auto_struct;
use reader_writer::generic_array::GenericArray;
use reader_writer::typenum::{U3, U6};
use reader_writer::{IteratorArray, LazyArray, RoArray, RoArrayIter};

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

    pub collision: CollisionIndexData<'r>,
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
#[derive(Debug, Clone, Copy)]
pub struct EdgeData {
    pub a: u16,
    pub b: u16,
}

#[auto_struct(Readable, Writable)]
#[derive(Debug, Clone, Copy)]
pub struct TriData {
    pub a: u16,
    pub b: u16,
    pub c: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CollisionMaterialFlags {
    Unknown = 0x0000_0001,
    Stone = 0x0000_0002,
    Metal = 0x0000_0004,
    Grass = 0x0000_0008,
    Ice = 0x0000_0010,
    Pillar = 0x0000_0020,
    MetalGrating = 0x0000_0040,
    Phazon = 0x0000_0080,
    Dirt = 0x0000_0100,
    Lava = 0x0000_0200,
    LavaStone = 0x0000_0400,
    Snow = 0x0000_0800,
    MudSlow = 0x0000_1000,
    Halfpipe = 0x0000_2000,
    Mud = 0x0000_4000,
    Glass = 0x0000_8000,
    Shield = 0x0001_0000,
    Sand = 0x0002_0000,
    ShootThru = 0x0004_0000,
    Solid = 0x0008_0000,
    /** Name from Metaforce, functionality unclear */
    NoPlatformCollision = 0x0010_0000,
    CameraThru = 0x0020_0000,
    Wood = 0x0040_0000,
    Organic = 0x0080_0000,
    /** Name from Metaforce, functionality unclear */
    NoEdgeCollision = 0x0100_0000,
    RedundantEdgeOrFlippedTri = 0x0200_0000,
    /** Seems to affect shadows/lights */
    SeeThru = 0x0400_0000,
    ScanThru = 0x0800_0000,
    AIWalkThru = 0x1000_0000,
    Ceiling = 0x2000_0000,
    Wall = 0x4000_0000,
    Floor = 0x8000_0000,
}

impl CollisionMaterialFlags {
    pub const MATERIAL_FLAGS: [CollisionMaterialFlags; 20] = [
        CollisionMaterialFlags::Unknown,
        CollisionMaterialFlags::Stone,
        CollisionMaterialFlags::Metal,
        CollisionMaterialFlags::Grass,
        CollisionMaterialFlags::Ice,
        CollisionMaterialFlags::Pillar,
        CollisionMaterialFlags::MetalGrating,
        CollisionMaterialFlags::Phazon,
        CollisionMaterialFlags::Dirt,
        CollisionMaterialFlags::Lava,
        CollisionMaterialFlags::LavaStone,
        CollisionMaterialFlags::Snow,
        CollisionMaterialFlags::MudSlow,
        CollisionMaterialFlags::Halfpipe,
        CollisionMaterialFlags::Mud,
        CollisionMaterialFlags::Glass,
        CollisionMaterialFlags::Shield,
        CollisionMaterialFlags::Sand,
        CollisionMaterialFlags::Wood,
        CollisionMaterialFlags::Organic,
    ];

    pub fn name(&self) -> &'static str {
        match self {
            CollisionMaterialFlags::Unknown => "Unknown",
            CollisionMaterialFlags::Stone => "Stone",
            CollisionMaterialFlags::Metal => "Metal",
            CollisionMaterialFlags::Grass => "Grass",
            CollisionMaterialFlags::Ice => "Ice",
            CollisionMaterialFlags::Pillar => "Pillar",
            CollisionMaterialFlags::MetalGrating => "MetalGrating",
            CollisionMaterialFlags::Phazon => "Phazon",
            CollisionMaterialFlags::Dirt => "Dirt",
            CollisionMaterialFlags::Lava => "Lava",
            CollisionMaterialFlags::LavaStone => "LavaStone",
            CollisionMaterialFlags::Snow => "Snow",
            CollisionMaterialFlags::MudSlow => "MudSlow",
            CollisionMaterialFlags::Halfpipe => "Halfpipe",
            CollisionMaterialFlags::Mud => "Mud",
            CollisionMaterialFlags::Glass => "Glass",
            CollisionMaterialFlags::Shield => "Shield",
            CollisionMaterialFlags::Sand => "Sand",
            CollisionMaterialFlags::ShootThru => "ShootThru",
            CollisionMaterialFlags::Solid => "Solid",
            CollisionMaterialFlags::NoPlatformCollision => "NoPlatformCollision",
            CollisionMaterialFlags::CameraThru => "CameraThru",
            CollisionMaterialFlags::Wood => "Wood",
            CollisionMaterialFlags::Organic => "Organic",
            CollisionMaterialFlags::NoEdgeCollision => "NoEdgeCollision",
            CollisionMaterialFlags::RedundantEdgeOrFlippedTri => "RedundantEdgeOrFlippedTri",
            CollisionMaterialFlags::SeeThru => "SeeThru",
            CollisionMaterialFlags::ScanThru => "ScanThru",
            CollisionMaterialFlags::AIWalkThru => "AIWalkThru",
            CollisionMaterialFlags::Ceiling => "Ceiling",
            CollisionMaterialFlags::Wall => "Wall",
            CollisionMaterialFlags::Floor => "Floor",
        }
    }
}
