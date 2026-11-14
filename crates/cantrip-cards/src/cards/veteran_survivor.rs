use cantrip_api::*;
use cantrip_mechanics::{effects, statics, targets, triggers};
use cantrip_oracle as oracle;

/// "As long as there are three or more cards exiled with this creature" (CR 607.2a): the cards in exile whose link points back
/// at it, so a Survivor that left and returned (a new object, CR 400.7) has none.
fn itself_with_three_exiled(q: &dyn LayerQuery, source: ObjectId, affected: ObjectId, _: &Chars) -> bool {
    source == affected && q.exiled_with(q.linked_to(source)).len() >= 3
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(
        triggers::YOUR_SECOND_MAIN_PHASE,
        effects::exile_first_target_with_this,
        "Survival \u{2014} At the beginning of your second main phase, if this creature is tapped, exile up to one target card from a graveyard.",
    )
    .condition(triggers::this_is_tapped)
    .targets(&[targets::CARD_IN_A_GRAVEYARD.optional()])],
    statics: &[
        static_ability(Layer::PtModify, itself_with_three_exiled, statics::plus_pt::<3, 3>, "As long as there are three or more cards exiled with this creature, it gets +3/+3 and has hexproof."),
        static_ability(Layer::Ability, itself_with_three_exiled, statics::have_hexproof, "As long as there are three or more cards exiled with this creature, it gets +3/+3 and has hexproof."),
    ],
    ..oracle::VETERAN_SURVIVOR
};
