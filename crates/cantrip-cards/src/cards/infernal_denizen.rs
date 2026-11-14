use cantrip_api::*;
use cantrip_mechanics::{choose, effects, select, targets, triggers};
use cantrip_oracle as oracle;

const GAIN: Question = Question::new("GainControl", "gain control of a creature?", "Choose a creature the ability's controller controls", "Gain control", "Don't gain control");

/// Sacrifice two Swamps, or as many as possible (CR 101.3, 701.21a); if fewer were sacrificed, tap this
/// creature and let an opponent choose whether and which creature to control (CR 608.2d, 611.2b).
fn upkeep(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let rule = select::land_cards(select::as_many_as_can(2)).with(select::subtype_card(Subtype::Swamp));
    let permanents = g.permanents_of(you);
    let lands = choose::select(g, you, item.object, Purpose::Sacrifice, permanents, rule)?;
    let sacrificed = g.sacrifice(you, &lands).count();
    g.end_sentence();
    if sacrificed < 2 {
        g.tap(item.object);
        let opponents = g.state().opponents(you).collect();
        let Some(opponent) = choose::player(g, you, item.object, opponents, "may gain control of a creature you control", 0)? else { return Ok(()) };
        let creatures = g.permanents_of(you);
        if creatures.iter().any(|&id| g.chars(id).is(CardType::Creature))
            && effects::ask_may(g, opponent, item.object, Purpose::Ask(&GAIN))?
        {
            let chosen = choose::select(g, opponent, item.object, Purpose::Choose, creatures, select::creature_permanents(select::as_many_as_can(1)))?;
            if let Some(&creature) = chosen.first() {
                g.gain_control(item.object, creature, opponent, Duration::WhileSourceOnBattlefield);
            }
        }
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(triggers::YOUR_UPKEEP, upkeep, "At the beginning of your upkeep, sacrifice two Swamps. If you can't, tap this creature, and an opponent may gain control of a creature you control of their choice for as long as this creature remains on the battlefield."),
        ability(TAP, effects::gain_control_of_target_while_source_on_battlefield, "{T}: Gain control of target creature for as long as this creature remains on the battlefield.").targets(&[targets::CREATURE]),
    ],
    ..oracle::INFERNAL_DENIZEN
};
