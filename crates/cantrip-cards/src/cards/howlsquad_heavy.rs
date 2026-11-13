use cantrip_api::*;
use cantrip_mechanics::max_speed::max_speed;
use cantrip_mechanics::statics::{have_haste, other_of_subtype_you_control};
use cantrip_mechanics::triggers::BEGINNING_OF_YOUR_COMBAT;
use cantrip_mechanics::{lands, permanents_with_subtype_you_control, tokens};
use cantrip_oracle as oracle;

/// "That token attacks this combat if able" (CR 508.1d, 611.2c): the token this ability made, named as the lasting rule starts.
fn that_token(_: &dyn Query, _: &Asker, about: Target, id: ObjectId) -> bool {
    about.object() == Some(id)
}

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

/// "{T}: Add {R} for each Goblin you control" (CR 106.1, 608.2h): counted as the ability resolves, this creature among them while it
/// is one. "Goblin" is read through this creature's words, so a text change makes it count the new creature type (CR 612.2).
fn add_red_for_each_goblin(g: &mut dyn Game, asker: &Asker, _: Option<Color>) -> Result<(), Suspend> {
    let goblin = g.words(asker.source).subtype(Subtype::Goblin);
    let goblins = permanents_with_subtype_you_control(g, asker.controller, goblin);
    if goblins > 0 {
        g.add_mana(asker.controller, Mana::R, goblins as u32);
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
    delayed: &[mana_ability(TAP, add_by(add_red_for_each_goblin), "{T}: Add {R} for each Goblin you control.")],
    lasting_rules: &[LastingRule { text: "That token attacks this combat if able.", rule: LastingRuleKind::MustAttack, applies: that_token }],
    statics: &[
        static_ability(Layer::Ability, other_of_subtype_you_control::<{ lands::sub(Subtype::Goblin) }>, have_haste, "Other Goblins you control have haste."),
        max_speed("Max speed \u{2014} {T}: Add {R} for each Goblin you control."),
    ],
    ..oracle::HOWLSQUAD_HEAVY
};

