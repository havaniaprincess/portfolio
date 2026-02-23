pub trait Transponent<T> {
    type OutType;
    fn transponent(&self) -> Self::OutType;
}