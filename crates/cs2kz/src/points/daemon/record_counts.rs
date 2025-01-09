use std::collections::VecDeque;

use crate::maps::courses::CourseFilterId;

pub type RecordCounts = Counts<CourseFilterId>;

pub struct Counts<T> {
    entries: VecDeque<(T, u64)>,
}

impl<T> Counts<T> {
    pub fn new() -> Self {
        Self { entries: VecDeque::new() }
    }

    pub fn push(&mut self, value: T)
    where
        T: Eq,
    {
        let Some(idx) = self.entries.iter().position(|entry| entry.0 == value) else {
            self.entries.push_back((value, 1));
            return;
        };

        self.entries[idx].1 += 1;

        if let Some(new_idx) = self
            .entries
            .range(..=idx)
            .enumerate()
            .rev()
            .filter(|&(_, &(_, count))| count == self.entries[idx].1 - 1)
            .last()
            .map(|(idx, _)| idx)
        {
            self.entries.swap(idx, new_idx);
        }
    }

    pub fn pop(&mut self) -> Option<(T, u64)> {
        self.entries.pop_front()
    }

    pub fn remove(&mut self, value: T) -> Option<u64>
    where
        T: Eq,
    {
        self.entries
            .iter()
            .position(|entry| entry.0 == value)
            .and_then(|idx| self.entries.remove(idx))
            .map(|(_, count)| count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_push() {
        let mut counts = Counts::<()>::new();

        counts.push(());

        assert_eq!(counts.entries[0], ((), 1));
    }

    #[test]
    fn few_pushes() {
        let mut counts = Counts::<i32>::new();

        counts.push(0);
        counts.push(1);
        counts.push(2);

        assert_eq!(counts.entries, vec![(0, 1), (1, 1), (2, 1)]);
    }

    #[test]
    fn few_pushes_and_increment() {
        let mut counts = Counts::<i32>::new();

        counts.push(0);
        counts.push(1);
        counts.push(1);
        counts.push(2);

        assert_eq!(counts.entries, vec![(1, 2), (0, 1), (2, 1)]);
    }

    #[test]
    fn more_pushes_and_increment() {
        let mut counts = Counts::<i32>::new();

        counts.push(0);
        counts.push(1);
        counts.push(1);
        counts.push(1);
        counts.push(2);
        counts.push(1);
        counts.push(1);
        counts.push(2);
        counts.push(2);

        assert_eq!(counts.entries, vec![(1, 5), (2, 3), (0, 1)]);
    }

    #[test]
    fn few_pushes_and_pops() {
        let mut counts = Counts::<i32>::new();

        counts.push(0);
        counts.push(1);
        counts.push(1);
        counts.push(2);

        assert_eq!(counts.pop(), Some((1, 2)));
        assert_eq!(counts.entries, vec![(0, 1), (2, 1)]);
    }
}
