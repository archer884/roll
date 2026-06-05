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

#[cfg(test)]
mod tests {
    use super::DefaultIfEmpty;

    #[test]
    fn empty_iterator_yields_default() {
        let items: Vec<i32> = vec![];
        let result: Vec<_> = items.into_iter().default_if_empty(42).collect();
        assert_eq!(result, vec![42]);
    }

    #[test]
    fn non_empty_iterator_yields_all_items() {
        let result: Vec<_> = vec![1, 2, 3].into_iter().default_if_empty(42).collect();
        assert_eq!(result, vec![1, 2, 3]);
    }

    #[test]
    fn single_item_iterator_ignores_default() {
        let result: Vec<_> = vec![99].into_iter().default_if_empty(42).collect();
        assert_eq!(result, vec![99]);
    }

    #[test]
    fn exhausted_iterator_returns_none_after_default() {
        let items: Vec<i32> = vec![];
        let mut iter = items.into_iter().default_if_empty(42);
        assert_eq!(iter.next(), Some(42));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn exhausted_iterator_returns_none_after_items() {
        let mut iter = vec![1, 2].into_iter().default_if_empty(42);
        assert_eq!(iter.next(), Some(1));
        assert_eq!(iter.next(), Some(2));
        assert_eq!(iter.next(), None);
    }

    #[test]
    fn works_with_strings() {
        let items: Vec<String> = vec![];
        let result: Vec<_> = items
            .into_iter()
            .default_if_empty("default".to_string())
            .collect();
        assert_eq!(result, vec!["default"]);
    }
}
