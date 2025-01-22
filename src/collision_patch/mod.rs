use crate::patcher::PrimePatcher;
use crate::pickup_meta;
use memmap::Mmap;
use reader_writer::{FourCC, LCow, Readable, Reader, Writable};
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;
use structs::Resource;

mod model_creation;

pub fn patch_iso_collision<T>(input_iso: Mmap, output_iso: File, mut pn: T) -> Result<(), String>
where
    T: structs::ProgressNotifier,
{
    let start_time = Instant::now();
    let mut reader = Reader::new(&input_iso[..]);
    let mut gc_disc: structs::GcDisc = reader.read(());

    build_and_run_collision_patches(&mut gc_disc)?;

    println!("Created patches in {:?}", start_time.elapsed());

    // temporarily disable disk writes so I don't eat up all my disk io while testing
    // let mut file = output_iso;
    // file.set_len(structs::GC_DISC_LENGTH as u64)
    //     .map_err(|e| format!("Failed to resize output file: {}", e))?;
    // gc_disc
    //     .write(&mut file, &mut pn)
    //     .map_err(|e| format!("Error writing output file: {}", e))?;
    pn.notify_flushing_to_disk();
    Ok(())
}

fn build_and_run_collision_patches<'r>(gc_disc: &mut structs::GcDisc<'r>) -> Result<(), String> {
    let mut patcher = PrimePatcher::new();
    // patcher.add_file_patch(b"opening.bnr", |file| crate::patches::patch_bnr(file, &config.game_banner));
    for (pak_name, rooms) in pickup_meta::ROOM_INFO.iter() {
        for room_info in rooms.iter() {
            let room_name = room_info.name().trim();
            let mrea_id = room_info.room_id.to_u32();

            patcher.add_resource_patch(
                (&[pak_name.as_bytes()], mrea_id, FourCC::from_bytes(b"MREA")),
                move |res| patch_mlvl(res, pak_name.to_string(), room_name.to_string()),
            );
        }
    }

    patcher.run(gc_disc)?;

    Ok(())
}

fn patch_mlvl(resource: &mut Resource, pak_name: String, room_name: String) -> Result<(), String> {
    let mrea = resource.kind.as_mrea().unwrap();

    println!("Patching room: {} {}", pak_name, room_name);
    println!("Section count: {}", mrea.sections.len());

    debug_assert!(mrea.world_geometry_section_idx < mrea.area_octree_section_idx);
    debug_assert!(mrea.area_octree_section_idx < mrea.scly_section_idx);
    debug_assert!(mrea.scly_section_idx < mrea.collision_section_idx);
    debug_assert!(mrea.collision_section_idx < mrea.unknown_section_idx);
    debug_assert!(mrea.unknown_section_idx < mrea.lights_section_idx);
    debug_assert!(mrea.lights_section_idx < mrea.visibility_tree_section_idx);
    debug_assert!(mrea.visibility_tree_section_idx < mrea.path_section_idx);

    // let sections: Vec<_> = mrea.sections.iter().collect();
    // let geometry_section = sections_to_bytes(
    //     &sections[mrea.world_geometry_section_idx as usize..mrea.area_octree_section_idx as usize],
    // )?;
    // let arot = sections_to_bytes(
    //     &sections[mrea.area_octree_section_idx as usize..mrea.scly_section_idx as usize],
    // )?;
    // let scly = sections_to_bytes(
    //     &sections[mrea.scly_section_idx as usize..mrea.collision_section_idx as usize],
    // )?;
    // let collision = sections_to_bytes(
    //     &sections[mrea.collision_section_idx as usize..mrea.unknown_section_idx as usize],
    // )?;
    // let unknown = sections_to_bytes(
    //     &sections[mrea.unknown_section_idx as usize..mrea.lights_section_idx as usize],
    // )?;
    // let lights = sections_to_bytes(
    //     &sections[mrea.lights_section_idx as usize..mrea.visibility_tree_section_idx as usize],
    // )?;
    // let visibility = sections_to_bytes(
    //     &sections[mrea.visibility_tree_section_idx as usize..mrea.path_section_idx as usize],
    // )?;
    // let path = sections_to_bytes(&sections[mrea.path_section_idx as usize..])?;

    // println!(
    //     "Section sizes: geometry: {}, arot: {}, scly: {}, collision: {}, unknown: {}, lights: {}, visibility: {}, path: {}",
    //     geometry_section.len(),
    //     arot.len(),
    //     scly.len(),
    //     collision.len(),
    //     unknown.len(),
    //     lights.len(),
    //     visibility.len(),
    //     path.len(),
    // );

    // read the collision geometry
    // let mut area_reader = Reader::new(&collision[..]);
    let area_collision = mrea.collision_section();
    let existing_materials = mrea.materials_section();
    println!(
        "Area collision: {:?} verts {:?} edges {:?} tris",
        area_collision.collision.vert_count,
        area_collision.collision.edge_count,
        area_collision.collision.tri_count
    );
    println!(
        "Existing materials: {:?} texture ids {:?} materials",
        existing_materials.texture_ids.len(),
        existing_materials.materials.len()
    );

    // todo: don't hardcode these locations (and probably don't export them to disk by default)
    model_creation::create_model_files(&area_collision, &pak_name, &room_name)?;

    Ok(())
}

fn sections_to_bytes(sections: &[LCow<structs::MreaSection>]) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    for section in sections {
        section
            .write_to(&mut bytes)
            .map_err(|e| format!("Failed to write section: {}", e))?;
    }
    Ok(bytes)
}
