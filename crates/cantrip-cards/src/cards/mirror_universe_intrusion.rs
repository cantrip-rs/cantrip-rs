use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// Copy one opposing creature, or each creature of the targeted opponent if kicked (CR 702.33g, 707.9a).
/// All of the kicked spell's tokens enter together (CR 608.2c, 603.6a); menace is copiable.
fn copies(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures = if item.kicked > 0 {
        targets::player(g, item, 1).into_iter().flat_map(|player| g.permanents_of(player))
            .filter(|&id| g.chars(id).is(CardType::Creature)).collect()
    } else {
        targets::object(g, item, 0).into_iter().collect::<Vec<_>>()
    };
    let copies: Vec<_> = creatures.into_iter()
        .map(|id| (g.copiable_values(id).with_keyword(Keyword::Menace), item.controller)).collect();
    g.create_token_copies_together(&copies)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    kicker: kicker(mana("{4}{U}{B}")),
    spell: spell(&[
        targets::CREATURE_AN_OPPONENT_CONTROLS.if_kicked(0, 0),
        targets::OPPONENT.between(0, 0).if_kicked(1, 1),
    ], copies),
    ..oracle::MIRROR_UNIVERSE_INTRUSION
};
