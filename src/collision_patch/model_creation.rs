use image::{imageops, ImageBuffer, Rgba, RgbaImage};
use reader_writer::{LCow, LazyArray, Writable};
use std::collections::{HashMap, HashSet};
use std::fs::File;
use std::hash::Hash;
use std::io::{BufWriter, Write};
use std::ops::Deref;
use structs::res_id::{ResIdKind, MREA, TXTR};
use structs::{res_id, AreaCollision, CollisionMaterialFlags, ResId, Resource, ResourceKind, TriData, Txtr, TxtrFormat};
use crate::custom_assets::build_resource;

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
struct GeometryMaterialDefinition {
    pub line1: String,
    pub line2: String,
    pub line3: String,
    /** argb */
    pub color: u32,
}

impl GeometryMaterialDefinition {
    pub fn texture_id(&self) -> u32 {
        let mut hasher = md5::Context::new();
        hasher.consume(self.line1.as_bytes());
        hasher.consume(self.line2.as_bytes());
        hasher.consume(self.line3.as_bytes());
        hasher.consume(&self.color.to_le_bytes());
        let hash = hasher.compute();
        u32::from_le_bytes(hash[0..4].try_into().unwrap())
    }

    pub fn texture_id_string(&self) -> String {
        format!("{:08x}", self.texture_id())
    }
}

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
    let material_collision_flags: Vec<_> = collision
        .collision
        .collision_materials
        .iter()
        .map(|m| m.into_owned())
        .collect();
    // let vertex_material_indices: Vec<_> = collision
    //     .collision
    //     .vertex_material_indices
    //     .iter()
    //     .map(|m| m.into_owned())
    //     .collect();
    // let edge_material_indices: Vec<_> = collision
    //     .collision
    //     .edge_material_indices
    //     .iter()
    //     .map(|m| m.into_owned())
    //     .collect();
    let tri_material_indices: Vec<_> = collision
        .collision
        .tri_material_indices
        .iter()
        .map(|m| m.into_owned())
        .collect();

    // ok now we can start generating the proper tris from the edges
    let mut tris = Vec::with_capacity(raw_tris.len());
    for i in 0..raw_tris.len() {
        let tri = raw_tris[i];
        let tri_material = material_collision_flags[tri_material_indices[i] as usize];

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

    // get material defs
    let geometry_material_map = create_material_defs(&material_collision_flags);
    println!(
        "There are {} collision materials",
        material_collision_flags.len()
    );

    // start with blender
    create_blender_textures(&geometry_material_map, &pak_name, &room_name)?;

    create_blender_obj(
        pak_name,
        room_name,
        &verts,
        &material_collision_flags,
        &tri_material_indices,
        &geometry_material_map,
        &mut tris,
    )?;

    // then the real game
    let txtrs = create_game_textures(&geometry_material_map)?;

    Ok(())
}

fn create_material_defs(
    collision_material_flags: &Vec<u32>,
) -> HashMap<u32, GeometryMaterialDefinition> {
    let mut map = HashMap::new();
    for &collision_material in collision_material_flags {
        let line1 = get_text_line1(collision_material);
        let line2 = get_text_line2(collision_material);
        let line3 = get_text_line3(collision_material);

        // TODO: use the same algo as the game for floors
        let mut color = if collision_material & CollisionMaterialFlags::Floor as u32 != 0 {
            0xFF_FF_80_80
        } else if collision_material & CollisionMaterialFlags::Wall as u32 != 0 {
            0xFF_FF_FF_FF
        } else if collision_material & CollisionMaterialFlags::Ceiling as u32 != 0 {
            0xFF_80_FF_80
        } else {
            0xFF_FF_80_80
        };

        if collision_material & CollisionMaterialFlags::ShootThru as u32 != 0 {
            //     color.3 = 0.5;
            color = color & 0x00_FF_FF_FF | 0x80_00_00_00;
        }

        map.insert(
            collision_material,
            GeometryMaterialDefinition {
                line1,
                line2,
                line3,
                color,
            },
        );
    }
    map
}

fn create_blender_textures(
    geometry_material_defs: &HashMap<u32, GeometryMaterialDefinition>,
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

    let mtl_file = format!("collision/{}/{}_materials.mtl", pak_name, room_name);
    let mut mtl_writer = BufWriter::new(
        File::create(mtl_file).map_err(|e| format!("Failed to create mtl file: {}", e))?,
    );

    // We want a 64x64 texture for each material; the room with the most is MQA at 77, and that should be fine on RAM
    // 4 bit luma format * 64 * 64 = 2048 bytes * 77 = 157,696 bytes
    // that's still probably ok on gcn *crosses fingers*
    // or else we'll have to de-dupe

    let unique_materials: HashSet<_> = geometry_material_defs.values().collect();
    println!("There are {} unique textures", &unique_materials.len());
    for def in unique_materials.into_iter() {
        let mut texture = RgbaImage::new(tri_base.width(), tri_base.height());
        // draw a triangle
        imageops::overlay(&mut texture, &tri_base, 0, 0);

        // figure out what we want to write
        let mut color = (
            ((def.color >> 24) & 0xFF) as f32 / 255.0,
            ((def.color >> 16) & 0xFF) as f32 / 255.0,
            ((def.color >> 8) & 0xFF) as f32 / 255.0,
            ((def.color >> 0) & 0xFF) as f32 / 255.0,
        );

        // change the color by multiplying the color by the color
        for pixel in texture.pixels_mut() {
            let [r, g, b, a] = pixel.0;
            pixel.0 = [
                (r as f32 * color.1) as u8,
                (g as f32 * color.2) as u8,
                (b as f32 * color.3) as u8,
                (a as f32 * color.0) as u8,
            ];
        }

        draw_text_line(&mut texture, &font, &def.line1, 6, 3);
        draw_text_line(&mut texture, &font, &def.line2, 11, 13);
        draw_text_line(&mut texture, &font, &def.line3, 16, 23);

        let material_name = format!("material_{}", def.texture_id());
        let out_file = format!("collision/{}/{}.png", pak_name, material_name);
        texture
            .save(out_file)
            .map_err(|e| format!("Failed to save texture: {}", e))?;
        // create a mtl for it
        writeln!(mtl_writer, "newmtl {}", material_name)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
        writeln!(mtl_writer, "map_Kd {}.png", material_name)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
        writeln!(mtl_writer, "map_d {}.png", material_name)
            .map_err(|e| format!("Failed to write mtl: {}", e))?;
    }

    mtl_writer
        .flush()
        .map_err(|e| format!("Failed to flush mtl file: {}", e))?;

    Ok(())
}

fn create_blender_obj(
    pak_name: &String,
    room_name: &String,
    verts: &Vec<(f32, f32, f32)>,
    material_collision_flags: &Vec<u32>,
    tri_material_indices: &Vec<u8>,
    geometry_material_map: &HashMap<u32, GeometryMaterialDefinition>,
    tris: &mut Vec<TriData>,
) -> Result<(), String> {
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
    for flag_index in 0..material_collision_flags.len() as u8 {
        let material_def = geometry_material_map
            .get(&material_collision_flags[flag_index as usize])
            .unwrap();
        let material_name = format!("material_{}", material_def.texture_id());
        writeln!(writer, "g {}", &material_name)
            .map_err(|e| format!("Failed to write obj: {}", e))?;
        writeln!(writer, "usemtl {}", material_name)
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

fn create_game_textures<'r> (
    geometry_material_defs: &HashMap<u32, GeometryMaterialDefinition>,
) -> Result<Vec<Resource>, String> {
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

    // We want a 64x64 texture for each material; the room with the most is MQA at 77, and that should be fine on RAM
    // 4 bit luma format * 64 * 64 = 2048 bytes * 77 = 157,696 bytes
    // that's still probably ok on gcn *crosses fingers*
    // or else we'll have to de-dupe

    let unique_materials: HashSet<_> = geometry_material_defs.values().collect();
    println!("There are {} unique txtr", &unique_materials.len());
    let mut txtrs = Vec::with_capacity(unique_materials.len());
    for def in unique_materials.into_iter() {
        let mut texture = RgbaImage::new(tri_base.width(), tri_base.height());
        // draw a triangle
        imageops::overlay(&mut texture, &tri_base, 0, 0);
        draw_text_line(&mut texture, &font, &def.line1, 6, 3);
        draw_text_line(&mut texture, &font, &def.line2, 11, 13);
        draw_text_line(&mut texture, &font, &def.line3, 16, 23);

        // now we need to write it in i4 format
        let i4 = convert_to_i4(&texture);
        let mipmaps: Vec<LazyArray<u8>> = vec![i4.into()];
        let txtr = Txtr {
            format: TxtrFormat::I4,
            width: texture.width() as u16,
            height: texture.height() as u16,
            pixel_data: mipmaps.into(),
        };
        // write txtr to bytes
        let mut txtr_bytes = Vec::new();
        txtr.write_to(&mut txtr_bytes)
            .map_err(|e| format!("Failed to write txtr: {}", e))?;
        let resource = build_resource(ResId::<res_id::SCAN>::new(def.texture_id()), ResourceKind::External(txtr_bytes, b"TXTR".into()));
        txtrs.push(resource);
    }

    Ok(txtrs)
}

fn convert_to_i4(img: &RgbaImage) -> Vec<u8> {
    //converted from my C++ code that I used for practice mod's text texture
    let width = img.width();
    let height = img.height();
    let block_width = 8;
    let block_height = 8;
    let x_blocks = width / block_width;
    let y_blocks = height / block_height;

    let mut out_data = vec![0; (width * height / 2) as usize];

    for x_block in 0..x_blocks {
        for y_block in 0..y_blocks {
            let block_start_x = x_block * block_width;
            let block_start_y = y_block * block_height;
            let out_start = ((y_block * x_blocks) + x_block) * (block_width * block_height);
            // Ok now loop over the pixels
            for block_relative_x in 0..block_width {
                for block_relative_y in 0..block_height {
                    let pixel_rgba = img.get_pixel(
                        block_start_x + block_relative_x,
                        block_start_y + block_relative_y,
                    );
                    // average RGB
                    let (r, g, b) = (pixel_rgba[0], pixel_rgba[1], pixel_rgba[2]);
                    let pixel = ((r + g + b + 0x80) >> 6) & 0xF; // divide by 4 & take the top 4 bits

                    let block_relative_offset = block_relative_y * block_width + block_relative_x;
                    let pixel_pos = out_start + block_relative_offset;
                    let byte_pos = pixel_pos / 2;
                    if pixel_pos % 2 == 0 {
                        out_data[byte_pos as usize] = out_data[byte_pos as usize] | pixel << 4;
                    } else {
                        out_data[byte_pos as usize] = out_data[byte_pos as usize] | pixel;
                    }
                }
            }
            // Done with the block
        }
    }
    out_data
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
    // don't to F to halve the material count
    // if collision_material_flags & CollisionMaterialFlags::RedundantEdgeOrFlippedTri as u32 != 0 {
    //     ret += "F";
    // }
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
