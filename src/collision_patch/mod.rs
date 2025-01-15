use crate::elevators::World;
use crate::patch_config::RoomConfig;
use crate::patcher::PrimePatcher;
use crate::pickup_meta;
use memmap::Mmap;
use reader_writer::{FourCC, Reader};
use resource_info_table::resource_info;
use std::fs::File;
use std::io::{Read, Write};
use std::time::Instant;
use structs::Resource;

pub fn patch_iso_collision<T>(input_iso: Mmap, output_iso: File, mut pn: T) -> Result<(), String>
where
    T: structs::ProgressNotifier,
{
    let start_time = Instant::now();
    let mut reader = Reader::new(&input_iso[..]);
    let mut gc_disc: structs::GcDisc = reader.read(());

    build_and_run_collision_patches(&mut gc_disc)?;

    println!("Created patches in {:?}", start_time.elapsed());

    let mut file = output_iso;
    file.set_len(structs::GC_DISC_LENGTH as u64)
        .map_err(|e| format!("Failed to resize output file: {}", e))?;
    gc_disc
        .write(&mut file, &mut pn)
        .map_err(|e| format!("Error writing output file: {}", e))?;
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
                move |res| patch_mlvl(res, room_name.to_string()),
            );
        }
    }

    // for (room, room_config) in other_patches {
    // if let Some(connections) = room_config.add_connections.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         crate::patches::patch_add_connections(ps, area, connections)
    //     });
    // }
    //
    // if let Some(connections) = room_config.remove_connections.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         crate::patches::patch_remove_connections(ps, area, connections)
    //     });
    // }
    //
    // if let Some(layers) = room_config.layers.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         crate::patches::patch_set_layers(ps, area, layers.clone())
    //     });
    // }
    //
    // if let Some(layer_objs) = room_config.layer_objs.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         crate::patches::patch_move_objects(ps, area, layer_objs.clone())
    //     });
    // }
    //
    // if let Some(edit_objs) = room_config.edit_objs.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         patch_edit_objects(ps, area, edit_objs.clone())
    //     });
    // }
    //
    // if let Some(ids) = room_config.delete_ids.as_ref() {
    //     patcher.add_scly_patch(*room, move |ps, area| {
    //         crate::patches::patch_remove_ids(ps, area, ids.clone())
    //     });
    // }
    // }

    patcher.run(gc_disc)?;

    Ok(())
}

fn patch_mlvl(resource: &mut Resource, room_name: String) -> Result<(), String> {
    let mrea = resource.kind.as_mrea().unwrap();

    println!("Patching room: {}", room_name);
    println!("Section count: {}", mrea.sections.len());

    Ok(())
}
