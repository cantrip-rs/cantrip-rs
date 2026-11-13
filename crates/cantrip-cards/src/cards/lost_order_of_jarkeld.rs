use cantrip_api::*;
use cantrip_mechanics::choose;
use cantrip_oracle as oracle;

/// "As this creature enters, choose an opponent." (CR 614.12a).
fn choose_opponent(g: &mut dyn Game, entering: &mut Entering) -> Result<(), Suspend> {
    let opponents = g.state().opponents(entering.controller).collect();
    entering.chosen_player = choose::player(g, entering.controller, entering.object, opponents,
        "has their creatures counted for this creature's power and toughness", 0)?;
    Ok(())
}

/// "Lost Order of Jarkeld's power and toughness are each equal to 1 plus the number of creatures the chosen player controls." (CR 604.3).
fn power_toughness(q: &dyn LayerQuery, source: ObjectId, chars: &mut Chars) {
    let chosen = q.state().chosen_player(q.linked_to(source));
    let count = q.state().battlefield().iter()
        .filter(|&&id| chosen == Some(q.controller(id)) && q.chars(id).is(CardType::Creature))
        .count() as i32;
    let value = 1 + count;
    chars.power = Some(value);
    chars.toughness = Some(value);
}

pub static CARD: CardDef = CardDef {
    power: Some(1),
    toughness: Some(1),
    statics: &[characteristic_defining(Layer::PtDefine, power_toughness,
        "Lost Order of Jarkeld's power and toughness are each equal to 1 plus the number of creatures the chosen player controls.")],
    enters: Some(choose_opponent),
    ..oracle::LOST_ORDER_OF_JARKELD
};
