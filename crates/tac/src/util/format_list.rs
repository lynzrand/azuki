use std::fmt::Display;

pub struct ListFormatter<I>(I);

impl<I, T> ListFormatter<I>
where
    I: Iterator<Item = T> + Clone,
{
    pub fn new(iter: I) -> ListFormatter<I> {
        ListFormatter(iter)
    }
}

impl<I, T> Display for ListFormatter<I>
where
    I: Iterator<Item = T> + Clone,
    T: Display,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut iter = self.0.clone();
        if let Some(first) = iter.next() {
            write!(f, "{}", first)?;
            for other in iter {
                write!(f, ", {}", other)?;
            }
        }
        Ok(())
    }
}
