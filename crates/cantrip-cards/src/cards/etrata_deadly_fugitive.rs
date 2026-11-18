use cantrip_api::*;
use cantrip_mechanics::cloak::cloak_top_of_library;
use cantrip_mechanics::triggers::subtype_you_control_deal_combat_damage_to_player;
use cantrip_mechanics::{effects, lands, statics};
use cantrip_oracle as oracle;

/// "Face-down creatures you control" (CR 613.1f, 708.2): the face-down permanents under the source's controller's control that
/// are creatures as the earlier layers made them (a face-down permanent is a 2/2 creature).
fn face_down_creatures_you_control(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    q.state().obj(affected).face_down && statics::your_creatures(q, source, affected, so_far)
}

/// "{2}{U}{B}: Turn this creature face up. If you can't, exile it, then you may cast the exiled card without paying its mana
/// cost." (CR 701.40b, 701.58, 708.2, 608.2g, 118.9): the creature the ability was activated from, as it is when the ability resolves,
/// is turned up by its controller. A card that stays face down (a manifested or cloaked instant or sorcery card, which is only
/// revealed, CR 701.40g, 701.58g, or a creature that can't be turned face up), or a permanent that is already face up, cannot
/// be turned up, so it is exiled and then its controller may cast the exiled card for free, whatever type it is. Nothing happens to
/// a creature that has left the battlefield (CR 400.7).
fn turn_this_face_up_or_exile_and_may_cast_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !g.is_on_battlefield(item.object) {
        return Ok(());
    }
    let was_face_down = g.obj(item.object).face_down;
    g.turn_face_up(item.object, item.controller);
    if was_face_down && !g.obj(item.object).face_down {
        return Ok(());
    }
    let card = g.exile(item.object);
    effects::may_cast_without_paying(g, item.controller, card)?;
    Ok(())
}

/// "Whenever an Assassin you control deals combat damage to an opponent" (CR 510.2, 603.2): each Assassin's damage to each
/// opponent, with that opponent the trigger's subject.
fn assassin_you_control_deals_combat_damage_to_an_opponent(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::Damage { target: Target::Player(p), .. } if q.state().opponents(me.controller).any(|o| o == *p) => {
            subtype_you_control_deal_combat_damage_to_player::<{ lands::sub(Subtype::Assassin) }>(q, me, event)
        }
        _ => None,
    }
}

/// "Cloak the top card of that player's library." (CR 701.58a): the ability's controller cloaks it.
fn cloak_the_top_card_of_that_players_library(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Player(player) = item.subject else { return Ok(()) };
    cloak_top_of_library(g, item.controller, player)?;
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        when_asked(EventMask::DAMAGE, assassin_you_control_deals_combat_damage_to_an_opponent),
        cloak_the_top_card_of_that_players_library,
        "Whenever an Assassin you control deals combat damage to an opponent, cloak the top card of that player's library.",
    )],
    // The ability face-down creatures are given: the creature is its source (CR 113.7).
    delayed: &[ability(
        cost("{2}{U}{B}"),
        turn_this_face_up_or_exile_and_may_cast_it,
        "{2}{U}{B}: Turn this creature face up. If you can't, exile it, then you may cast the exiled card without paying its mana cost.",
    )],
    statics: &[static_ability(
        Layer::Ability,
        face_down_creatures_you_control,
        statics::grant_delayed_ability,
        "Face-down creatures you control have \"{2}{U}{B}: Turn this creature face up. If you can't, exile it, then you may cast the exiled card without paying its mana cost.\"",
    )],
    ..oracle::ETRATA_DEADLY_FUGITIVE
};
