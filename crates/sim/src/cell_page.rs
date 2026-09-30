//! Lossless pages: uniform and short runs avoid paying per-cell storage for
//! accumulated marks or one learning revision. Busy pages retain dense cells.
const CELLS: usize = 256;

#[derive(Clone)]
pub(crate) struct Run<T> {
    end: u16,
    value: T,
}

#[derive(Clone)]
pub(crate) enum Page<T> {
    Uniform(T),
    Runs(Vec<Run<T>>),
    Dense(Box<[T; CELLS]>),
}

impl<T: Copy + Eq + Default> Default for Page<T> {
    fn default() -> Self {
        Self::Uniform(T::default())
    }
}

impl<T: Copy + Eq + Default> Page<T> {
    pub fn get(&self, cell: usize) -> T {
        match self {
            Self::Uniform(value) => *value,
            Self::Runs(runs) => runs[runs.partition_point(|run| run.end as usize <= cell)].value,
            Self::Dense(cells) => cells[cell],
        }
    }

    pub fn set(&mut self, cell: usize, value: T) {
        if self.get(cell) == value {
            return;
        }
        if let Self::Dense(cells) = self {
            cells[cell] = value;
            return;
        }
        if let Self::Uniform(old) = self {
            *self = Self::Runs(vec![Run {
                end: CELLS as u16,
                value: *old,
            }]);
        }
        let Self::Runs(runs) = self else {
            unreachable!()
        };
        let at = runs.partition_point(|run| run.end as usize <= cell);
        let start = if at == 0 {
            0
        } else {
            runs[at - 1].end as usize
        };
        let end = runs[at].end as usize;
        let old = runs[at].value;
        // Editing one cell splits one run; adjacent equal values merge again.
        let mut replacement = std::array::from_fn::<_, 3, _>(|_| Run { end: 0, value: old });
        let mut count = 0;
        if start < cell {
            replacement[count] = Run {
                end: cell as u16,
                value: old,
            };
            count += 1;
        }
        replacement[count] = Run {
            end: (cell + 1) as u16,
            value,
        };
        count += 1;
        if cell + 1 < end {
            replacement[count] = Run {
                end: end as u16,
                value: old,
            };
            count += 1;
        }
        runs.splice(at..=at, replacement.into_iter().take(count));
        let mut i = at.saturating_sub(1);
        while i + 1 < runs.len() && i <= at + 2 {
            if runs[i].value == runs[i + 1].value {
                runs[i].end = runs[i + 1].end;
                runs.remove(i + 1);
            } else {
                i += 1;
            }
        }
        if runs.len() == 1 {
            *self = Self::Uniform(runs[0].value);
        } else if runs.capacity() * std::mem::size_of::<Run<T>>()
            >= std::mem::size_of::<[T; CELLS]>()
        {
            *self = Self::Dense(Box::new(std::array::from_fn(|i| self.get(i))));
        }
    }

    pub fn compress(&mut self) {
        let Self::Dense(cells) = self else { return };
        let count = 1 + cells.windows(2).filter(|pair| pair[0] != pair[1]).count();
        if count == 1 {
            *self = Self::Uniform(cells[0]);
            return;
        }
        if count * std::mem::size_of::<Run<T>>() >= std::mem::size_of::<[T; CELLS]>() {
            return;
        }
        let mut runs = Vec::with_capacity(count);
        let mut value = cells[0];
        for (i, &next) in cells.iter().enumerate().skip(1) {
            if next != value {
                runs.push(Run {
                    end: i as u16,
                    value,
                });
                value = next;
            }
        }
        runs.push(Run {
            end: CELLS as u16,
            value,
        });
        *self = Self::Runs(runs);
    }

    pub fn bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + match self {
                Self::Uniform(_) => 0,
                Self::Runs(runs) => runs.capacity() * std::mem::size_of::<Run<T>>(),
                Self::Dense(_) => std::mem::size_of::<[T; CELLS]>(),
            }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uniform_writes_stay_compact_before_the_seal() {
        let mut page = Page::<u32>::default();
        for i in 0..CELLS {
            page.set(i, 19);
            assert!(
                page.bytes() < 128,
                "uniform learning expanded while writing cell {i}"
            );
        }
        for i in (0..CELLS).rev() {
            page.set(i, 31);
            assert!(page.bytes() < 128);
        }
        for i in 0..CELLS {
            assert_eq!(page.get(i), 31);
        }
    }

    #[test]
    fn uniform_and_run_pages_store_values_without_a_dense_cell_copy() {
        for values in [[19u32; CELLS], std::array::from_fn(|i| (i / 64) as u32)] {
            let mut page = Page::default();
            for (i, value) in values.into_iter().enumerate() {
                page.set(i, value);
            }
            page.compress();
            assert!(page.bytes() < std::mem::size_of::<[u32; CELLS]>() / 8);
            for (i, value) in values.into_iter().enumerate() {
                assert_eq!(page.get(i), value);
            }
        }
    }

    #[test]
    fn compression_preserves_uniform_runs_and_all_distinct_values() {
        for values in [
            [17u32; CELLS],
            std::array::from_fn(|i| (i / 31) as u32),
            std::array::from_fn(|i| i as u32),
        ] {
            let mut page = Page::default();
            for (i, value) in values.into_iter().enumerate() {
                page.set(i, value);
            }
            page.compress();
            for (i, value) in values.into_iter().enumerate() {
                assert_eq!(page.get(i), value);
            }
            page.set(0, 911);
            page.set(255, 733);
            page.compress();
            assert_eq!(page.get(0), 911);
            assert_eq!(page.get(255), 733);
            for (i, value) in values.into_iter().enumerate().skip(1).take(254) {
                assert_eq!(page.get(i), value);
            }
        }
    }
}
