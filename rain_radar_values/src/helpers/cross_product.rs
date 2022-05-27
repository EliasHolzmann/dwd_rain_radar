pub(crate) struct CrossProduct<T: Iterator + Clone, U: Iterator> {
    iter1_start: T,
    iter1_now: T,
    iter2: std::iter::Peekable<U>,
}

pub(crate) trait CrossIteratorExt<T>: Iterator<Item = T> + Clone {
    fn cross_product<U: Iterator>(self, iter2: U) -> CrossProduct<Self, U> {
        CrossProduct {
            iter1_start: self.clone(),
            iter1_now: self,
            iter2: iter2.peekable(),
        }
    }
}

impl<T, I: Iterator<Item = T> + Clone> CrossIteratorExt<T> for I {}

impl<T: Iterator + Clone, U: Iterator> Iterator for CrossProduct<T, U>
where
    U::Item: Clone,
{
    type Item = (T::Item, U::Item);

    fn next(&mut self) -> Option<Self::Item> {
        let iter1_next = self.iter1_now.next();
        let iter2_next = self.iter2.peek();
        match (iter1_next, iter2_next) {
            (_, None) => None,
            (None, _) => {
                self.iter1_now = self.iter1_start.clone();
                self.iter2.next();
                self.next()
            }
            (Some(iter1_next), Some(iter2_next)) => Some((iter1_next, iter2_next.clone())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn cross_product_0_to_4() {
        assert_eq!(
            (0..5).cross_product(0..5).collect::<Vec<_>>(),
            vec![
                (0, 0),
                (1, 0),
                (2, 0),
                (3, 0),
                (4, 0),
                (0, 1),
                (1, 1),
                (2, 1),
                (3, 1),
                (4, 1),
                (0, 2),
                (1, 2),
                (2, 2),
                (3, 2),
                (4, 2),
                (0, 3),
                (1, 3),
                (2, 3),
                (3, 3),
                (4, 3),
                (0, 4),
                (1, 4),
                (2, 4),
                (3, 4),
                (4, 4),
            ]
        );
    }
}
