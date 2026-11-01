use cantrip_api::*;
use cantrip_mechanics::triggers::ward_sacrifice_nonland;
use cantrip_mechanics::{replacements, your_turn};
use cantrip_oracle as oracle;

/// "If a card you didn't control would be put into an opponent's graveyard from anywhere" (CR 614.1a, 400.3): a card going
/// to an opponent's graveyard, unless it was a permanent or a spell this permanent's controller controlled as it left (a
/// card in a hand, a library or a graveyard has no controller, CR 108.4, 108.4a), so their stolen creature dying goes to its
/// owner's graveyard.
fn card_you_didnt_control(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    let Proposal::ZoneChange(z) = proposal else { return false };
    replacements::card_to_an_opponents_graveyard(q, asker, proposal)
        && !(matches!(z.from.kind(), ZoneKind::Battlefield | ZoneKind::Stack) && q.controller(z.object) == asker.controller)
}

/// "During your turn, you may play cards exiled with Valgavoth": its controller ("you", CR 109.5) on their own turn, any card
/// the replacement exiled with this object (CR 607.2b), a land as a land play and a spell at the time its type allows (the
/// rulings).
fn allows(q: &dyn Query, asker: &Asker, card: ObjectId, player: PlayerId) -> bool {
    player == asker.controller && your_turn(q, asker) && q.exiled_with(q.linked_to(asker)).contains(&card)
}

pub static CARD: CardDef = CardDef {
    abilities: &[ward_sacrifice_nonland::<3>("Ward\u{2014}Sacrifice three nonland permanents.")],
    // "... exile it instead": exiled with Valgavoth, which its last ability refers to (CR 607.2b).
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        card_you_didnt_control,
        replacements::exile_instead_linked_apply,
        "exiles a card you didn't control that an opponent's graveyard would receive",
    )],
    // "If you cast a spell this way, pay life equal to its mana value rather than pay its mana cost" (CR 118.9, 119.4): no
    // other alternative cost, additional costs as usual (the rulings).
    exile_casts: &[ExileCast {
        text: "Play during your turn; a spell for life equal to its mana value",
        allows,
        additional: Cost::NONE,
        any_type: false,
        lands: true,
        life_for_mana_value: true,
        instead: None,
    }],
    ..oracle::VALGAVOTH_TERROR_EATER
};
