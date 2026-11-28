use cantrip_api::select::SelectFn;
use cantrip_api::*;
use cantrip_mechanics::effects::ask_may;
use cantrip_mechanics::select::{self, up_to, Selection};
use cantrip_mechanics::{choose, history};
use cantrip_oracle as oracle;

/// The mana spent on the activation `item` (CR 601.2h by way of 602.2b), each part by type and from a snow source or not
/// (CR 107.4h): its cost is {X}, and what else an effect adds to it (a cost increase, CR 601.2f) is paid with the same mana.
/// The ability is activated only as a sorcery, with nothing on the stack (CR 307.1), so its last activation is the one
/// resolving; a copy of it (CR 707.10) had no cost paid, and nothing was spent on it (Ice Cauldron's ruling: what X was does
/// not matter, only the mana spent).
fn spent_on_x(q: &dyn Query, item: &StackItem) -> Vec<(Mana, u32, bool)> {
    let records = q.history().all();
    let copied = records.iter().any(|r| matches!(r.event, Event::AbilityCopied { copy, .. } if copy == item.serial));
    // The ability itself, not another of the Mask's activated while it waits (a mana ability A Realm Reborn gives it).
    let activated = records
        .iter()
        .rposition(|r| matches!(r.event, Event::AbilityActivated { source, ability, .. } if source == item.object && ability == item.ability))
        .filter(|_| !copied);
    activated.map_or_else(Vec::new, |end| history::mana_spent_activating(q, end, item.object))
}

/// "A creature card ... whose mana cost could be paid by some amount of, or all of, the mana you spent on {X}" (CR 202.1,
/// 601.2h): the amount and the types of that mana both count (the ruling: {U}{U} pays {U}{U}, {1}{U}, {2} or {W/U}{W/U}, not
/// {2}{U} or {G}), an {X} in the card's cost as 0 (CR 107.3g); a card with no mana cost has none to be paid (CR 118.6). Of
/// mana spent beyond X, where an effect increased the cost, no part is told apart as X's: the card's mana cost is paid by at
/// most X of the mana spent, in any combination of it (owner ruling 2026-11-27), so with no mana spent beyond X it is any of
/// it. `serial` names the activation resolving.
fn creature_payable_by_x_mana(q: &dyn Query, choice: &Choice, serial: u32) -> Verdict {
    let item = q.state().stack.iter().chain(&q.state().left_stack).find(|i| i.serial == serial);
    let spent = item.map(|item| (spent_on_x(q, item), item.x));
    Verdict::each(choice, |card| {
        let chars = q.chars(card);
        chars.is(CardType::Creature)
            && q.card(chars.card).has_cost
            && spent.as_ref().is_some_and(|(spent, x)| q.mana_could_pay(spent, *x, &q.mana_cost(card)))
    })
}

/// The choice of the activation `serial` resolving.
fn maskable(serial: u32) -> Selection {
    Selection::new(up_to(1))
        .with(select::part(&(creature_payable_by_x_mana as SelectFn), serial, ""))
        .called("creature card whose mana cost the mana spent on X could pay")
}

const CAST_FACE_DOWN: Question =
    Question::new("CastFaceDown", "cast it face down as a 2/2 creature spell?", "Chosen as Illusionary Mask's ability resolves", "Cast it face down", "Don't");

/// "{X}: You may choose a creature card in your hand whose mana cost could be paid by some amount of, or all of, the mana you
/// spent on {X}. If you do, you may cast that card face down as a 2/2 creature spell without paying its mana cost." The cast
/// is part of the resolution (CR 608.2g), turned face down before it goes on the stack (CR 708.4); the spell then carries the
/// effect of the sentence after, about the creature it becomes (`GrantedReplacementDef::becomes`), three replacement effects
/// of one instruction - one for each event it watches - that last for as long as that creature does, whatever becomes of the
/// Mask (the ruling).
fn mask(g: &mut dyn Game, item: &StackItem) -> Result<(), Suspend> {
    let you = item.controller;
    let hand = g.state().zone(Zone::hand(you)).to_vec();
    let chosen = choose::select(g, you, item.object, Purpose::Choose, hand, maskable(item.serial))?;
    let Some(&card) = chosen.first() else { return Ok(()) };
    if !g.could_cast_face_down_during_resolution(card) || !ask_may(g, you, item.object, Purpose::Ask(&CAST_FACE_DOWN))? {
        return Ok(());
    }
    let Some(spell) = g.cast_face_down_during_resolution(card)? else { return Ok(()) };
    instruction(g, |g| {
        for index in 0..3 {
            let kind = EffectKind::Replacement { card: item.ability.card, index, to: Target::None, into: Target::None, value: 0 };
            g.add_effect(item.object, spell, kind, Duration::Indefinite);
        }
    });
    Ok(())
}

/// "If the creature that spell becomes as it resolves has not been turned face up": it is face down, and has not been turned
/// up since it entered (CR 708.8) - a creature turned up and then face down again (Backslide) is no longer one it applies to.
fn still_face_down(q: &dyn Query, creature: ObjectId) -> bool {
    q.obj(creature).face_down
        && !q.history().all().iter().any(|r| matches!(r.event, Event::TurnedFaceUp { permanent, .. } if permanent == creature))
}

/// "... would ... deal damage, be dealt damage": damage it is the source of or is dealt (CR 120.1).
fn would_deal_or_be_dealt_damage(q: &dyn Query, _: &Asker, creature: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Damage(d) if d.source == creature || d.target == Target::Object(creature)) && still_face_down(q, creature)
}

/// "... would ... become tapped" (CR 701.26a).
fn would_become_tapped(q: &dyn Query, _: &Asker, creature: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::Tap(t) if t.permanent == creature) && still_face_down(q, creature)
}

/// "... would assign ... damage" (CR 510.1): its combat damage.
fn would_assign_damage(q: &dyn Query, _: &Asker, creature: ObjectId, proposal: &Proposal) -> bool {
    matches!(proposal, Proposal::CombatDamageAssignment(a) if a.creature == creature) && still_face_down(q, creature)
}

/// The event itself is left as it is: "... assigns or deals damage, is dealt damage, or becomes tapped".
fn unchanged(_: &dyn Query, _: &Asker, _: ObjectId, _: &mut Proposal) {}

/// "Instead it's turned face up and ...": turned up as the effect applies, before the event goes on (CR 614.6, 708.8), by the
/// effect's controller (CR 609.1). A turning up nothing allows does nothing, and the event goes on (CR 101.2).
fn turn_it_face_up(g: &mut dyn Game, asker: &Asker, creature: ObjectId, _: &Proposal, _: &Proposal) {
    g.turn_face_up(creature, asker.controller);
}

const TEXT: &str = "If the creature that spell becomes as it resolves has not been turned face up and would assign or deal damage, be dealt damage, or become tapped, instead it's turned face up and assigns or deals damage, is dealt damage, or becomes tapped.";

pub static CARD: CardDef = CardDef {
    abilities: &[ability(
        cost("{X}"),
        mask,
        "{X}: You may choose a creature card in your hand whose mana cost could be paid by some amount of, or all of, the mana you spent on {X}. If you do, you may cast that card face down as a 2/2 creature spell without paying its mana cost. If the creature that spell becomes as it resolves has not been turned face up and would assign or deal damage, be dealt damage, or become tapped, instead it's turned face up and assigns or deals damage, is dealt damage, or becomes tapped. Activate only as a sorcery.",
    )
    .sorcery_speed()],
    granted_replacements: &[
        granted_replacement(ReplacementKind::Damage, would_deal_or_be_dealt_damage, unchanged, "turn it face up")
            .then(turn_it_face_up)
            .of_the_permanent_it_becomes()
            .text(TEXT),
        granted_replacement(ReplacementKind::Tap, would_become_tapped, unchanged, "turn it face up")
            .then(turn_it_face_up)
            .of_the_permanent_it_becomes()
            .text(TEXT),
        granted_replacement(ReplacementKind::CombatDamageAssignment, would_assign_damage, unchanged, "turn it face up")
            .then(turn_it_face_up)
            .of_the_permanent_it_becomes()
            .text(TEXT),
    ],
    ..oracle::ILLUSIONARY_MASK
};
