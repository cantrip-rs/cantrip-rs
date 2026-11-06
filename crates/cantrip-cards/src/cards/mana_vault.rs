use cantrip_api::*;
use cantrip_mechanics::effects;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// You may pay {4}. If you do, untap this artifact (CR 608.2c, 701.26).
fn pay_to_untap(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::may_pay_to_untap_self(g, item, cost("{4}"))
}

pub static CARD: CardDef = CardDef {
    // "This artifact doesn't untap during your untap step."
    keywords: Keywords::of(&[Keyword::DoesNotUntap]),
    abilities: &[
        mana_ability(TAP, add_n(Mana::C, 3), "{T}: Add {C}{C}{C}."),
        triggered(triggers::YOUR_UPKEEP, pay_to_untap, "At the beginning of your upkeep, you may pay {4}. If you do, untap this artifact."),
        triggered(triggers::YOUR_DRAW_STEP, effects::source_deals_one_to_controller, "At the beginning of your draw step, if this artifact is tapped, it deals 1 damage to you.")
            .condition(triggers::this_is_tapped),
    ],
    ..oracle::MANA_VAULT
};
