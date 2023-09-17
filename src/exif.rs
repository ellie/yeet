use std::path::Path;

use eyre::{eyre, Result};

/// Copy safe/required exif tags from the input path to the output path.
/// Currently this is just the orientation
pub fn copy_exif_tags(input: &Path, output: &Path) -> Result<()> {
    let input_meta = rexiv2::Metadata::new_from_path(input)?;

    // Create output tags and make very sure they have no GPS data.
    let output_meta = rexiv2::Metadata::new_from_path(output)?;
    output_meta.delete_gps_info();

    let orientation = input_meta.get_orientation();
    output_meta.set_orientation(orientation);

    match output_meta.save_to_file(output) {
        Ok(_) => Ok(()),
        Err(_) => Err(eyre!("Failed to save error")),
    }
}
