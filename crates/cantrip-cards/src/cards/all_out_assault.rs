use cantrip_api::*;
use cantrip_mechanics::statics::{have_deathtouch, plus_pt, your_creatures};
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// The intervening "if it's your main phase" (CR 603.4): a main phase of a turn of the ability's controller, checked as the
/// enchantment enters and again as the ability resolves; entering in any other step or phase, or on an opponent's turn, it
/// does not trigger at all, so the delayed ability is not made either.
fn your_main_phase(q: &dyn Query, asker: &Asker) -> bool {
    q.state().turn.active == asker.controller && q.state().turn.step().is_main()
}

/// "There is an additional combat phase after this phase followed by an additional main phase" (CR 500.8, 505.1): both are
/// inserted after the main phase under way, in the order printed. "When you next attack this turn, untap each creature you
/// control" is a delayed triggered ability (CR 603.7b) made as the ability resolves, so it sees the turn's own combat if it is
/// still ahead (cast in the first main phase) or only the additional one (cast in the second main phase); it ends with the turn
/// if you never attack.
fn extra_combat_and_main_then_untap_on_next_attack(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    g.add_phases(PhaseAnchor::Current, &[PhaseSpec::Whole(Phase::Combat), PhaseSpec::Whole(Phase::Main)]);
    g.end_sentence();
    let this = item.ability.card;
    g.delayed_trigger(DelayedWhen::NextThisTurn, item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Ability, your_creatures, have_deathtouch, "Creatures you control get +1/+1 and have deathtouch."),
        static_ability(Layer::PtModify, your_creatures, plus_pt::<1, 1>, "Creatures you control get +1/+1 and have deathtouch.").continues(),
    ],
    abilities: &[triggered(
        triggers::THIS_ENTERS,
        extra_combat_and_main_then_untap_on_next_attack,
        "When this enchantment enters, if it's your main phase, there is an additional combat phase after this phase followed by an additional main phase. When you next attack this turn, untap each creature you control.",
    )
    .condition(your_main_phase)],
    delayed: &[triggered(
        triggers::YOU_ATTACK,
        effects::untap_all_creatures_you_control,
        "When you next attack this turn, untap each creature you control.",
    )],
    ..oracle::ALL_OUT_ASSAULT
};
