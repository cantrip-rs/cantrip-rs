use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::triggers;
use cantrip_oracle as oracle;

/// A blue Illusion creature token: the generic definition, its size X/X given as it is made (`illusion_for_owner`).
pub static ILLUSION: CardDef = CardDef {
    colors: Some(ColorSet::of(Color::Blue)),
    types: TypeSet::of(&[CardType::Creature]),
    power: Some(0),
    toughness: Some(0),
    ..token!(Illusion)
};

/// "Nonland, nontoken permanent you don't control with mana value 4 or less."
fn target_filter(q: &dyn Query, asker: &Asker, t: Target) -> bool {
    (targets::NONLAND_PERMANENT_AN_OPPONENT_CONTROLS.filter)(q, asker, t)
        && matches!(t, Target::Object(id) if !q.obj(id).token && q.mana_value(id) <= 4)
}

/// "The exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled
/// card."
fn illusion_for_owner(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    cantrip_mechanics::effects::exiled_cards_owner_creates_x_x(g, item, &ILLUSION)
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::THIS_ENTERS, cantrip_mechanics::effects::exile_first_target_with_this, "When this creature enters, exile up to one target nonland, nontoken permanent you don't control with mana value 4 or less.")
            .targets(&[up_to_one(target_filter)]),
        triggered(triggers::THIS_LEAVES, illusion_for_owner, "When this creature leaves the battlefield, the exiled card's owner creates an X/X blue Illusion creature token, where X is the mana value of the exiled card."),
    ],
    ..oracle::SKYCLAVE_APPARITION
};

pub static EXTRA: &[&CardDef] = &[&ILLUSION];
