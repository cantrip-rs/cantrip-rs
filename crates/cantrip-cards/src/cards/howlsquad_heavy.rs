use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::statics::{have_haste, other_of_subtype_you_control};
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{lands, lasting_rules, mana, tokens};
use cantrip_oracle as oracle;

/// "Create a 1/1 red Goblin creature token. That token attacks this combat if able." (CR 111.1, 508.1d): the requirement is an
/// effect of this ability, lasting until the end of this combat, on the token it made - not on a token another ability made.
fn goblin_that_attacks(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let made = tokens::create(g, item.controller, &tokens::GOBLIN, 1)?;
    g.end_sentence();
    for token in made {
        g.add_lasting_rule(item, 0, Target::Object(token), Duration::EndOfCombat);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    start_your_engines: true,
    abilities: &[triggered(
        BEGINNING_OF_YOUR_COMBAT,
        goblin_that_attacks,
        "At the beginning of combat on your turn, create a 1/1 red Goblin creature token. That token attacks this combat if able.",
    )],
    delayed: &[mana_ability(TAP, add_by(mana::add_for_each_subtype_you_control::<{ Mana::R as u8 }, { lands::sub(Subtype::Goblin) }>), "{T}: Add {R} for each Goblin you control.")],
    // "That token attacks this combat if able" (CR 508.1d, 611.2c): the token this ability made, named as the lasting rule starts.
    lasting_rules: &[LastingRule {
        text: "That token attacks this combat if able.",
        rule: LastingRuleKind::MustAttack,
        applies: lasting_rules::the_object_named,
    }],
    statics: &[
        static_ability(Layer::Ability, other_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>, have_haste, "Other Goblins you control have haste."),
        max_speed("Max speed \u{2014} {T}: Add {R} for each Goblin you control."),
    ],
    ..oracle::HOWLSQUAD_HEAVY
};

