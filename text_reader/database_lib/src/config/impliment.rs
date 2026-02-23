use crate::config::{traits::{Tagging, TaggingItem}, Exp, ExpType, Fish};


impl Tagging for ExpType {
    fn get_compare_str(&self) -> String {
        match self {
            ExpType::AllExp => "всегоопыта".to_string(),
            ExpType::Base => "очковопыта".to_string(),
            ExpType::PremBonus => "премиум".to_string(),
            ExpType::DrinkBonus => "алкоголь".to_string(),
            ExpType::HappyBonus => "счастливыйчас".to_string(),
            ExpType::LBonus => "снасть".to_string(),
            ExpType::Real => "jhdfhsdfkjhsdjfhjsdhfksdjhf".to_string(),
        }
        
    }
}
impl Tagging for String {
    fn get_compare_str(&self) -> String {
        self.replace(" ", "")       
    }
}

impl TaggingItem for Fish {
    type OutTags = Vec<String>;
    fn get_tags(&self) -> &Self::OutTags {
        &self.tags
    }
    fn get_primary_tags(&self) -> &Self::OutTags {
        &self.primary_tags
    }
}

impl TaggingItem for Exp {
    type OutTags = Vec<String>;
    fn get_tags(&self) -> &Self::OutTags {
        &self.tags
    }
    fn get_primary_tags(&self) -> &Self::OutTags {
        &self.primary_tags
    }
}