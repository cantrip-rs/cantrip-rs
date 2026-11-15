use cantrip_api::*;
use cantrip_mechanics::effects::this_gets_and_gains_menace;
use cantrip_oracle as oracle;

/// "Whenever this creature attacks while there are four or more card types among cards in your graveyard" (CR 207.2c, 508.1m,
/// 603.2): its own attack, declared while its controller has delirium. "While" is part of what triggers it, not an intervening
/// "if" (CR 603.4): the graveyard is looked at as the attack is declared and never again.
fn this_attacks_with_delirium(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Attacks { attacker, .. } if *attacker == me.source && cantrip_mechanics::delirium(q, me.controller) => Fired::PLAIN,
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::ATTACKS, this_attacks_with_delirium),
        this_gets_and_gains_menace::<2, 0>,
        "Delirium \u{2014} Whenever this creature attacks while there are four or more card types among cards in your graveyard, it gets +2/+0 and gains menace until end of turn.",
    )],
    ..oracle::HAND_THAT_FEEDS
};
