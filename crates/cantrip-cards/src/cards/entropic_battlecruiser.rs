use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ATTACKS;
use cantrip_mechanics::{effects, statics, station};
use cantrip_oracle as oracle;

const ONE_PLUS: &str = "1+ | Whenever an opponent discards a card, they lose 3 life.";
const EIGHT_PLUS: &str = "8+ | Flying, deathtouch";

/// "Whenever an opponent discards a card" (CR 603.2, 701.9): one trigger for each card discarded, so two discarded at once
/// trigger it twice; the discarding opponent is its subject, "they".
fn opponent_discards_a_card(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    let Event::Discarded { player, .. } = event else { return None };
    if q.state().opponents(me.controller).any(|o| o == *player) { Fired::about(Target::Player(*player)) } else { None }
}

/// "They lose 3 life": the opponent who discarded.
fn they_lose_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(player) = item.subject {
        g.lose_life(player, 3);
    }
    Ok(())
}

/// "Whenever this Spacecraft attacks, each opponent discards a card. Each opponent who can't loses 3 life." (CR 101.4, 701.9,
/// 119.3): the discards are made together, then each opponent who could not discard loses 3 life.
fn each_opponent_discards_or_loses_3_life(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let losses: Vec<(PlayerId, i32)> = effects::each_opponent_discards_a_card_and_those_who_cant(g, item)?.into_iter().map(|p| (p, 3)).collect();
    g.lose_life_together(&losses);
    Ok(())
}

/// The 8+ striation's abilities (CR 721.2a): "Flying, deathtouch" and the attack trigger, all in the one layer-6 effect.
fn have_flying_deathtouch_and_the_attack_trigger(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    chars.keywords = chars.keywords.with(Keyword::Flying).with(Keyword::Deathtouch);
    station::has::<1>(view, source, chars);
}

/// The 1+ and 8+ striations (CR 721.2a) hold triggered abilities: the card's `delayed` list, in order, and each is given to the
/// Spacecraft while it has enough charge counters.
pub static CARD: CardDef = CardDef {
    abilities: &[station::ability(
        "Station (Tap another creature you control: Put charge counters equal to its power on this Spacecraft. Station only as a sorcery. It's an artifact creature at 8+.)",
    )],
    statics: &[
        static_ability(Layer::Ability, station::charged::<1>, station::has::<0>, ONE_PLUS),
        static_ability(Layer::Type, station::charged::<8>, station::is_a_creature, EIGHT_PLUS),
        static_ability(Layer::Ability, statics::itself, have_flying_deathtouch_and_the_attack_trigger, EIGHT_PLUS).continues(),
        static_ability(Layer::PtSet, statics::itself, statics::base_pt::<3, 10>, EIGHT_PLUS).continues(),
    ],
    delayed: &[
        triggered(when_asked(EventMask::DISCARDED, opponent_discards_a_card), they_lose_3_life, "Whenever an opponent discards a card, they lose 3 life."),
        triggered(THIS_ATTACKS, each_opponent_discards_or_loses_3_life, "Whenever this Spacecraft attacks, each opponent discards a card. Each opponent who can't loses 3 life."),
    ],
    ..oracle::ENTROPIC_BATTLECRUISER
};
