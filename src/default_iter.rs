pub struct DefaultIfEmptyIter<T: Iterator> {
    default: Option<T::Item>,
    iterator: T,
    non_empty: bool,
}

impl<T: Iterator> Iterator for DefaultIfEmptyIter<T> {
    type Item = T::Item;

    fn next(&mut self) -> Option<Self::Item> {
        match self.iterator.next() {
            Some(t) => {
                self.non_empty = true;
                Some(t)
            }

            None if self.non_empty => None,
            None => self.default.take(),
        }
    }
}

pub trait DefaultIfEmpty<T: Iterator> {
    fn default_if_empty(self, default: T::Item) -> DefaultIfEmptyIter<T>;
}

impl<T> DefaultIfEmpty<T> for T
where
    T: Iterator,
{
    fn default_if_empty(self, default: <T as Iterator>::Item) -> DefaultIfEmptyIter<T> {
        DefaultIfEmptyIter {
            default: Some(default),
            iterator: self,
            non_empty: false,
        }
    }
}
