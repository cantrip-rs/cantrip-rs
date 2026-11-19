use cantrip_api::*;
use cantrip_mechanics::{effects, replacements, targets};
use cantrip_oracle as oracle;

/// "You may cast Dinosaur creature spells from among cards you own exiled with this creature": its controller ("you", CR 109.5),
/// a card they own (CR 108.3) that this creature exiled (CR 607.2a), cast as a Dinosaur creature spell (CR 205.3m, read through
/// the words of this creature's text, CR 612). A card exiled by another Paleontologist, or one the controller does not own, is
/// not among them; neither is one this creature exiled before it left and came back (CR 400.7).
fn dinosaur_creature_you_own(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    let dinosaur = q.words(asker.source).subtype(Subtype::Dinosaur);
    let chars = q.chars(card);
    player == asker.controller
        && q.obj(card).owner == player
        && q.exiled_with(q.linked_to(asker)).contains(&card)
        && chars.is(CardType::Creature)
        && chars.has_subtype(dinosaur)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
        ability(cost("{2}"), effects::exile_first_target_with_this, "{2}: Exile target card from a graveyard.").targets(&[targets::CARD_IN_A_GRAVEYARD]),
    ],
    exile_casts: &[ExileCast {
        text: "Cast as a Dinosaur creature spell",
        from: ZoneKind::Exile,
        allows: dinosaur_creature_you_own,
        additional: Cost::NONE,
        any_type: false,
        any_color: false,
        lands: false,
        life_for_mana_value: false,
        instead: None,
        spell_replacement: Some(0),
    }],
    // "If you cast a spell this way, that creature enters with a finality counter on it": a replacement effect about the
    // spell cast by the permission (`ExileCast::spell_replacement`), made as it is cast and so still applying if the
    // Paleontologist has left before it resolves (its ruling). A Dinosaur cast from hand or by another permission has none.
    granted_replacements: &[replacements::SPELL_ENTERS_WITH_FINALITY
        .text("If you cast a spell this way, that creature enters with a finality counter on it.")],
    ..oracle::INTREPID_PALEONTOLOGIST
};
