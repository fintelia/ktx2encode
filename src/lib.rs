mod dfd_table;

pub use dfd_table::DFD_TABLE;
pub use ktx2::Format;

use crate::dfd_table::TYPE_SIZES;

/// Encode a KTX2 file from a list of image slices. The resulting image will be supercompressed
/// using Zstandard compression.
///
/// # Arguments
/// * `image_slices` - A list of image slices, each slice is a list of bytes.
/// * `width` - The width of the image.
/// * `height` - The height of the image.
/// * `depth` - The depth of the image (0 if the image is not 3D).
/// * `layers` - The number of layers (0 if the image is not an array).
/// * `cubemap` - Whether the image is a cubemap.
/// * `format` - The format of the image. Used to fill metadata, but not for any sort of conversion.
/// * `compression_level` - What zstd compression level to use.
pub fn encode_ktx2(
    image_slices: &[Vec<u8>],
    width: u32,
    height: u32,
    depth: u32,
    layers: u32,
    cubemap: bool,
    format: ktx2::Format,
    compression_level: i32,
) -> Vec<u8> {
    if format.0.get() as usize >= DFD_TABLE.len()
        || DFD_TABLE[format.0.get() as usize].is_empty()
        || TYPE_SIZES[format.0.get() as usize] == 0
    {
        panic!("Unsupported format {:?}", format);
    }
    let dfd = DFD_TABLE[format.0.get() as usize];
    let levels = image_slices.len() as u32;

    // Write header
    let mut contents = Vec::new();
    contents.extend_from_slice(&[
        0xAB, 0x4B, 0x54, 0x58, 0x20, 0x32, 0x30, 0xBB, 0x0D, 0x0A, 0x1A, 0x0A,
    ]);
    contents.extend_from_slice(&format.0.get().to_le_bytes());
    contents.extend_from_slice(&(TYPE_SIZES[format.0.get() as usize] as u32).to_le_bytes());
    contents.extend_from_slice(&width.to_le_bytes());
    contents.extend_from_slice(&height.to_le_bytes());
    contents.extend_from_slice(&depth.to_le_bytes());
    contents.extend_from_slice(&layers.to_le_bytes());
    if cubemap {
        contents.extend_from_slice(&6u32.to_le_bytes());
    } else {
        contents.extend_from_slice(&1u32.to_le_bytes());
    }
    contents.extend_from_slice(&levels.to_le_bytes()); // levels
    contents.extend_from_slice(&2u32.to_le_bytes()); // supercompressionScheme = zstd

    // Write index
    contents.extend_from_slice(&(80 + 24 * levels).to_le_bytes());
    contents.extend_from_slice(&(dfd.len() as u32).to_le_bytes());
    contents.extend_from_slice(&0u32.to_le_bytes()); // kvdByteOffset
    contents.extend_from_slice(&0u32.to_le_bytes()); // kvdByteLength
    contents.extend_from_slice(&0u64.to_le_bytes()); // sgdByteOffset
    contents.extend_from_slice(&0u64.to_le_bytes()); // sgdByteLength
    assert_eq!(contents.len(), 80);

    // Write level index
    let mut compressed_image_slices = Vec::new();
    let mut offset = (80 + 24 * levels + dfd.len() as u32) as u64;
    for image_slice in image_slices {
        let compressed =
            zstd::encode_all(std::io::Cursor::new(image_slice), compression_level).unwrap();
        contents.extend_from_slice(&offset.to_le_bytes());
        contents.extend_from_slice(&(compressed.len() as u64).to_le_bytes());
        contents.extend_from_slice(&(image_slice.len() as u64).to_le_bytes());
        offset += compressed.len() as u64;
        compressed_image_slices.push(compressed);
    }

    // Write Data Format Descriptor
    assert_eq!(contents.len(), 80 + 24 * levels as usize);
    contents.extend_from_slice(dfd);
    if contents.len() % 4 != 0 {
        contents.resize((contents.len() & !3) + 4, 0);
    }

    // Write image data
    assert_eq!(contents.len(), 80 + 24 * levels as usize + dfd.len());
    for image_slice in compressed_image_slices {
        contents.extend_from_slice(&image_slice);
    }

    contents
}
