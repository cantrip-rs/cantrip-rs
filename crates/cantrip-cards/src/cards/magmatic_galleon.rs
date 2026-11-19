use cantrip_api::*;
use cantrip_mechanics::{crew, effects, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// "Whenever one or more creatures your opponents control are dealt excess noncombat damage" (CR 120.10, 603.2c): a creature
/// of an opponent of the Galleon's controller was dealt noncombat damage beyond what was lethal to it (CR 120.10: lethal damage
/// counting damage it already had, deathtouch not counting). Excess is the creature's own: the batch's total for it
/// (`recipient_excess`). Damage dealt together is one event, so the ability is `.once_per_batch()`: however many creatures,
/// however many sources, it triggers once for the batch (the card's ruling).
fn opponents_creatures_are_dealt_excess_noncombat_damage(q: &dyn Query, asker: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Object(id), kind, recipient_excess, .. }
            if !kind.is_combat()
                && *recipient_excess > 0
                && q.chars(*id).is(CardType::Creature)
                && q.state().opponents(asker.controller).any(|p| p == q.controller(*id)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            effects::damage_first_target::<5>,
            "When this Vehicle enters, it deals 5 damage to target creature an opponent controls.",
        )
        .targets(&[targets::CREATURE_AN_OPPONENT_CONTROLS]),
        triggered(
            when_asked(EventMask::DAMAGE, opponents_creatures_are_dealt_excess_noncombat_damage),
            tokens::create_a_treasure,
            "Whenever one or more creatures your opponents control are dealt excess noncombat damage, create a Treasure token.",
        )
        .once_per_batch(),
        crew::crew(2, "Crew 2 (Tap any number of creatures you control with total power 2 or more: This Vehicle becomes an artifact creature until end of turn.)"),
    ],
    ..oracle::MAGMATIC_GALLEON
};

pub static EXTRA: &[&CardDef] = &[&tokens::TREASURE];
