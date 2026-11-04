use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// Gain the landwalk named by this ability's text and deal 2 damage to its controller (CR 612.2, 608.2c).
fn landwalk_and_damage(g: &mut dyn Game, item: &StackItem, land: Subtype) -> Result<(), Suspend> {
    let word = g.words(item.object).subtype(land);
    let walk = Keyword::landwalk(word);
    instruction(g, |g| {
        if let Some(walk) = walk {
            effects::gain_until_end_of_turn(g, item.object, item.object, walk);
        }
        g.damage_together(&[(item.object, Target::Player(item.controller), 2)]);
    });
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        ability(cost("{G}{G}"), |g, item| landwalk_and_damage(g, item, Subtype::Forest), "{G}{G}: This creature gains forestwalk until end of turn and deals 2 damage to you."),
        ability(cost("{B}{B}"), |g, item| landwalk_and_damage(g, item, Subtype::Swamp), "{B}{B}: This creature gains swampwalk until end of turn and deals 2 damage to you."),
    ],
    ..oracle::WORMWOOD_TREEFOLK
};
