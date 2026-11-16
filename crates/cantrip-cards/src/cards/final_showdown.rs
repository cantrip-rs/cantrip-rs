use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, spree};
use cantrip_oracle as oracle;

/// "All creatures lose all abilities until end of turn." (CR 613.1f, 611.2c.) The creatures are the ones on the battlefield as the
/// mode resolves, a set fixed then: a creature that enters later keeps its abilities. Each loses all its abilities for the rest of
/// the turn (until cleanup, CR 514.2), a later effect that gives one an ability, as the second mode does, still applying
/// (CR 613.7). One instruction (CR 608.2c).
fn all_creatures_lose_all_abilities_until_end_of_turn(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.state().battlefield().iter().copied().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    instruction(g, |g| {
        for &creature in &creatures {
            g.add_effect(item.object, creature, EffectKind::LoseAllAbilities, Duration::EndOfTurn);
        }
    });
    Ok(())
}

/// "Choose a creature you control. It gains indestructible until end of turn." (CR 702.12b, 611.2a.) Not a target (no
/// hexproof or protection matters, nothing is chosen on casting): its controller chooses as the mode resolves (CR 608.2d), among
/// the creatures they control then, asked even when there is only one and nothing happens when they control none.
fn choose_a_creature_you_control_that_gains_indestructible(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let creatures: Vec<ObjectId> = g.permanents_of(item.controller).into_iter().filter(|&id| g.chars(id).is(CardType::Creature)).collect();
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, creatures, select::creature_permanents(select::exactly(1)))?;
    if let Some(&creature) = chosen.first() {
        effects::gain_indestructible_until_end_of_turn(g, item.object, creature);
    }
    Ok(())
}

const MODES: &[ModeDef] = &[
    mode(all_creatures_lose_all_abilities_until_end_of_turn, "+ {1} \u{2014} All creatures lose all abilities until end of turn.").cost(mana("{1}")),
    mode(choose_a_creature_you_control_that_gains_indestructible, "+ {1} \u{2014} Choose a creature you control. It gains indestructible until end of turn.")
        .cost(mana("{1}")),
    mode(effects::destroy_all_creatures, "+ {3}{W}{W} \u{2014} Destroy all creatures.").cost(mana("{3}{W}{W}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef { spell: Some(SpellDef::MODAL), modal: Some(&MODAL), ..oracle::FINAL_SHOWDOWN };

pub static EXTRA: &[&CardDef] = &[];
