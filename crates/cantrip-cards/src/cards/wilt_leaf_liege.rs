use cantrip_api::*;
use cantrip_mechanics::statics;
use cantrip_oracle as oracle;

/// "If a spell or ability an opponent controls causes you to discard this card, put it onto the battlefield instead of putting
/// it into your graveyard" (CR 614.1a, 701.9a): the discard of this card, from its owner's hand, that a spell or ability
/// an opponent of its owner controls causes (`Query::effect_controller`) - a replacement effect's discard being caused by that
/// effect's controller (CR 609.1: an opponent's Chains of Mephistopheles replacing a draw of its owner's own spell). Its own
/// discard cost, a spell or ability its owner controls, or the turn's cleanup leave it to go to the graveyard.
fn opponent_causes_discard(q: &dyn Query, asker: &Asker, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::ZoneChange(z)
        if z.object == asker.source
            && z.reason == ZoneReason::Discard
            && z.from.is(ZoneKind::Hand)
            && z.to.is(ZoneKind::Graveyard)
            && q.effect_controller().is_some_and(|by| q.state().opponents(q.obj(asker.source).owner).any(|opponent| opponent == by)))
}

/// "Other [color] creatures you control" (CR 613.1f): `C` is a `Color` as `u8`, read through the words of this card's text (CR 612.2).
fn other_creatures_of_color<const C: u8>(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, so_far: &Chars) -> bool {
    statics::other_creatures_you_control(q, source, affected, so_far) && statics::creatures_of_color::<C>(q, source, affected, so_far)
}

fn put_onto_the_battlefield(q: &dyn Query, _: &Asker, proposal: &mut Proposal) {
    if let Proposal::ZoneChange(z) = proposal {
        let owner = q.obj(z.object).owner;
        z.to = Zone::BATTLEFIELD;
        z.controller = owner;
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(
            Layer::PtModify,
            other_creatures_of_color::<{ Color::Green as u8 }>,
            statics::plus_pt::<1, 1>,
            "Other green creatures you control get +1/+1.",
        ),
        static_ability(
            Layer::PtModify,
            other_creatures_of_color::<{ Color::White as u8 }>,
            statics::plus_pt::<1, 1>,
            "Other white creatures you control get +1/+1.",
        ),
    ],
    replacements: &[replacement(
        ReplacementKind::ZoneChange,
        opponent_causes_discard,
        put_onto_the_battlefield,
        "put onto the battlefield instead of into the graveyard",
    )
    .own_everywhere()],
    ..oracle::WILT_LEAF_LIEGE
};
