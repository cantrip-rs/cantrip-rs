use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

/// "Vicious Verse deals 1 damage to target opponent": a target no longer legal is dealt nothing (CR 608.2b).
fn vicious_verse(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(opponent) = targets::player(g, item, 0) else { return Ok(()) };
    g.damage(item.object, Target::Player(opponent), 1);
    Ok(())
}

/// Stingerquill Voxmancer's prepare spell (CR 722): not a card, only ever a copy cast from exile (CR 722.3c).
pub static VICIOUS_VERSE: CardDef = CardDef {
    name: "Vicious Verse",
    cost: mana("{B/R}"),
    types: TypeSet::of(&[CardType::Sorcery]),
    spell: spell(&[targets::OPPONENT], vicious_verse),
    text: "Vicious Verse deals 1 damage to target opponent.",
    ..CardDef::BLANK
};

pub static CARD: CardDef = CardDef {
    abilities: &[cantrip_mechanics::prepare::AT_YOUR_UPKEEP_IF_NOT_PREPARED_BECOMES_PREPARED],
    prepare: Some(&VICIOUS_VERSE),
    ..oracle::STINGERQUILL_VOXMANCER
};

pub static EXTRA: &[&CardDef] = &[&VICIOUS_VERSE];
