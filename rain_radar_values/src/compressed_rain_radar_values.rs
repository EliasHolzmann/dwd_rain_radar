use crate::{CrossIteratorExt, RainRadarValues, TimeInformation};

mod aligned_alloc {
    // from https://stackoverflow.com/a/69544158/4674154
    use std::alloc::*;
    use std::ptr::NonNull;
    pub struct AlignedAlloc<const N: usize>;
    unsafe impl<const N: usize> Allocator for AlignedAlloc<N> {
        fn allocate(&self, layout: Layout) -> Result<NonNull<[u8]>, AllocError> {
            Global.allocate(layout.align_to(N).unwrap())
        }
        unsafe fn deallocate(&self, ptr: NonNull<u8>, layout: Layout) {
            Global.deallocate(ptr, layout.align_to(N).unwrap())
        }
    }
}
use aligned_alloc::AlignedAlloc;

pub struct CompressedRainRadarValues {
    // format specification: everything is little endian.
    // bytes 0 - 7: UNIX timestamp for base time
    // bytes 8 - 6607: [[[u16; 12]; 11]; 25] –
    //   - location of 100x100 value blocks
    //   - 0xFFFF => values are all nonexistant
    //   - 0x7FFF => values are all 0
    //   - all other values: Highest bit: values are 16 bit iff 1, 8 bit iff 0. Lowest 15 bits: Offset beginning from byte 6608 in 10000 byte steps.
    // byte 6608 and onwards: The real values in blocks of 100x100, either u8 or u16 (see above), if u8::MAX/u16::MAX: value missing
    data: Box<[u8], AlignedAlloc<2>>,
}

impl CompressedRainRadarValues {
    pub fn from_rain_radar_values<T: super::RainRadarValues>(from: &T) -> Self {
        let time_information = from.time_information();
        assert_eq!(time_information.available_time_slots, 25);

        let first_time = (time_information.first_time.timestamp() as u64).to_le_bytes();

        let mut offsets: Vec<u8> = Vec::with_capacity(6600);
        let mut current_offset_from_start: u16 = 0;
        let mut values_vec: Vec<u8> = Vec::with_capacity(100000);

        for time_offset in 0..25 {
            let time =
                time_information.first_time + chrono::Duration::minutes(time_offset as i64 * 5);
            for x in 0..11 {
                for y in 0..12 {
                    let values_in_block = from
                        .for_area(time, (x * 100)..((x + 1) * 100), (y * 100)..((y + 1) * 100))
                        .collect::<Vec<Option<u16>>>();
                    assert_eq!(values_in_block.len(), 10000);

                    let offset: u16 = if values_in_block.iter().all(|value| *value == None) {
                        0xFFFF
                    } else if values_in_block.iter().all(|value| *value == Some(0)) {
                        0x7FFF
                    } else if values_in_block.iter().any(|value| value.unwrap_or(0) > 254) {
                        assert!(current_offset_from_start < 0x7FFF);
                        let offset = current_offset_from_start | (1 << 15);
                        current_offset_from_start += 2;
                        let values_as_bytes_iter = values_in_block
                            .iter()
                            .map(|value| value.unwrap_or(u16::MAX))
                            .flat_map(|value| value.to_le_bytes().into_iter());
                        debug_assert!(values_as_bytes_iter.clone().count() == 20000);
                        values_vec.extend(values_as_bytes_iter);
                        offset
                    } else {
                        let offset = current_offset_from_start;
                        current_offset_from_start += 1;
                        let values_as_bytes_iter = values_in_block
                            .into_iter()
                            .map(|value| value.map(|value| value as u8))
                            .map(|value| value.unwrap_or(u8::MAX))
                            .flat_map(|value| value.to_le_bytes().into_iter());
                        debug_assert!(values_as_bytes_iter.clone().count() == 10000);
                        values_vec.extend(values_as_bytes_iter);
                        offset
                    };
                    offsets.extend(offset.to_le_bytes().into_iter());
                }
            }
        }

        assert_eq!(offsets.len(), 6600);
        assert_eq!(values_vec.len(), current_offset_from_start as usize * 10000);

        let mut data = Vec::new_in(AlignedAlloc::<2>);

        data.extend_from_slice(&first_time);
        data.extend_from_slice(&offsets);
        data.extend_from_slice(&values_vec);

        let data = data.into_boxed_slice();

        assert_eq!(
            data.len(),
            6608 + current_offset_from_start as usize * 10000
        );

        Self { data }
    }

    fn first_time(&self) -> chrono::NaiveDateTime {
        chrono::NaiveDateTime::from_timestamp(
            i64::from_le_bytes(
                self.data[0..8]
                    .try_into()
                    .expect("Could not convert to [u8; 8] (this should not happen"),
            ),
            0,
        )
    }

    fn block_offsets(&self) -> &[[[u16; 12]; 11]; 25] {
        let block_offsets_byte_area = &self.data[8..6608];

        assert!(block_offsets_byte_area.len() == std::mem::size_of::<[[[u16; 12]; 11]; 25]>());

        unsafe {
            // this should be sound: the slice has the correct length (as asserted above)
            &*(block_offsets_byte_area as *const [u8] as *const [[[u16; 12]; 11]; 25])
        }
    }

    fn block_u8(&self, offset: usize) -> &[[u8; 100]; 100] {
        let offset = 6608 + (offset * 10000);
        let block_byte_area = &self.data[offset..(offset + 10000)];

        assert!(block_byte_area.len() == std::mem::size_of::<[[u8; 100]; 100]>());

        unsafe {
            // this should be sound: the slice has the correct length (as asserted above)
            &*(block_byte_area as *const [u8] as *const [[u8; 100]; 100])
        }
    }

    fn block_u16(&self, offset: usize) -> &[[u16; 100]; 100] {
        let offset = 6608 + (offset * 10000);
        assert!(offset % 2 == 0); // alignment of the buffer is guaranteed as it is allocated via AlignedAlloc<2>
        let block_byte_area = &self.data[offset..(offset + 20000)];

        assert!(block_byte_area.len() == std::mem::size_of::<[[u16; 100]; 100]>());

        unsafe {
            // this should be sound: the slice has the correct length (as asserted above)
            &*(block_byte_area as *const [u8] as *const [[u16; 100]; 100])
        }
    }

    fn reader(&self) -> impl std::io::Read + '_ {
        std::io::Cursor::new(&self.data)
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}

pub struct Iterator<'a, X: super::Range, Y: super::Range> {
    radar_values: &'a CompressedRainRadarValues,
    prediction_index: usize,
    current_index_iter: super::CrossProduct<X, Y>,
}

impl<'a, X: super::Range, Y: super::Range> std::iter::Iterator for Iterator<'a, X, Y> {
    type Item = Option<u16>;

    fn next(&mut self) -> Option<Option<u16>> {
        (&mut self.current_index_iter)
            .map(|(x, y)| -> Option<u16> {
                let x_block = x / 100;
                let y_block = y / 100;
                assert!(x_block < 11);
                assert!(y_block < 12);

                let offsets = self.radar_values.block_offsets();
                let offset = u16::from_le(offsets[self.prediction_index][x_block][y_block]);

                let highest_bit_set = (offset & 0x8000) != 0;

                let (is_16_bit, offset) = match (highest_bit_set, offset) {
                    (_, 0xFFFF) => return None,
                    (_, 0x7FFF) => return Some(0),
                    (is_16_bit, offset) => (is_16_bit, (offset & 0x7FFF) as usize),
                };

                if is_16_bit {
                    let value = self.radar_values.block_u16(offset)[y % 100][x % 100];
                    if value == u16::MAX {
                        None
                    } else {
                        Some(value)
                    }
                } else {
                    let value = self.radar_values.block_u8(offset)[y % 100][x % 100];
                    if value == u8::MAX {
                        None
                    } else {
                        Some(value as u16)
                    }
                }
            })
            .next()
    }
}

impl RainRadarValues for CompressedRainRadarValues {
    type Iter<'a, X: super::Range, Y: super::Range> = Iterator<'a, X, Y>;

    fn for_area<X: super::Range, Y: super::Range>(
        &self,
        time: chrono::naive::NaiveDateTime,
        x: X,
        y: Y,
    ) -> Iterator<X, Y> {
        let duration = time - self.first_time();
        let prediction_index: usize = (duration.num_minutes() / 5)
            .try_into()
            .expect("prediction_index is not usize");
        assert_eq!(
            chrono::Duration::minutes(prediction_index as i64 * 5),
            duration,
            "Illegal duration: Not multiple of 5 minutes"
        );
        assert!(prediction_index < 25);
        Iterator {
            radar_values: self,
            prediction_index,
            current_index_iter: x.cross_product(y),
        }
    }

    fn time_information(&self) -> crate::TimeInformation {
        TimeInformation {
            first_time: self.first_time(),
            available_time_slots: 25,
        }
    }
}

#[cfg(test)]
mod test {
    use anyhow::{anyhow, Context, Result};
    use rayon::prelude::*;

    use super::*;

    #[test]
    fn test_file() -> Result<()> {
        crate::local_file_analysis::selected_files()
            .into_par_iter()
            .map(|path| -> Result<()> {
                let dwd_rain_radar_values = crate::DWDRainRadarValues::from_file(path)
                    .with_context(|| anyhow!("Opening {path:?} failes"))?;

                let compressed_rain_radar_values =
                    CompressedRainRadarValues::from_rain_radar_values(&dwd_rain_radar_values);

                dbg!(compressed_rain_radar_values.data.len());

                std::io::copy(
                    &mut compressed_rain_radar_values.reader(),
                    &mut std::fs::File::create("/tmp/compressed_data").unwrap(),
                )
                .unwrap();

                let compressed_times: Vec<chrono::NaiveDateTime> =
                    compressed_rain_radar_values.available_times().collect();
                let dwd_times: Vec<chrono::NaiveDateTime> =
                    dwd_rain_radar_values.available_times().collect();

                assert_eq!(compressed_times, dwd_times);

                for time in compressed_times {
                    let mut dwd_rain_radar_values =
                        dwd_rain_radar_values.for_area(time, 0..1100, 0..1200);
                    let mut compressed_rain_radar_values =
                        compressed_rain_radar_values.for_area(time, 0..1100, 0..1200);

                    for (index, (dwd_value, compressed_value)) in (&mut dwd_rain_radar_values)
                        .zip(&mut compressed_rain_radar_values)
                        .enumerate()
                    {
                        assert_eq!(
                            dwd_value, compressed_value,
                            "{dwd_value:?} != {compressed_value:?} (index {index}, time {time})"
                        )
                    }
                    assert!(dwd_rain_radar_values.next().is_none());
                    assert!(compressed_rain_radar_values.next().is_none());
                }
                Ok(())
            })
            .collect()
    }
}
