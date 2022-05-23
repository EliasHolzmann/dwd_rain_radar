use std::fmt::Debug;

use anyhow::{Context, Result};
use rain_radar_values::RainRadarValues;

use rayon::prelude::*;

#[derive(Debug)]
struct CheckerResults {
    min_available_data_points: u32,
    max_available_data_points: u32,

    min_rain_value: u16,
    max_rain_value: u16,
}

fn main() -> Result<()> {
    let target_directory: std::path::PathBuf = std::env::var("DWD_DOWNLOADER_TARGET_DIRECTORY")
        .context("Failed reading environment variable DWD_DOWNLOADER_TARGET_DIRECTORY")?
        .into();

    let files = std::fs::read_dir(target_directory)
        .context("Could not read directory")?
        .flat_map(|directory| -> std::fs::ReadDir {
            std::fs::read_dir(directory.expect("Could not read directory").path())
                .expect("Could not read directory")
        })
        .map(|file| file.unwrap())
        .collect::<Vec<_>>();

    let result = files
        .par_iter()
        .map(|file| {
            let file_path = file.path();
            let values = rain_radar_values::DWDRainRadarValues::from_file(file_path)
                .unwrap_or_else(|err| panic!("Failed loading {file:?}: {err}"));

            values
                .available_times()
                .map(|time| -> CheckerResults {
                    let mut result = CheckerResults {
                        min_available_data_points: 0,
                        max_available_data_points: 0,
                        min_rain_value: u16::MAX,
                        max_rain_value: u16::MIN,
                    };
                    for value in values.for_area(time, 0..1100, 0..1200) {
                        if let Some(value) = value {
                            // increment both min and max: Two fields are only for reduce outside of this
                            result.min_available_data_points += 1;
                            result.max_available_data_points += 1;
                            result.min_rain_value = u16::min(value, result.min_rain_value);
                            result.max_rain_value = u16::max(value, result.max_rain_value);
                        }
                    }
                    result
                })
                .collect::<Vec<_>>()
                .into_iter()
        })
        .collect::<Vec<_>>()
        .iter_mut()
        .flatten()
        .reduce(|res1, res2| CheckerResults {
            min_available_data_points: u32::min(
                res1.min_available_data_points,
                res2.min_available_data_points,
            ),
            max_available_data_points: u32::max(
                res1.max_available_data_points,
                res2.max_available_data_points,
            ),

            min_rain_value: u16::min(res1.min_rain_value, res2.min_rain_value),
            max_rain_value: u16::max(res1.max_rain_value, res2.max_rain_value),
        });
    println!("{result:#?}");

    Ok(())
}
