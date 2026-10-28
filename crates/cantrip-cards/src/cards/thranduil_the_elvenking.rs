use cantrip_api::*;
use cantrip_mechanics::effects::draw_two_then_discard;
use cantrip_mechanics::statics::{grant_activated_abilities_of, itself};
use cantrip_oracle as oracle;

/// "Thranduil has all activated abilities of all Elf cards in your graveyard." A static ability of Thranduil about himself (CR 604.1,
/// 613.1f, layer 6): the activated abilities - mana abilities and activated keywords among them, no triggered or static ability - of
/// every card in its controller's graveyard that is an Elf there, as the graveyard has it now (CR 611.3a), in graveyard order. Elf
/// is a word of this card's text (CR 612). Each is Thranduil's own as written, so he is its source and "this creature" or its own
/// name means him (the ruling); an opponent's graveyard gives nothing.
fn graveyard_elf_abilities(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let s = view.state();
    let elf = view.words(source).subtype(Subtype::Elf);
    let cards: Vec<CardId> =
        s.zone(Zone::graveyard(s.obj(source).controller)).iter().map(|&card| view.chars(card)).filter(|card| card.has_subtype(elf)).map(|card| card.card).collect();
    grant_activated_abilities_of(view, &cards, chars);
}

/// "Whenever another legendary Elf you control enters" (CR 603.6a): a permanent other than this one that is legendary and an Elf as
/// it entered, under its controller's control; the type is read through the words of this card's text (CR 612).
fn another_legendary_elf_you_control_enters(q: &dyn Query, me: &Asker, event: &Event) -> Option<Fired> {
    match event {
        Event::ZoneChange { new, to, .. }
            if *new != me.source && to.is(ZoneKind::Battlefield) && q.controller(*new) == me.controller && {
                let entered = q.chars(*new);
                entered.supertypes.legendary && entered.has_subtype(q.words(me.source).subtype(Subtype::Elf))
            } =>
        {
            Fired::PLAIN
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(Layer::Ability, itself, graveyard_elf_abilities, "This creature has all activated abilities of all Elf cards in your graveyard.")],
    abilities: &[triggered(
        when_asked(EventMask::ZONE_CHANGE, another_legendary_elf_you_control_enters),
        draw_two_then_discard,
        "Whenever another legendary Elf you control enters, draw two cards, then discard a card.",
    )],
    ..oracle::THRANDUIL_THE_ELVENKING
};
