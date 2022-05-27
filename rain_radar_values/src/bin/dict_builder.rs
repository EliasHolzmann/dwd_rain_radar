use rayon::prelude::*;
use rain_radar_values::*;

fn main() {
    let output_file = std::env::args().nth(1).expect(&format!("Usage: {} <output_file>", std::env::args().nth(0).unwrap_or("".to_string())));
    let mut file = std::fs::File::create(&output_file).expect("Creating file failed");
    eprintln!("Outputting to {output_file:?}");

    let mut results = vec![];

    crate::local_file_analysis::all_files().into_par_iter().map(|path| {
        CompressedRainRadarValues::from_rain_radar_values(&DWDRainRadarValues::from_file(path).expect("DWDRainRadarValues could not be created"))
    }).collect_into_vec(&mut results);

    dbg!(results.len());

    let dictionary = zstd::dict::from_samples(&results.iter().map(|values| values.data()).collect::<Vec<_>>(), 100000).expect("Could not create dictionary");

    std::io::copy(&mut std::io::Cursor::new(&dictionary), &mut file).expect("Copy failed");
}