use cantrip_api::*;
use cantrip_mechanics::{effects, spree, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Untap all creatures you control. If it's your combat phase, there is an additional combat phase after this phase" (CR
/// 701.26b, 500.8): the second sentence reads the phase under way as the spell resolves, on the controller's own turn.
fn untap_and_extra_combat(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::untap_all_creatures_you_control(g, item)?;
    g.end_sentence();
    if g.state().turn.active == item.controller && g.state().turn.step().phase() == Phase::Combat {
        g.add_combat_phase();
    }
    Ok(())
}

/// "Choose target opponent. Whenever a creature you control deals combat damage to that player this turn, ..." (CR 603.7b,
/// 603.7c): the delayed ability is made about the chosen player, and an opponent that is no longer a legal target makes none
/// (CR 608.2b).
fn watch_the_opponent(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Some(player) = targets::player(g, item, 0) {
        let this = g.card_id(&CARD);
        g.delayed_trigger(DelayedWhen::ThisTurn, item.object, item.controller, AbilityRef::delayed(this, 0), Target::Player(player));
    }
    Ok(())
}

/// A creature the ability's controller controls deals combat damage to the player the ability was made about (its event
/// subject, CR 603.7c).
fn creature_you_control_deals_combat_damage_to_that_player(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let fired = triggers::creature_you_control_deals_combat_damage_to_player(q, me, event)?;
    (fired.subject == q.event_subject()).then_some(fired)
}

fn create_a_tapped_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_tapped_treasure(g, item.controller)
}

const MODES: &[ModeDef] = &[
    mode(
        untap_and_extra_combat,
        "+ {2}{R} \u{2014} Untap all creatures you control. If it's your combat phase, there is an additional combat phase after this phase.",
    )
    .cost(mana("{2}{R}")),
    mode(
        effects::creatures_you_control_get_and_gain::<1, 0, { Keywords::of(&[Keyword::FirstStrike]).0 }>,
        "+ {2} \u{2014} Creatures you control get +1/+0 and gain first strike until end of turn.",
    )
    .cost(mana("{2}")),
    mode(
        watch_the_opponent,
        "+ {R} \u{2014} Choose target opponent. Whenever a creature you control deals combat damage to that player this turn, create a tapped Treasure token.",
    )
    .targets(&[targets::OPPONENT])
    .cost(mana("{R}")),
];

static MODAL: ModalDef = spree::modal(MODES);

pub static CARD: CardDef = CardDef {
    spell: Some(SpellDef::MODAL),
    modal: Some(&MODAL),
    delayed: &[triggered(
        when_asked(EventMask::DAMAGE, creature_you_control_deals_combat_damage_to_that_player),
        create_a_tapped_treasure,
        "Whenever a creature you control deals combat damage to that player this turn, create a tapped Treasure token.",
    )],
    ..oracle::GREAT_TRAIN_HEIST
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
