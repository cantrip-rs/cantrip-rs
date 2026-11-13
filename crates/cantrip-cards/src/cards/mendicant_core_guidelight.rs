use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::{copy, statics, triggers};
use cantrip_oracle as oracle;

const COPY_Q: Question = Question::new("CopySpell", "pay {cost} to copy that artifact spell?", "", "Pay {cost}", "Don't pay");

/// "You may pay {1}. If you do, copy it" (CR 118.12, 608.2d, 707.10): "it" is the artifact spell that was cast, and the copy
/// becomes a token as it resolves (CR 707.10f).
fn pay_to_copy_it(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if g.may_pay(item.controller, item.object, item.object, cost("{1}"), &COPY_Q)? {
        copy::copy_the_cast_spell(g, item)?;
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    statics: &[
        characteristic_defining(Layer::PtDefine, statics::power_is_artifacts_you_control, "Mendicant Core's power is equal to the number of artifacts you control.").lost_pt(0, 3),
        max_speed("Max speed \u{2014} Whenever you cast an artifact spell, you may pay {1}. If you do, copy it. (The copy becomes a token.)"),
    ],
    delayed: &[triggered(triggers::ARTIFACT_SPELL_YOU_CAST, pay_to_copy_it, "Whenever you cast an artifact spell, you may pay {1}. If you do, copy it. (The copy becomes a token.)")],
    ..oracle::MENDICANT_CORE_GUIDELIGHT
};
