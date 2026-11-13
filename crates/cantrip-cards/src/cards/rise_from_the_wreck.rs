use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_oracle as oracle;

fn creature_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Creature])
}

/// "Target [subtype] card" from your graveyard: a card there with the subtype, read as a word of this spell's text
/// (CR 205.3g, 612.1).
fn card_with_subtype(q: &dyn Query, asker: &Asker, t: Target, subtype: Subtype) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[])
        && matches!(t, Target::Object(id) if q.chars(id).has_subtype(q.words(asker.source).subtype(subtype)))
}

fn mount_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_with_subtype(q, asker, t, Subtype::Mount)
}

fn vehicle_card(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    card_with_subtype(q, asker, t, Subtype::Vehicle)
}

/// "Target creature card with no abilities" from your graveyard (CR 113.1): `targets::has_no_abilities`, judged there.
fn creature_card_with_no_abilities(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    creature_card(q, asker, t) && matches!(t, Target::Object(id) if targets::has_no_abilities(q, id))
}

/// "Return up to one target creature card, up to one target Mount card, up to one target Vehicle card, and up to one target
/// creature card with no abilities from your graveyard to your hand": each "target" is chosen on its own, so one card may be
/// chosen by several of them (CR 115.3), and it is returned once.
fn return_them(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let mut cards = Vec::new();
    for card in targets::legal_objects(g, item) {
        if !cards.contains(&card) {
            cards.push(card);
        }
    }
    g.return_to_hand(&cards);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(
        &[up_to_one(creature_card), up_to_one(mount_card), up_to_one(vehicle_card), up_to_one(creature_card_with_no_abilities)],
        return_them,
    ),
    ..oracle::RISE_FROM_THE_WRECK
};
