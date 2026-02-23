

pub trait Tagging {
    fn get_compare_str(&self) -> String;
}

pub trait TaggingItem {
    type OutTags: ?Sized;
    fn get_tags(&self) -> &Self::OutTags;
    fn get_primary_tags(&self) -> &Self::OutTags;
}