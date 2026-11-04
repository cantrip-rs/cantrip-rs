use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_DIES;
use cantrip_mechanics::{effects, targets, tiered};
use cantrip_oracle as oracle;

/// "Until end of turn, target creature you control gains 'When this creature dies, return it to the battlefield tapped under its
/// owner's control' and has the chosen base power and toughness" (CR 611.2a, 613.4b): both for the rest of the turn, the first the
/// card's own delayed ability, the second a base-setting effect that replaces earlier ones while counters and other modifiers still
/// apply. With the target gone nothing happens (CR 608.2b).
fn becomes(g: &mut dyn Game, item: &StackItem, power: i16, toughness: i16) -> Result<(), Suspend> {
    let Some(creature) = targets::object(g, item, 0) else { return Ok(()) };
    g.one_instruction(&mut |g| {
        let this = g.card_id(&CARD);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
        g.add_effect(item.object, creature, EffectKind::SetPt(power, toughness), Duration::EndOfTurn);
        Ok(())
    })
}

fn galian_beast(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    becomes(g, item, 3, 2)
}

fn death_gigas(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    becomes(g, item, 5, 2)
}

fn hellmasker(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    becomes(g, item, 7, 2)
}

const MODES: &[ModeDef] = &[
    mode(galian_beast, "Galian Beast — {0} — 3/2.").cost(mana("{0}")).targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(death_gigas, "Death Gigas — {1} — 5/2.").cost(mana("{1}")).targets(&[targets::CREATURE_YOU_CONTROL]),
    mode(hellmasker, "Hellmasker — {3} — 7/2.").cost(mana("{3}")).targets(&[targets::CREATURE_YOU_CONTROL]),
];

static MODAL: ModalDef = tiered::modal(MODES);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[triggered(THIS_DIES, effects::return_it_tapped, "When this creature dies, return it to the battlefield tapped under its owner's control.")],
    ..oracle::VINCENTS_LIMIT_BREAK
};
