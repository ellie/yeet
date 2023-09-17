// Handle JPG optimization
// Basically just using mozjpeg
// It's slow compared to other tools, but does a good job of compressing.
// Seeing as we cache, that's fine.
// We are optimizing for an excellent JPEG, and good things take time.

use eyre::{eyre, Result};
use std::{fs::File, path::Path};

use crate::exif;

/// Take a path to a non-optimized JPG and output path.
/// Optimize the JPG and save it to the output.
/// Also opinionated as fuck, I'll probably add config options later.
pub fn optimize(input: &Path, output: &Path) -> Result<()> {
    // TODO(ellie): learn about how jpeg works so I can tune this and make it perfect. obv there
    // are magic numbers everyone should use all of the time
    //
    // I'm gonna whack some questions in this that I need to dig into later.

    // Ripped from the mozjpeg docs, to explain this panic-y thing. I don't like it.
    // =======
    // The interface is still being developed, so it has rough edges and may change.
    //
    // In particular, error handling is weird due to libjpeg’s peculiar design. Error handling can’t use Result,
    // but needs to depend on Rust’s resume_unwind (a panic, basically) to signal any errors in libjpeg.
    // It’s necessary to wrap all uses of this library in catch_unwind.

    let res = std::panic::catch_unwind(|| -> Result<()> {
        // Do I need markers? What _exactly_ is a marker?
        let d = mozjpeg::Decompress::with_markers(mozjpeg::ALL_MARKERS).from_path(input)?;
        let mut image = d.rgb()?;
        let pixels = image.read_scanlines()?;

        // it's been read! compress it

        // what colourspace is "best"? (this is intended for the web)
        let mut comp = mozjpeg::Compress::new(mozjpeg::ColorSpace::JCS_RGB);

        // I'm aware a lot of these are subjective and I need to play with them
        comp.set_quality(80.0); // Kinda just guessed with 80, maybe 70 is better?
        comp.set_progressive_mode(); // I like to think of myself as progressive, so this can go on
        comp.set_optimize_scans(true); // supposedly this makes progressives smaller. sounds like
                                       // an agenda to me

        // "Specifies whether multiple scans should be considered during trellis quantization"
        // What is trellis quantization? what does multiple scans achieve?
        comp.set_use_scans_in_trellis(true); // some rando github issue I saw suggested this. dig
                                             // into why.

        // comp.set_color_space(color_space); ok but like is there a color space I should be using?
        // comp.set_smoothing_factor(smoothing); what number for this is good?

        comp.set_size(image.width(), image.height());

        let output = File::create(output)?;
        let mut comp = comp.start_compress(output)?; // any io::Write will work

        // replace with your image data
        comp.write_scanlines(&pixels)?;

        comp.finish()?;
        image.finish()?;

        Ok(())
    });

    exif::copy_exif_tags(input, output)?;

    match res {
        Ok(r) => r,
        Err(_) => Err(eyre!("o no! we caught a panic!")),
    }
}
