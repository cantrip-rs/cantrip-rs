use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_oracle as oracle;

/// "Whenever a white creature you control attacks" (CR 508.1m, 603.2c): each attacker, as it is declared, that is a white
/// creature under this permanent's controller's control. Declaring attackers names each in its own event, so the ability
/// triggers once for each. White is the color as this card's text reads it (CR 612), and Linden, who is white, is one.
fn a_white_creature_you_control_attacks(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. }
            if q.controller(*attacker) == me.controller
                && q.chars(*attacker).is(CardType::Creature)
                && q.chars(*attacker).colors.has(q.words(me.source).color(Color::White)) =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, a_white_creature_you_control_attacks),
        effects::gain_one_life,
        "Whenever a white creature you control attacks, you gain 1 life.",
    )],
    ..oracle::LINDEN_THE_STEADFAST_QUEEN
};
