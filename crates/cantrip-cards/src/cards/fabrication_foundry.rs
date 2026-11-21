use cantrip_api::*;
use cantrip_mechanics::{effects, targets::{self, filters}};
use cantrip_oracle as oracle;

/// "Target artifact card with mana value X or less from your graveyard" (CR 107.3a, 202.3): X is the one announced for this
/// activation, judged again on resolution (CR 608.2b); before it is announced any artifact card is a possible target.
fn artifact_card_in_your_graveyard_with_mana_value_x_or_less(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    targets::card_in_your_graveyard(q, asker, t, &[CardType::Artifact])
        && matches!(t, Target::Object(id) if q.announced_x().is_none_or(|x| q.mana_value(id) <= x))
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        mana_ability(
            TAP,
            add(Mana::W).only_for(ManaRestriction::ARTIFACT_SPELLS_OR_ABILITIES),
            "{T}: Add {W}. Spend this mana only to cast an artifact spell or activate an ability of an artifact source.",
        ),
        ability(
            cost("{2}{W}, {T}").and(measured(CostTake::ExilePermanents, CostMeasure::TotalManaValue, filters::another_artifact)),
            effects::return_first_target_to_battlefield,
            "{2}{W}, {T}, Exile one or more other artifacts you control with total mana value X: Return target artifact card with mana value X or less from your graveyard to the battlefield. Activate only as a sorcery.",
        )
        .sorcery_speed()
        .targets(&[target(artifact_card_in_your_graveyard_with_mana_value_x_or_less)])
        .targets_after_x(),
    ],
    ..oracle::FABRICATION_FOUNDRY
};
