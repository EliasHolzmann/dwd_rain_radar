use std::fmt::Debug;

use anyhow::{Context, Result};
use rain_radar_values::RainRadarValues;

use rayon::prelude::*;

#[derive(Debug)]
struct CheckerResults {
    min_available_data_points: u32,
    max_available_data_points: u32,

    min_rain_value_except_0: u16,
    max_rain_value: u16,
    non_zero_values: u32,
    values_greater_255: u32,
    blocks_of_100: u32,
    blocks_of_100_with_no_values: u32,
    blocks_of_100_with_only_zero_values: u32,
    blocks_of_100_with_values_greater_254: u32,
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
                        min_rain_value_except_0: u16::MAX,
                        max_rain_value: u16::MIN,
                        non_zero_values: 0,
                        values_greater_255: 0,
                        blocks_of_100: 0,
                        blocks_of_100_with_no_values: 0,
                        blocks_of_100_with_only_zero_values: 0,
                        blocks_of_100_with_values_greater_254: 0,
                    };
                    for (index, value) in values.for_area(time, 0..1100, 0..1200).enumerate() {
                        if let Some(value) = value {
                            // increment both min and max: Two fields are only for reduce outside of this
                            result.min_available_data_points += 1;
                            result.max_available_data_points += 1;
                            if value != 0 {
                                result.min_rain_value_except_0 =
                                    u16::min(value, result.min_rain_value_except_0);
                                result.non_zero_values += 1;
                            }
                            if value > 255 {
                                result.values_greater_255 += 1;
                            }
                            result.max_rain_value = u16::max(value, result.max_rain_value);
                        }
                    }
                    for x in 0..11 {
                        for y in 0..12 {
                            result.blocks_of_100 += 1;
                            let values: Vec<Option<u16>> = values
                                .for_area(
                                    time,
                                    (x * 100)..((x + 1) * 100),
                                    (y * 100)..((y + 1) * 100),
                                )
                                .collect();
                            if values.iter().all(|value| value.is_none()) {
                                result.blocks_of_100_with_no_values += 1;
                            } else if values.iter().all(|value| value.unwrap_or(1) == 0) {
                                result.blocks_of_100_with_only_zero_values += 1;
                            } else if values.iter().any(|value| value.unwrap_or(0) > 254) {
                                result.blocks_of_100_with_values_greater_254 += 1;
                            }
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

            min_rain_value_except_0: u16::min(
                res1.min_rain_value_except_0,
                res2.min_rain_value_except_0,
            ),
            max_rain_value: u16::max(res1.max_rain_value, res2.max_rain_value),
            non_zero_values: res1.non_zero_values + res2.non_zero_values,
            values_greater_255: res1.values_greater_255 + res2.values_greater_255,
            blocks_of_100: res1.blocks_of_100 + res2.blocks_of_100,
            blocks_of_100_with_no_values: res1.blocks_of_100_with_no_values
                + res2.blocks_of_100_with_no_values,
            blocks_of_100_with_only_zero_values: res1.blocks_of_100_with_only_zero_values
                + res2.blocks_of_100_with_only_zero_values,
            blocks_of_100_with_values_greater_254: res1.blocks_of_100_with_values_greater_254
                + res2.blocks_of_100_with_values_greater_254,
        });
    println!("{result:#?}");

    Ok(())
}
