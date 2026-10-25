use cantrip_api::*;
use cantrip_mechanics::targets;
use cantrip_mechanics::tokens::TREASURE;
use cantrip_oracle as oracle;

const ASK: Question = Question::new("CreateTreasure", "create a Treasure token?", "If you do, target opponent creates a tapped Treasure token", "Create a Treasure", "No Treasure");

/// You may create a Treasure token. When you do (CR 603.12), target opponent creates a tapped Treasure token: a
/// creation that did not happen does not satisfy "when you do".
fn maybe_treasure(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if !cantrip_mechanics::effects::ask_may(g, item.controller, item.object, Purpose::Ask(&ASK))? {
        return Ok(());
    }
    let made = cantrip_mechanics::tokens::create(g, item.controller, &TREASURE, 1)?;
    if made.is_empty() {
        return Ok(());
    }
    let this = g.card_id(&CARD);
    g.reflexive_trigger(item.object, item.controller, AbilityRef::delayed(this, 0), Target::None);
    Ok(())
}

/// Target opponent creates a tapped Treasure token. An opponent who has left the game took the only target with them.
fn gift(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    if let Target::Player(opponent) = item.target(0) {
        cantrip_mechanics::tokens::create_tapped_treasure(g, opponent)?;
    }
    Ok(())
}

/// It deals damage to defending player equal to the number of artifacts they control.
fn raid(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    // Its defending player (CR 508.5), whoever controls the planeswalker it attacked now.
    let Some(defending) = g.defending_player(item.object) else { return Ok(()) };
    let artifacts = g.permanents_of(defending).into_iter().filter(|&id| g.chars(id).is(CardType::Artifact)).count();
    if artifacts > 0 {
        g.damage(item.object, Target::Player(defending), artifacts as u16);
    }
    Ok(())
}

pub static CARD: CardDef = CardDef {
    abilities: &[
        triggered(cantrip_mechanics::triggers::YOUR_UPKEEP, maybe_treasure, "At the beginning of your upkeep, you may create a Treasure token. When you do, target opponent creates a tapped Treasure token."),
        triggered(cantrip_mechanics::triggers::THIS_ATTACKS_DEFENDER, raid, "Whenever this creature attacks, it deals damage to defending player equal to the number of artifacts they control."),
    ],
    delayed: &[delayed_trigger(gift, "When you do, target opponent creates a tapped Treasure token.")
        .targets(&[targets::OPPONENT])],
    ..oracle::GENEROUS_PLUNDERER
};

pub static EXTRA: &[&CardDef] = &[&TREASURE];
