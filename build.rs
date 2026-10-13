//! Embeds the can icon into the executable (the one Explorer shows) by writing a compiled
//! resource file by hand, so no resource compiler is needed when cross-compiling.

use std::path::PathBuf;
use std::{env, fs};

#[path = "src/icon.rs"]
mod icon;

const ICON_SIZES: [u32; 6] = [16, 24, 32, 48, 64, 256];
const RT_ICON: u16 = 3;
const RT_GROUP_ICON: u16 = 14;

fn main() {
    println!("cargo:rerun-if-changed=build.rs");
    println!("cargo:rerun-if-changed=src/icon.rs");

    // Only the MSVC-style linkers (link.exe, lld-link) accept .res files directly.
    if env::var("CARGO_CFG_TARGET_ENV").as_deref() != Ok("msvc") {
        return;
    }

    // A .res file starts with an empty resource entry.
    let mut res = Vec::new();
    push_resource(&mut res, 0, 0, &[]);

    let mut group = Vec::new();
    group.extend_from_slice(&0u16.to_le_bytes());
    group.extend_from_slice(&1u16.to_le_bytes()); // type: icon
    group.extend_from_slice(&(ICON_SIZES.len() as u16).to_le_bytes());

    for (index, size) in ICON_SIZES.into_iter().enumerate() {
        let id = index as u16 + 1;
        let image = icon_image(size);
        push_resource(&mut res, RT_ICON, id, &image);

        let dimension = if size >= 256 { 0 } else { size as u8 };
        group.extend_from_slice(&[dimension, dimension, 0, 0]);
        group.extend_from_slice(&1u16.to_le_bytes()); // planes
        group.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
        group.extend_from_slice(&(image.len() as u32).to_le_bytes());
        group.extend_from_slice(&id.to_le_bytes());
    }
    push_resource(&mut res, RT_GROUP_ICON, 1, &group);

    let path = PathBuf::from(env::var("OUT_DIR").unwrap()).join("icon.res");
    fs::write(&path, res).unwrap();
    println!("cargo:rustc-link-arg-bins={}", path.display());
}

/// Appends one resource with a numeric type and name, padded to a 4-byte boundary.
fn push_resource(res: &mut Vec<u8>, kind: u16, name: u16, data: &[u8]) {
    res.extend_from_slice(&(data.len() as u32).to_le_bytes());
    res.extend_from_slice(&32u32.to_le_bytes()); // header size
    res.extend_from_slice(&[0xFF, 0xFF]);
    res.extend_from_slice(&kind.to_le_bytes());
    res.extend_from_slice(&[0xFF, 0xFF]);
    res.extend_from_slice(&name.to_le_bytes());
    res.extend_from_slice(&0u32.to_le_bytes()); // data version
    res.extend_from_slice(&0u16.to_le_bytes()); // memory flags
    res.extend_from_slice(&0x0409u16.to_le_bytes()); // language: en-US
    res.extend_from_slice(&0u32.to_le_bytes()); // version
    res.extend_from_slice(&0u32.to_le_bytes()); // characteristics
    res.extend_from_slice(data);
    res.resize(res.len().next_multiple_of(4), 0);
}

/// One icon image as a 32-bit DIB: header, bottom-up BGRA pixels, then the (unused) AND mask.
fn icon_image(size: u32) -> Vec<u8> {
    let rgba = icon::render(size, true);
    let mut image = Vec::new();
    image.extend_from_slice(&40u32.to_le_bytes()); // BITMAPINFOHEADER size
    image.extend_from_slice(&(size as i32).to_le_bytes());
    image.extend_from_slice(&(size as i32 * 2).to_le_bytes()); // colour rows + mask rows
    image.extend_from_slice(&1u16.to_le_bytes()); // planes
    image.extend_from_slice(&32u16.to_le_bytes()); // bits per pixel
    image.extend_from_slice(&[0; 24]); // uncompressed, remaining fields unused

    for row in rgba.chunks_exact(size as usize * 4).rev() {
        for pixel in row.chunks_exact(4) {
            image.extend_from_slice(&[pixel[2], pixel[1], pixel[0], pixel[3]]);
        }
    }

    let mask_row_bytes = size.div_ceil(32) as usize * 4;
    image.resize(image.len() + mask_row_bytes * size as usize, 0);
    image
}
