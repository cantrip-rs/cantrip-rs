use cantrip_api::*;
use cantrip_mechanics::{effects, triggers};
use cantrip_oracle as oracle;

/// Every player skips their untap step (CR 614.10).
fn skip_untap(_: &dyn Query, _: &Asker, _: PlayerId) -> bool {
    true
}

/// The ability’s controller sacrifices this enchantment unless they pay {U} (CR 118.12, 701.21a).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    effects::sacrifice_this_unless_pay(g, item, cost("{U}"))
}

pub static CARD: CardDef = CardDef {
    abilities: &[triggered(triggers::YOUR_UPKEEP, upkeep,
        "At the beginning of your upkeep, sacrifice this enchantment unless you pay {U}.")],
    skip_steps: &[SkipStep { applies: skip_untap, step: Step::Untap }],
    ..oracle::STASIS
};
