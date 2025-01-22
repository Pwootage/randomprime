use image::{imageops, ImageBuffer, Rgba, RgbaImage};
use reader_writer::LCow;
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufWriter, Write};
use std::ops::Deref;
use structs::{AreaCollision, CollisionMaterialFlags, TriData};

pub fn create_model_files(
    collision: &LCow<AreaCollision>,
    pak_name: &String,
    room_name: &String,
) -> Result<(), String> {
    // we can pull the verts directly
    let verts: Vec<_> = collision
        .collision
        .verts
        .iter()
        .map(|v| (v.deref()[0], v.deref()[1], v.deref()[2]))
        .collect();

    // we'll have to convert edges and tris manually though
    let raw_edges: Vec<_> = collision
        .collision
        .edges
        .iter()
        .map(|e| e.into_owned())
        .collect();
    let raw_tris: Vec<_> = collision
        .collision
        .tris
        .iter()
        .map(|t| t.into_owned())
        .collect();

    // now we need to pull in the material flags
    let material_flags: Vec<_> = collision
        .collision
        .collision_materials
        .iter()
        .map(|m| m.into_owned())
        .collect();
    let vertex_material_indices: Vec<_> = collision
        .collision
        .vertex_material_indices
        .iter()
        .map(|m| m.into_owned())
        .collect();
    let edge_material_indices: Vec<_> = collision
        .collision
        .edge_material_indices
        .iter()
        .map(|m| m.into_owned())
        .collect();
    let tri_material_indices: Vec<_> = collision
        .collision
        .tri_material_indices
        .iter()
        .map(|m| m.into_owned())
        .collect();

    // create a texture for the materials
    create_textures(&material_flags, &pak_name, &room_name)?;

    // ok now we can start generating the proper tris from the edges
    let mut tris = Vec::with_capacity(raw_tris.len());

    for i in 0..raw_tris.len() {
        let tri = raw_tris[i];
        let tri_material = material_flags[tri_material_indices[i] as usize];

        let e1 = raw_edges[tri.a as usize];
        let e2 = raw_edges[tri.b as usize];
        let e3 = raw_edges[tri.c as usize];

        // this code is from my old exporter, but I don't remember *why* it's like this
        let a = e1.a;
        let (b, other_edge) = if e1.a == e2.a {
            (e2.b, e3)
        } else if e1.a == e2.b {
            (e2.a, e3)
        } else if e1.a == e3.a {
            (e3.b, e2)
        } else if e1.a == e3.b {
            (e3.a, e2)
        } else {
            panic!("Edge 1 doesn't match any other edge")
        };
        let c = if b == other_edge.a {
            other_edge.b
        } else if b == other_edge.b {
            other_edge.a
        } else {
            panic!("Edge 2 doesn't match the other line")
        };

        // yeah it winds backwards, according to my old code
        let (a, b, c) =
            if tri_material & CollisionMaterialFlags::RedundantEdgeOrFlippedTri as u32 != 0 {
                (a, b, c)
            } else {
                (c, b, a)
            };
        tris.push(TriData { a, b, c });
    }

    let out_file = format!("collision/{}/{}_collision.obj", pak_name, room_name);
    // create the directories
    std::fs::create_dir_all(format!("collision/{}", pak_name)).unwrap();
    let mut writer = BufWriter::new(File::create(out_file).unwrap());

    writeln!(writer, "o {}", room_name).map_err(|e| format!("Failed to write obj: {}", e))?;
    writeln!(writer, "mtllib {}_materials.mtl", room_name)
      .map_err(|e| format!("Failed to write obj: {}", e))?;
    // all tris will use the same 3 uv: top left, top right, bottom center
    writeln!(writer, "vt 1 1").map_err(|e| format!("Failed to write obj: {}", e))?;
    writeln!(writer, "vt 0 1").map_err(|e| format!("Failed to write obj: {}", e))?;
    writeln!(writer, "vt 0.5 0").map_err(|e| format!("Failed to write obj: {}", e))?;

    // write verts
    for (x, y, z) in verts {
        writeln!(writer, "v {} {} {}", x, y, z)
            .map_err(|e| format!("Failed to write obj: {}", e))?;
    }

    // write an obj file
    for flag_index in 0..material_flags.len() as u8 {
        writeln!(writer, "g {}_material_{}", room_name, flag_index)
            .map_err(|e| format!("Failed to write obj: {}", e))?;
        writeln!(writer, "usemtl material_{}", flag_index)
            .map_err(|e| format!("Failed to write obj: {}", e))?;

        for i in 0..tris.len() {
            if tri_material_indices[i] != flag_index {
                continue;
            }
            let tri = tris[i];
            writeln!(writer, "f {}/1 {}/2 {}/3", tri.a + 1, tri.b + 1, tri.c + 1)
            .map_err(|e| format!("Failed to write obj: {}", e))?;
        }
    }

    writer
        .flush()
        .map_err(|e| format!("Failed to flush obj file: {}", e))?;

    Ok(())
}

fn create_textures(
    materials: &Vec<u32>,
    pak_name: &String,
    room_name: &String,
) -> Result<(), String> {
    // this is what I use in practice mod, so I just copied it along. 8x8 font atlas
    const FONT: &[u8] = include_bytes!("../../extra_assets/ProggyTinyTT.png");
    const TRI_BASE: &[u8] = include_bytes!("../../extra_assets/collision_tri_base_64.png");
    // decode the pngs
    let font = image::load_from_memory_with_format(FONT, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8();
    let tri_base = image::load_from_memory_with_format(TRI_BASE, image::ImageFormat::Png)
        .unwrap()
        .to_rgba8();

    // create the directories
    std::fs::create_dir_all(format!("collision/{}", pak_name)).unwrap();

    println!("There are {} collision materials", materials.len());

    let mtl_file = format!("collision/{}/{}_materials.mtl", pak_name, room_name);
    let mut mtl_writer = BufWriter::new(
        File::create(mtl_file).map_err(|e| format!("Failed to create mtl file: {}", e))?,
    );

    // We want a 64x64 texture for each material; the room with the most is MQA at 77, and that should be fine on RAM
    // 4 bit luma format * 64 * 64 = 2048 bytes * 77 = 157,696 bytes
    // that's still probably ok on gcn *crosses fingers*
    // or else we'll have to de-dupe
    for i in 0..materials.len() {
        let mut texture = RgbaImage::new(tri_base.width(), tri_base.height());
        let material = materials[i];

        // draw a triangle
        imageops::overlay(&mut texture, &tri_base, 0, 0);

        // figure out what we want to write
        // TODO: use the same algo as the game
        let mut color = if material & CollisionMaterialFlags::Floor as u32 != 0 {
            (1.0, 0.5, 0.5, 1.0)
        } else if material & CollisionMaterialFlags::Wall as u32 != 0 {
            (1.0, 1.0, 1.0, 1.0)
        } else if material & CollisionMaterialFlags::Ceiling as u32 != 0 {
            (0.5, 1.0, 0.5, 1.0)
        } else {
            (1.0, 0.5, 0.5, 1.0)
        };

        if material & CollisionMaterialFlags::ShootThru as u32 != 0 {
            color.3 = 0.5;
        }

        // change the color by multiplying the color by the color
        for pixel in texture.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            pixel.0 = [
                (r as f32 * color.0) as u8,
                (g as f32 * color.1) as u8,
                (b as f32 * color.2) as u8,
                (a as f32 * color.3) as u8,
            ];
        }

        let line1 = get_text_line1(material);
        draw_text_line(&mut texture, &font, &line1, 6, 3);

        let line2 = get_text_line2(material);
        draw_text_line(&mut texture, &font, &line2, 11, 13);

        let line3 = get_text_line3(material);
        draw_text_line(&mut texture, &font, &line3, 16, 23);

        let out_file = format!("collision/{}/{}_material_{}.png", pak_name, room_name, i);
        texture
            .save(out_file)
            .map_err(|e| format!("Failed to save texture: {}", e))?;
        // create a mtl for it
        writeln!(mtl_writer, "newmtl material_{}", i)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
        writeln!(mtl_writer, "map_Kd {}_material_{}.png", room_name, i)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
        writeln!(mtl_writer, "map_d {}_material_{}.png", room_name, i)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
    }

    mtl_writer
        .flush()
        .map_err(|e| format!("Failed to flush mtl file: {}", e))?;

    Ok(())
}

fn get_text_line1(collision_material_flags: u32) -> String {
    // loop over the flags and build a string, backwards
    for &flag in CollisionMaterialFlags::MATERIAL_FLAGS.iter().rev() {
        if collision_material_flags & (flag as u32) != 0 {
            return flag.name().to_string();
        }
    }
    "???".to_string()
}

fn get_text_line2(collision_material_flags: u32) -> String {
    if collision_material_flags & CollisionMaterialFlags::Halfpipe as u32 != 0 {
        "Pipe".to_string()
    } else if collision_material_flags & CollisionMaterialFlags::ShootThru as u32 != 0 {
        "Shoot".to_string()
    } else if collision_material_flags & CollisionMaterialFlags::CameraThru as u32 != 0 {
        "Cam".to_string()
    } else if collision_material_flags & CollisionMaterialFlags::ScanThru as u32 != 0 {
        "Scan".to_string()
    } else {
        "".to_string()
    }
}

fn get_text_line3(collision_material_flags: u32) -> String {
    let mut ret = "".to_string();
    if collision_material_flags & CollisionMaterialFlags::RedundantEdgeOrFlippedTri as u32 != 0 {
        ret += "F";
    }
    if collision_material_flags & CollisionMaterialFlags::NoEdgeCollision as u32 != 0 {
        ret += "E";
    }

    ret
}

fn draw_text_line(texture: &mut RgbaImage, font: &RgbaImage, line1: &str, x: u32, y: u32) {
    let cols = font.width() / 8;
    for (i, c) in line1.chars().enumerate() {
        let char_x = (c as u32 % cols) * 8;
        let char_y = (c as u32 / cols) * 8;
        let char = imageops::crop_imm(font, char_x, char_y, 8, 8);
        imageops::overlay(texture, &char, x + (i as u32 * 8), y);
    }
}
