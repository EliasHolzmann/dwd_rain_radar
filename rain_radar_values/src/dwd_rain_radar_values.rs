use super::RainRadarValues;
use anyhow::{anyhow, bail, ensure, Context, Result};
use chrono::prelude::*;
use std::io::Read;

trait ReadExt: Read {
    fn integer(&mut self, length: usize) -> Result<u32> {
        let mut vec = vec![0u8; length];
        self.read_exact(&mut *vec)
            .with_context(|| anyhow!("Failed reading integer of length {length}"))?;
        std::str::from_utf8(&*vec)
            .with_context(|| {
                anyhow!("Failed converting file contents of length {length} to UTF-8")
            })?
            .trim()
            .parse()
            .with_context(|| anyhow!("Failes parsing file contents of length {length} to integer"))
    }
    fn array<const SIZE: usize>(&mut self) -> Result<[u8; SIZE]> {
        let mut result = [0u8; SIZE];
        self.read_exact(&mut result[..])
            .context("Failed reading space in front of format version")?;
        Ok(result)
    }
    fn ensure_next_is_space(&mut self) -> Result<()> {
        let hopefully_space = self
            .array::<1>()
            .context("Failed reading space in front of format version")?;
        ensure!(
            hopefully_space == *b" ",
            "Next character is not \" \", but {:?}",
            hopefully_space[0]
        );
        Ok(())
    }
    fn vec(&mut self, size: usize) -> Result<Vec<u8>> {
        let mut result = vec![0u8; size];
        self.read_exact(&mut result[..])
            .context("Failed reading space in front of format version")?;
        Ok(result)
    }
}

impl<T: Read> ReadExt for T {}

pub struct DWDRainRadarValues {
    base_time: chrono::naive::NaiveDateTime,
    predictions: Vec<Vec<u8>>,
}

impl DWDRainRadarValues {
    pub fn from_file<P: AsRef<std::path::Path>>(path: P) -> Result<Self> {
        let file = std::io::BufReader::new(
            std::fs::File::open(path).context("Could not open rain radar values file")?,
        );
        let decoder = bzip2::bufread::BzDecoder::new(file);
        let mut archive = tar::Archive::new(decoder);

        let mut base_time: Option<NaiveDateTime> = None;
        let predictions = archive
            .entries()
            .context("Could not iterate over archive entries")?
            .enumerate()
            .map(|(index, entry)| -> Result<Vec<u8>> {
                let mut entry = entry.context("Could not get next archive entry")?;
                // parse header
                let mut product_name = [0u8; 2];
                entry
                    .read_exact(&mut product_name)
                    .context("Failed reading product_name")?;
                ensure!(
                    product_name == *b"RV",
                    "Expected product name to be \"RV\" but found {product_name:?}"
                );

                let day = entry.integer(2).context("Failed extracting day")?;
                let hour = entry.integer(2).context("Failed extracting hour")?;
                let minute = entry.integer(2).context("Failed extracting minute")?;
                let _wmo_number = entry.integer(5).context("Failed extracting WMO number")?; // probably irrelevant here
                let month = entry.integer(2).context("Failed extracting month")?;
                let year = entry.integer(2).context("Failed extracting year")?;

                let this_time = NaiveDateTime::new(
                    NaiveDate::from_ymd(2000 + year as i32, month, day),
                    NaiveTime::from_hms(hour, minute, 0),
                );

                match base_time {
                    Some(base_time) => {
                        if this_time != base_time {
                            bail!("Found different times: {base_time} and {this_time}")
                        }
                    }
                    None => base_time = Some(this_time),
                }

                loop {
                    let mut identifier_bytes = [0u8; 3];

                    entry.read_exact(&mut identifier_bytes[0..1]).context("Failed reading byte 0 for identifier of next information")?;
                    if identifier_bytes[0] == 0x03 {
                        // etx => end of text
                        break;
                    }

                    entry.read_exact(&mut identifier_bytes[1..2]).context("Failed reading byte 1 for identifier of next information")?;

                    let found_match = match &identifier_bytes[0..2] {
                        b"BY" => {
                            let _product_length = entry.integer(7).context("Failed extracting product_length")?;
                            // we don't verify this -- if the file is truncated, we will panic anyway, no use in checking this here
                            true
                        },
                        b"VS" => {
                            entry.ensure_next_is_space().context("Failed ensuring a space in front of format version")?;
                            let format_version = entry.integer(1).context("Failed extracting format_version")?;
                            ensure!(format_version == 3, "Format version {format_version} not supported (expected version 3)");
                            true
                        },
                        b"SW" => {
                            let _software_version = entry.array::<9>().context("Failed reading software_version")?;
                            // not handling this in any way -- we don't care what software DWD is using as long as its output is spec conformant
                            true
                        },
                        b"PR" => {
                            entry.ensure_next_is_space().context("Failed ensuring a space in front of precision")?;
                            let precision = entry.array::<4>().context("Failed reading precision")?;
                            ensure!(precision == *b"E-02", "Precision {precision:?} is not supported (expected \"E-02\")");
                            true
                        },
                        b"GP" => {
                            let resolution = entry.array::<9>().context("Failed reading resolution")?;
                            ensure!(resolution == *b"1200x1100", "Expected resolution to be \"1200x1100\" but found {resolution:?}");
                            true
                        },
                        b"VV" => {
                            entry.ensure_next_is_space().context("Failed ensuring a space in front of prediction_time")?;
                            let prediction_time = entry.integer(3).context("Failed extracting product_length")?;
                            ensure!(prediction_time == (index as u32) * 5, "Expected prediction_time to be {} (index * 5) but found {prediction_time:?}", index * 5);
                            true
                        },
                        b"MF" => {
                            entry.ensure_next_is_space().context("Failed ensuring a space in front of module_flags")?;
                            let _module_flags = entry.integer(8).context("Failed extracting module_flags")?;
                            // no idea what those mean
                            true
                        },
                        b"MS" => {
                            let text_length = entry.integer(3).context("Failed extracting text_length")?;
                            let _text = entry.vec(text_length as usize).context("Failed extracting text")?;
                            // no idea what this text means
                            true
                        },
                        _ => false
                    };

                    if !found_match {
                        entry.read_exact(&mut identifier_bytes[2..3]).context("Failed reading byte 2 for identifier of next information")?;
                        if identifier_bytes == *b"INT" {
                            let interval = entry.integer(4).context("Failed reading interval")?;
                            ensure!(interval == 5, "Expected interval to be 5 but found {interval}")
                        } else {
                            bail!("Unknown information identifier {identifier_bytes:?}")
                        }
                    }
                }

                let mut data = vec![];
                entry.read_to_end(&mut data).context("Failed reading data")?;

                ensure!(data.len() == 2640000, "Binary file size is wrong -- expected 2640000 byte but got {}", data.len());
                Ok(data)
            })
            .collect::<Result<Vec<_>>>().context("Failed parsing archived files")?;

        ensure!(
            predictions.len() == 25,
            "Expected predictions.len() to be 25, but found {}",
            predictions.len()
        );

        let base_time =
            base_time.ok_or_else(|| anyhow!("Did not find base_time (this shouldn't happen)"))?;

        Ok(Self {
            base_time,
            predictions,
        })
    }
}

pub struct TimeIterator<'a> {
    radar_values: &'a DWDRainRadarValues,
    current_index_iter: std::ops::Range<usize>,
}

impl<'a> std::iter::Iterator for TimeIterator<'a> {
    type Item = NaiveDateTime;
    fn next(&mut self) -> Option<NaiveDateTime> {
        (&mut self.current_index_iter)
            .map(|current_index| {
                self.radar_values.base_time + chrono::Duration::minutes(current_index as i64 * 5)
            })
            .next()
    }
}

pub struct Iterator<'a, X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>>
{
    radar_values: &'a DWDRainRadarValues,
    prediction_index: usize,
    current_index_iter: std::iter::Zip<X, Y>,
}

impl<'a, X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>>
    std::iter::Iterator for Iterator<'a, X, Y>
{
    type Item = Option<u16>;
    fn next(&mut self) -> Option<Option<u16>> {
        (&mut self.current_index_iter)
            .map(|(x, y)| -> Option<u16> {
                let y = 1199 - y; // Binärformat beginnt unten, aber unsere Koordinaten beginnen oben

                let offset = 2 * (1100 * y + x);

                let result = match u16::from_le_bytes(
                    self.radar_values.predictions[self.prediction_index][offset..offset + 2]
                        .try_into()
                        .expect("Could not get bytes from predictions (this should not happen)"),
                ) {
                    0x29C4 => None,
                    result => Some(result),
                };

                if result.iter().any(|result| *result >= 4096) {
                    panic!("Result is bigger than 4095: {result:?}");
                }

                result
            })
            .next()
    }
}

impl RainRadarValues for DWDRainRadarValues {
    type Iter<'a, X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>> =
        Iterator<'a, X, Y>;
    type TimeIter<'a> = TimeIterator<'a>;

    fn for_area<X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>>(
        &self,
        time: chrono::naive::NaiveDateTime,
        x: X,
        y: Y,
    ) -> Self::Iter<'_, X, Y> {
        let duration = time - self.base_time;
        let prediction_index: usize = (duration.num_minutes() / 5)
            .try_into()
            .expect("prediction_index is not usize");
        assert_eq!(
            chrono::Duration::minutes(prediction_index as i64 * 5),
            duration,
            "Illegal duration: Not multiple of 5 minutes"
        );
        assert!(prediction_index < self.predictions.len());
        Iterator {
            radar_values: self,
            prediction_index,
            current_index_iter: x.zip(y),
        }
    }

    fn available_times(&self) -> TimeIterator<'_> {
        TimeIterator {
            radar_values: self,
            current_index_iter: (0..25),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_file() -> Result<()> {
        let dwd_rain_radar_values = DWDRainRadarValues::from_file(format!(
            "{}/test_values/{}.tar.bz2",
            std::env::var("CARGO_MANIFEST_DIR")
                .context("Failed getting manifest dir env variable")?, "valid"
        ))?;

        assert_eq!(dwd_rain_radar_values.available_times().count(), 25);

        for time in dwd_rain_radar_values.available_times() {
            let i1 = dwd_rain_radar_values.for_area(time, 0..5, 0..1200);
            let i2 = dwd_rain_radar_values.for_area(time, 1095..1100, 0..1200);
            let i3 = dwd_rain_radar_values.for_area(time, 0..1100, 0..5);
            let i4 = dwd_rain_radar_values.for_area(time, 0..1100, 1095..1200);

            for value in i1.chain(i2).chain(i3).chain(i4) {
                assert!(value.is_none());
            }

            for _value in dwd_rain_radar_values.for_area(time, 0..1100, 0..1200) {
                // Doing nothing with it. We just want to know whether iterating over all returns errors
            }
        }

        Ok(())
    }

}
