use cantrip_api::*;
use cantrip_mechanics::{effects, spree};
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER;
use cantrip_oracle as oracle;

/// Mode 1: "Target creature gains trample until end of turn."
fn trample(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        cantrip_mechanics::trample_until_end_of_turn(g, item.object, creature);
    }
    Ok(())
}

/// Mode 2: "Until end of turn, target creature gains 'Whenever this creature deals combat damage to a player, draw two
/// cards.'" A continuous effect fixed to the creature as the spell resolves (CR 611.2c) that grants the triggered
/// ability in layer 6 (CR 613.1f), the creature's own, so its controller draws.
fn grant_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let this = g.card_id(&CARD);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    }
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(effects::two_plus_one_counters_on_target, "+ {2} \u{2014} Put two +1/+1 counters on target creature.")
        .targets(&[targets::CREATURE])
        .cost(mana("{2}")),
    mode(trample, "+ {1} \u{2014} Target creature gains trample until end of turn.")
        .targets(&[targets::CREATURE])
        .cost(mana("{1}")),
    mode(grant_draw, "+ {1} \u{2014} Until end of turn, target creature gains \"Whenever this creature deals combat damage to a player, draw two cards.\"")
        .targets(&[targets::CREATURE])
        .cost(mana("{1}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[triggered(DEALS_COMBAT_DAMAGE_TO_PLAYER, |g, item| effects::draw(g, item, 2), "Whenever this creature deals combat damage to a player, draw two cards.")],
    ..oracle::TRASH_THE_TOWN
};
