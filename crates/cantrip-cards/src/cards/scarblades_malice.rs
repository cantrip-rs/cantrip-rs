use cantrip_api::*;
use cantrip_mechanics::{delayed, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The 2/2 black and green Elf creature token (CR 111.1, 111.4).
pub static ELF_2_2_BLACK_GREEN: CardDef = CardDef {
    colors: Some(ColorSet(ColorSet::of(Color::Black).0 | ColorSet::of(Color::Green).0)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(2),
    toughness: Some(2),
    ..token!(Elf)
};

/// "Target creature you control gains deathtouch and lifelink until end of turn. When that creature dies this turn, create a
/// 2/2 black and green Elf creature token." Both keywords come from one layer 6 effect (CR 611.2a, 613.1f); the delayed
/// triggered ability (CR 603.7c) is fixed on that creature, so it triggers only when that very object dies (a creature that
/// left and came back is a new object, CR 400.7).
fn scarblades_malice(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.add_effect(item.object, creature, EffectKind::AddKeywords(Keywords::of(&[Keyword::Deathtouch, Keyword::Lifelink])), Duration::EndOfTurn);
    delayed::this_turn(g, item, creature);
    Ok(())
}

/// "Create a 2/2 black and green Elf creature token", under the spell's controller (CR 701.7a).
fn create_elf(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create(g, item.controller, &ELF_2_2_BLACK_GREEN, 1)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL], scarblades_malice),
    delayed: &[triggered(triggers::THIS_DIES, create_elf, "When that creature dies this turn, create a 2/2 black and green Elf creature token.")],
    ..oracle::SCARBLADES_MALICE
};

pub static EXTRA: &[&CardDef] = &[&ELF_2_2_BLACK_GREEN];
