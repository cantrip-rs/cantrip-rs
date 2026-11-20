use cantrip_api::*;
use cantrip_mechanics::{effects, lands, look_for_card_type, targets, triggers};
use cantrip_oracle as oracle;

/// "Look at the top six cards of your library. You may reveal an artifact card from among them and put it into your hand.
/// Put the rest on the bottom of your library in a random order." With no artifact among them nobody is asked.
fn look_for_an_artifact(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    look_for_card_type(g, item.controller, item.object, 6, CardType::Artifact)
}

/// "Whenever you cast a Spacecraft spell" (CR 603.2): the trigger is about the spell, which is no permanent, so the
/// subtype is read through Rutherford's words (CR 612.2).
pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS,
            look_for_an_artifact,
            "When Rutherford enters, look at the top six cards of your library. You may reveal an artifact card from among them and put it into your hand. Put the rest on the bottom of your library in a random order.",
        ),
        triggered(
            when_asked(EventMask::SPELL_CAST, triggers::subtype_spell_you_cast::<{ lands::sub(Subtype::Spacecraft) }>),
            effects::plus_one_counter_on_target,
            "Whenever you cast a Spacecraft spell, put a +1/+1 counter on target creature.",
        )
        .targets(&[targets::CREATURE]),
    ],
    ..oracle::RUTHERFORD_UPGRADED_ENSIGN
};
