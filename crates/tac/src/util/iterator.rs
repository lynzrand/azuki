pub enum OptionIter<T> {
    None,
    One(T),
    Two(T, T),
    // Iter(I),
}

impl<T> Iterator for OptionIter<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let this = std::mem::replace(self, OptionIter::None);
        match this {
            OptionIter::None => None,
            OptionIter::One(i) => Some(i),
            OptionIter::Two(i, j) => {
                *self = OptionIter::One(j);
                Some(i)
            }
        }
    }
}

pub enum VarIter<T, I> {
    None,
    One(T),
    Two(T, T),
    Iter(I),
}

impl<T, I> Iterator for VarIter<T, I>
where
    T: Clone,
    I: Iterator<Item = T>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let this = std::mem::replace(self, VarIter::None);
        match this {
            VarIter::None => None,
            VarIter::One(i) => Some(i),
            VarIter::Two(i, j) => {
                *self = VarIter::One(j);
                Some(i)
            }
            VarIter::Iter(mut it) => {
                let next = it.next();
                *self = VarIter::Iter(it);
                next
            }
        }
    }
}
