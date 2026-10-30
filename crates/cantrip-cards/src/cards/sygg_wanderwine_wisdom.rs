use cantrip_api::*;
use cantrip_mechanics::{effects, targets, triggers};
use cantrip_oracle as oracle;

/// "Target creature gains 'Whenever this creature deals combat damage to a player or planeswalker, draw a card' until end
/// of turn." A continuous effect fixed to the creature as the ability resolves (CR 611.2c) that grants the triggered
/// ability in layer 6 (CR 613.1f), the creature's own, so its controller draws.
fn grant_draw(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let this = g.card_id(&CARD);
        g.add_effect(item.object, creature, EffectKind::AddAbility(AbilityRef::delayed(this, 0)), Duration::EndOfTurn);
    }
    Ok(())
}

/// "Target creature you control gains protection from each color until your next turn" (CR 702.16a, 611.2a).
fn grant_protection(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(creature) = targets::object(g, item, 0) {
        let colors = Color::ALL.into_iter().fold(ColorSet::NONE, ColorSet::with);
        g.add_effect(item.object, creature, EffectKind::Protection(colors), Duration::UntilNextTurnOf(item.controller));
    }
    Ok(())
}

/// "At the beginning of your first main phase, you may pay {W}. If you do, transform Sygg."
fn pay_white(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{W}"))
}

/// "At the beginning of your first main phase, you may pay {U}. If you do, transform Sygg."
fn pay_blue(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_transform(g, item, mana("{U}"))
}

pub static BACK: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_TRANSFORMS, grant_protection, "Whenever this creature transforms into Sygg, Wanderbrine Shield, target creature you control gains protection from each color until your next turn.")
            .targets(&[targets::CREATURE_YOU_CONTROL]),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_blue, "At the beginning of your first main phase, you may pay {U}. If you do, transform Sygg."),
    ],
    ..oracle::SYGG_WANDERBRINE_SHIELD
};

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS_OR_TRANSFORMS, grant_draw, "Whenever this creature enters or transforms into Sygg, Wanderwine Wisdom, target creature gains \"Whenever this creature deals combat damage to a player or planeswalker, draw a card\" until end of turn.")
            .targets(&[targets::CREATURE]),
        triggered(triggers::YOUR_FIRST_MAIN_PHASE, pay_white, "At the beginning of your first main phase, you may pay {W}. If you do, transform Sygg."),
    ],
    delayed: &[triggered(triggers::DEALS_COMBAT_DAMAGE_TO_PLAYER_OR_PLANESWALKER, |g, item| effects::draw(g, item, 1), "Whenever this creature deals combat damage to a player or planeswalker, draw a card.")],
    back: Some(&BACK),
    ..oracle::SYGG_WANDERWINE_WISDOM
};
