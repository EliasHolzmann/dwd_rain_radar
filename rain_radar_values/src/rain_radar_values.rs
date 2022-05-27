pub struct TimeInformation {
    pub first_time: chrono::naive::NaiveDateTime,
    pub available_time_slots: u32,
}

type TimeIterClousure =
    impl FnMut((usize, chrono::naive::NaiveDateTime)) -> chrono::naive::NaiveDateTime;

pub struct TimeIter {
    inner: std::iter::Map<
        std::iter::Enumerate<std::iter::Take<std::iter::Repeat<chrono::naive::NaiveDateTime>>>,
        TimeIterClousure,
    >,
}

impl Iterator for TimeIter {
    type Item = chrono::naive::NaiveDateTime;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next()
    }
}

pub trait Range: std::iter::Iterator<Item = usize> + Clone {}
impl<T: std::iter::Iterator<Item = usize> + Clone> Range for T {}

pub trait RainRadarValues {
    type Iter<'a, X: Range, Y: Range>: Iterator<Item = Option<u16>>
    where
        Self: 'a;

    fn for_area<X: Range, Y: Range>(
        &self,
        time: chrono::naive::NaiveDateTime,
        x: X,
        y: Y,
    ) -> Self::Iter<'_, X, Y>;

    fn time_information(&self) -> TimeInformation;

    fn available_times(&self) -> TimeIter {
        let time_information = self.time_information();
        fn map_index_and_first_time(
            index_and_first_time: (usize, chrono::NaiveDateTime),
        ) -> chrono::NaiveDateTime {
            let (index, first_time) = index_and_first_time;
            first_time + chrono::Duration::minutes(index as i64 * 5)
        }
        TimeIter {
            inner: std::iter::repeat(time_information.first_time)
                .take(time_information.available_time_slots as usize)
                .enumerate()
                .map(map_index_and_first_time),
        }
    }

    #[cfg(feature = "downloads_analyzer")]
    fn to_bmp<P: AsRef<std::path::Path>>(&self, path: P) {
        use std::path::*;
        let path = PathBuf::from(path.as_ref());
        if !path.is_dir() {
            std::fs::create_dir_all(&path).expect("Failed creating directory");
        }

        for time in self.available_times() {
            let mut path = path.clone();
            let file_name = time.format("%Y%m%d%H%M%S.bmp");
            path.push(file_name.to_string());

            let mut image = bmp::Image::new(1100, 1200);

            for x in 0..1100 {
                for y in 0..1200 {
                    let pixel_value = self
                        .for_area(time, x..=x, y..=y)
                        .next()
                        .expect("Couldn't get pixel (this shouldn't happen)");

                    let (r, g, b): (u8, u8, u8) = match pixel_value {
                        None => (0x99, 0x99, 0x99),
                        Some(pixel_value) if pixel_value <= 255 => {
                            (0xff - pixel_value as u8, 0xff - pixel_value as u8, 0xff)
                        }
                        Some(_) => (0xff, 0x00, 0x00),
                    };
                    image.set_pixel(x as u32, y as u32, bmp::Pixel::new(r, g, b));
                }
            }

            dbg!(&path);

            image.save(path).expect("Failed sabing bmp");
        }
    }
}
