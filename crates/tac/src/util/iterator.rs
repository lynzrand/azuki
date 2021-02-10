
pub enum VarIter<T> {
    None,
    One(T),
    // Two(T, T),
    // Iter(I),
}

impl<T> Iterator for VarIter<T>
where
    T: Clone,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        let this = std::mem::replace(self, VarIter::None);
        match this {
            VarIter::None => None,
            VarIter::One(i) => Some(i),
        }
    }
}
