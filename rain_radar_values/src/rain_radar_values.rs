pub trait RainRadarValues {
    type Iter<'a, X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>>: Iterator<Item = Option<u16>>
    where
        Self: 'a;
    type TimeIter<'a>: Iterator<Item = chrono::naive::NaiveDateTime>
    where
        Self: 'a;

    fn for_area<X: std::iter::Iterator<Item = usize>, Y: std::iter::Iterator<Item = usize>>(
        &self,
        time: chrono::naive::NaiveDateTime,
        x: X,
        y: Y,
    ) -> Self::Iter<'_, X, Y>;

    fn available_times(&self) -> Self::TimeIter<'_>;
}
