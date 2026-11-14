use cantrip_api::*;
use cantrip_mechanics::{effects, prevention};
use cantrip_oracle as oracle;

/// "Prevent all damage that black and/or red sources would deal to creatures you control this turn" (CR 615.1, 609.7): the
/// damage is dealt to a creature its controller controls, as the damage is dealt - so a creature that came under their control
/// later in the turn, or became a creature, is covered (the ruling) - by a source that is black, red or both (CR 105.2).
fn black_or_red_to_your_creatures(q: &dyn Query, asker: &Asker, _: ObjectId, d: &DamageEvent) -> bool {
    let Target::Object(creature) = d.target else { return false };
    q.chars(creature).is(CardType::Creature)
        && q.controller(creature) == asker.controller
        && q.chars(d.source).colors.intersects(ColorSet::of(Color::Black).with(Color::Red))
}

const BLACK_OR_RED: GrantedPreventionDef = GrantedPreventionDef {
    hint: "prevents all damage black and/or red sources would deal to creatures you control",
    damage: "damage from black and/or red sources",
    of_it: None,
    applies: black_or_red_to_your_creatures,
    then: None,
    gain_prevented: false,
    text: "Prevent all damage that black and/or red sources would deal to creatures you control this turn.",
};

/// "You and permanents you control gain hexproof until end of turn." The permanents are those you control as it resolves,
/// fixed then (CR 611.2c). "Prevent all damage ..." is a second sentence (CR 608.2c).
fn surge(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let permanents = g.permanents_of(item.controller);
    instruction(g, |g| {
        g.add_effect(item.object, item.object, EffectKind::PlayerHexproof, Duration::EndOfTurn);
        for permanent in permanents {
            effects::gain_until_end_of_turn(g, item.object, permanent, Keyword::Hexproof);
        }
    });
    g.end_sentence();
    prevention::this_turn(g, item, Target::None, 0, Shield::All);
    Ok(())
}

pub static CARD: CardDef = CardDef {
    spell: spell(&[], surge),
    granted_preventions: &[BLACK_OR_RED],
    ..oracle::SURGE_OF_SALVATION
};
