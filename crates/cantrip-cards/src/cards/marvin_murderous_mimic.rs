use cantrip_api::*;
use cantrip_mechanics::statics::itself;
use cantrip_oracle as oracle;

/// "Marvin has all activated abilities of creatures you control that don't have the same name as this creature." A static
/// ability of Marvin about himself (CR 604.1, 613.1f, layer 6): every activated ability - mana abilities and activated keywords
/// among them, no triggered or static ability (the rulings), nor a special action (CR 116.2) - that each other creature its
/// controller controls has now, however it has it (printed, or given by an effect before this one), in battlefield order. A
/// creature with no name, a face-down one, has none to share (CR 708.2a). Each is Marvin's own as written, so he is its source and
/// "this creature" means him (the rulings); he shares his own name, so is no donor of his own. The same ability from two creatures is two abilities (CR 602.5c).
fn activated_abilities_of_creatures_you_control_with_other_names(view: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let s = view.state();
    let controller = s.obj(source).controller;
    let name = chars.name;
    for &id in s.battlefield() {
        if s.obj(id).controller != controller {
            continue;
        }
        let donor = view.chars(id);
        if !donor.is(CardType::Creature) || donor.name == name {
            continue;
        }
        for ability in donor.abilities() {
            if matches!(view.cards().ability(ability), AbilityDef::Activated(a) if !a.special_action) {
                chars.grant_ability_from(ability, id);
            }
        }
    }
}

pub static CARD: CardDef = CardDef {
    statics: &[static_ability(
        Layer::Ability,
        itself,
        activated_abilities_of_creatures_you_control_with_other_names,
        "This creature has all activated abilities of creatures you control that don't have the same name as this creature.",
    )],
    ..oracle::MARVIN_MURDEROUS_MIMIC
};
