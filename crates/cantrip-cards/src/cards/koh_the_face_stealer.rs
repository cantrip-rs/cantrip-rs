use cantrip_api::*;
use cantrip_mechanics::triggers::{self, THIS_ENTERS};
use cantrip_mechanics::{choose, effects, select, statics, targets};
use cantrip_oracle as oracle;

/// "You may exile it." The card it became in the graveyard, if it is still there (CR 400.7e, 608.2b): a card that left before
/// this resolves is not exiled (the ruling). It is exiled with Koh (CR 607.2a), and the ability that chooses a card finds it there.
fn may_exile_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let Target::Object(creature) = item.subject else { return Ok(()) };
    let cards = g.found_all_after_move(creature);
    if cards.is_empty() || !effects::ask_may(g, item.controller, item.object, Purpose::Optional)? {
        return Ok(());
    }
    g.exile_with(&cards, g.linked(item), false);
    Ok(())
}

/// "Choose a creature card exiled with Koh." (CR 608.2d, 607.2d): not a target. A card chosen before is chosen no more (it is "the
/// last chosen card" that counts); with no creature card exiled with Koh - none at all, or only cards that are no creature card
/// in exile (an animated Mutavault) - nothing is chosen and the last choice stands.
fn choose_a_creature_card(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let by = g.linked(item);
    let among = g.exiled_with(by);
    let chosen = choose::select(g, item.controller, item.object, Purpose::Choose, among, select::creature_cards(select::as_many_as_can(1)))?;
    if let Some(&card) = chosen.first() {
        g.remember_chosen_object(by, card);
    }
    Ok(())
}

/// "Koh has all activated and triggered abilities of the last chosen card." (CR 613.1f, layer 6): each one the card prints, as
/// written, so Koh is its source and "this creature" in it means Koh. Only while that card is still the one exiled with Koh and
/// chosen: a card that left exile is a new object (CR 400.7), no longer chosen, and Koh has nothing of it. Each is lent with the
/// card as its donor (an instance of its own, CR 113.2c); as abilities this one effect gave Koh, a linked pair of the card is
/// linked on Koh to each other and not to Koh's own (CR 607.5), an activated and a triggered one alike (Boiling Rock Rioter).
/// A special action is no ability (CR 116.2).
fn abilities_of_the_last_chosen_card(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let by = view.linked_to(source);
    let Some(chosen) = view.state().chosen_object(by).filter(|id| view.exiled_with(by).contains(id)) else { return };
    let card = view.state().obj(chosen).card;
    statics::grant_activated_abilities_from(view, &[(chosen, card)], chars);
    for (index, ability) in view.cards().get(card).abilities.iter().enumerate() {
        if matches!(ability, AbilityDef::Triggered(_)) {
            chars.grant_ability_from(AbilityRef { card, index: index as u8 }, chosen);
        }
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(THIS_ENTERS, effects::exile_first_target_with_this, "When Koh enters, exile up to one other target creature.").targets(&[targets::OTHER_CREATURE.optional()]),
        triggered(when_asked(EventMask::ZONE_CHANGE, triggers::another_nontoken_creature_dies), may_exile_it, "Whenever another nontoken creature dies, you may exile it."),
        ability(life(1), choose_a_creature_card, "Pay 1 life: Choose a creature card exiled with Koh."),
    ],
    statics: &[static_ability(
        Layer::Ability,
        statics::itself,
        abilities_of_the_last_chosen_card,
        "Koh has all activated and triggered abilities of the last chosen card.",
    )],
    ..oracle::KOH_THE_FACE_STEALER
};
