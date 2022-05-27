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
}
