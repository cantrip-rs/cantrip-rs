use cantrip_api::*;
use cantrip_mechanics::triggers::THIS_ENTERS;
use cantrip_mechanics::{effects, mana, targets, tokens, triggers};
use cantrip_oracle as oracle;

/// The colorless artifact token named Meteorite (CR 111.4, 111.11) with "When this token enters, it deals 2 damage to any
/// target." and "{T}: Add one mana of any color."
pub static METEORITE: CardDef = CardDef {
    name: "Meteorite",
    has_cost: false,
    types: TypeSet::of(&[CardType::Artifact]),
    abilities: &[
        triggered(THIS_ENTERS, effects::damage_first_target::<2>, "When this token enters, it deals 2 damage to any target.").targets(&[targets::ANY]),
        mana_ability(TAP, add_any_color(1), "{T}: Add one mana of any color."),
    ],
    text: "When this token enters, it deals 2 damage to any target.\n{T}: Add one mana of any color.",
    ..CardDef::BLANK
};

/// Creates the Meteorite tapped (CR 111.1, 110.5b).
fn create_a_tapped_meteorite(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    tokens::create_by_rule(g, item.controller, &METEORITE, 1, true)?;
    Ok(())
}

/// "Whenever you tap an artifact token for mana" (CR 106.12, 605.1b): an artifact that is a token, tapped for mana by the
/// ability's controller, handed to `mana::that_land_produced` for the mana it produced (CR 106.12a). Only Roxanne prints it.
fn you_tap_an_artifact_token_for_mana(q: &dyn Query, source: ObjectId, event: &Event) -> Option<Fired> {
    match *event {
        Event::TappedForMana { permanent, player, produced } if player == q.controller(source) && q.obj(permanent).token && q.chars(permanent).is(CardType::Artifact) => {
            mana::that_land_produced(permanent, player, produced)
        }
        _ => None,
    }
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(
            triggers::THIS_ENTERS_OR_ATTACKS,
            create_a_tapped_meteorite,
            "Whenever Roxanne enters or attacks, create a tapped colorless artifact token named Meteorite with \"When this token enters, it deals 2 damage to any target\" and \"{T}: Add one mana of any color.\"",
        ),
        triggered(
            when(EventMask::TAPPED, you_tap_an_artifact_token_for_mana),
            mana::add_one_mana_of_any_type_that_land_produced,
            "Whenever you tap an artifact token for mana, add one mana of any type that artifact token produced.",
        )
        .mana_ability(TriggeredMana::OneOfProduced),
    ],
    ..oracle::ROXANNE_STARFALL_SAVANT
};

pub static EXTRA: &[&CardDef] = &[&METEORITE];
