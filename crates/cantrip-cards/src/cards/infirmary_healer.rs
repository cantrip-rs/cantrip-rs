use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Target player gains X life" (CR 119.1): the player the spell targeted, if still a legal target (CR 608.2b).
fn stream_of_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        g.gain_life(player, item.x as i32);
    }
    Ok(())
}

/// Infirmary Healer's prepare spell (CR 722): not a card, only ever a copy.
pub static STREAM_OF_LIFE: CardDef = CardDef {
    name: "Stream of Life",
    cost: mana("{X}{G}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::PLAYER], stream_of_life),
    text: "Target player gains X life.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    enters: Some(cantrip_mechanics::prepare::enters_prepared),
    prepare: Some(&STREAM_OF_LIFE),
    ..oracle::INFIRMARY_HEALER
};

pub static EXTRA: &[&CardDef] = &[&STREAM_OF_LIFE];
