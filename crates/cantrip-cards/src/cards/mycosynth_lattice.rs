use cantrip_api::*;
use cantrip_mechanics::statics::{artifact_too, colorless};
use cantrip_oracle as oracle;

/// "All permanents" (CR 110.1): every permanent on the battlefield, Mycosynth Lattice itself among them. Spells and cards
/// elsewhere are no permanents, so they don't become artifacts (the ruling).
fn all_permanents(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, _: &Chars) -> bool {
    true
}

/// "All cards that aren't on the battlefield, spells, and permanents": every object in every zone (the ruling: "everything,
/// in every zone of the game").
fn everything(_: &dyn LayerQuery, _: ObjectId, _: ObjectId, _: &Chars) -> bool {
    true
}

/// "Players may spend mana as though it were mana of any color." (CR 609.4b): every player, for every payment - a spell, an
/// ability, any other mana cost - and colorless mana too (the ruling). It changes no cost, and no restriction on the mana
/// (the ruling: mana that may be spent only on artifact spells still may not pay for another spell).
fn any_payment(_: &dyn Query, _: &Asker, _: &Spending) -> bool {
    true
}

pub static CARD: CardDef = CardDef {
    statics: &[
        static_ability(Layer::Type, all_permanents, artifact_too, "All permanents are artifacts in addition to their other types."),
        static_ability(Layer::Color, everything, colorless, "All cards that aren't on the battlefield, spells, and permanents are colorless.")
            .everywhere(),
    ],
    spend_any_color: &[any_payment],
    ..oracle::MYCOSYNTH_LATTICE
};
