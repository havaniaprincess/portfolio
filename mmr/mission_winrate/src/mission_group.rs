

#[derive(Clone, PartialEq, Hash, Eq)]
pub enum BalanceCat{
    Faction1(String),
    Faction2(String),
    Balance
}

#[derive(Clone, PartialEq, Hash, Eq)]
pub enum WinTeam{
    Faction1(String),
    Faction2(String)
}



impl std::string::ToString for BalanceCat {
    fn to_string(&self) -> String {
        match self {
            BalanceCat::Faction1(nation) => nation.to_owned() + "_unbalanced",
            BalanceCat::Faction2(nation) => nation.to_owned() + "_unbalanced",
            BalanceCat::Balance => "balanced".to_string(),
        }
    }
}
impl std::string::ToString for WinTeam {
    fn to_string(&self) -> String {
        match self {
            WinTeam::Faction1(nation) => nation.to_owned() + "_win",
            WinTeam::Faction2(nation) => nation.to_owned() + "_win",
        }
    }
}