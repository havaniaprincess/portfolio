pub fn min<T> (left: T, right: T) -> T 
where 
    T: std::cmp::PartialOrd
{
    if left < right {
        left
    } else {
        right
    }
}
pub trait _IntervalMath {
    type T;
    type S;
    fn collision(&self, 
        right: &Self
    ) -> bool;
    fn separate(&self,
        divider: &Self
    ) -> (Option<Self::T>, Option<Self::T>);
    fn intersection(&self, 
        right: &Self
    ) -> Option<Self::T>;
    fn duration(&self) -> Self::S;
    fn check_interval(&self) -> bool;
}