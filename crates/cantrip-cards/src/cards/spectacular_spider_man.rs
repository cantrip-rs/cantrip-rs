use cantrip_api::*;
use cantrip_mechanics::effects::{self, this_gains_flying};
use cantrip_oracle as oracle;

/// {1}, Sacrifice this: creatures you control gain hexproof and indestructible until end of turn. The set is the
/// creatures there are as this resolves, this one already sacrificed (CR 611.2c).
fn protect_team(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::creatures_you_control_gain(g, item, &[Keyword::Hexproof, Keyword::Indestructible]);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{1}"), this_gains_flying, "{1}: Spectacular Spider-Man gains flying until end of turn."),
        ability(cost("{1}").and(SACRIFICE_THIS), protect_team, "{1}, Sacrifice Spectacular Spider-Man: Creatures you control gain hexproof and indestructible until end of turn."),
    ],
    ..oracle::SPECTACULAR_SPIDER_MAN
};
