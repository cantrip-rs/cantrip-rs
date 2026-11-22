use cantrip_api::*;
use cantrip_mechanics::{fight, targets};
use cantrip_oracle as oracle;

/// "Target creature you control fights target creature an opponent controls. If the creature the opponent controls is dealt
/// excess damage this way, add that much {R}. Until end of turn, you don't lose unspent red mana as steps and phases end."
/// The fight is the shared verb, which does nothing unless both are creatures on the battlefield (CR 701.14b). Excess damage
/// (CR 120.10) is read from the damage event the fight recorded: what the creature you control dealt above the other's lethal
/// damage, judged after any prevention. The mana is added as it resolves, so it is in the pool before the keeping effect
/// begins; that effect is made even if the fight did nothing (CR 608.2b: only a target that is illegal on resolution is not
/// affected, and the keeping effect has none).
fn fight_add_red_for_the_excess_and_keep_red(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // The fight asks both targets again; the opponent's creature is the one whose excess damage counts.
    if let Some(theirs) = targets::object(g, item, 1) {
        let before = g.history().head() as usize;
        fight::targets_fight(g, item, 0, 1);
        let excess: u32 = g.history().all()[before..]
            .iter()
            .filter_map(|r| match r.event {
                Event::Damage { target, recipient_excess, .. } if target == Target::Object(theirs) => Some(recipient_excess),
                _ => None,
            })
            .sum();
        g.add_mana(item.controller, Mana::R, excess);
    }
    g.add_player_effect(item.controller, item.object, EffectKind::ManaKept(Mana::R), Duration::EndOfTurn);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[targets::CREATURE_YOU_CONTROL, targets::CREATURE_AN_OPPONENT_CONTROLS], fight_add_red_for_the_excess_and_keep_red),
    ..oracle::THE_LAST_AGNI_KAI
};
