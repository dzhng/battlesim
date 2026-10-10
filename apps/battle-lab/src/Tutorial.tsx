// The main menu's Tutorial page: how a skirmish is played, from the plan
// before the first shot to the last flag. The economy's numbers are read from
// the skirmish rules and the command keys from their bindings, so the
// guide says what the game does; the rest describes rules owned by the
// simulation (objectives, preparation, purchases) and must follow them. Its
// pictures are drawn from a real skirmish by `web/tutorial-shots.mjs`.
import type { ReactNode } from "react";
import skirmish from "@fixtures/skirmish.json";
import {
  CommandBindings,
  ShowOrdersBinding,
  type CommandBinding,
} from "@web/battle/input/commandBindings";

/** A command's key alone, where its binding's label also names a mouse
 *  alternative the guide words itself. */
const keyOf = (b: CommandBinding) => b.code.replace(/^Key/, "");

function Section({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section>
      <h3>{title}</h3>
      {children}
    </section>
  );
}

/** A picture of the moment the text describes: a still, or a gesture's
 *  silent loop. The caption says what it shows. */
function Shot({
  name,
  loop = false,
  children,
}: {
  name: string;
  loop?: boolean;
  children: string;
}) {
  return (
    <figure>
      <div className="hud-panel menu-shot">
        {loop ? (
          <video src={`/tutorial/${name}.mp4`} autoPlay loop muted playsInline />
        ) : (
          <img src={`/tutorial/${name}.jpg`} alt="" />
        )}
      </div>
      <figcaption>{children}</figcaption>
    </figure>
  );
}

export function Tutorial() {
  return (
    <article className="menu-tutorial" aria-label="How to play" tabIndex={0}>
      <Section title="The aim">
        <p>
          Every battle is fought over a handful of <strong>objectives</strong>: numbered capture
          zones spread across the map (three on a small map, five on a medium one, seven on large
          ones). Take them, hold them, and your <strong>victory score</strong> climbs towards{" "}
          <strong>1,000</strong>. The first side to reach it wins.
        </p>
        <Shot name="overview">
          A battle opening: the town and its three numbered objectives, under the preparation
          countdown and the order to start.
        </Shot>
        <p>
          Your opponent is a computer commander with the same rules, the same income and the same
          shop as you. Destroying its forces does not win by itself; it wins you the ground.
        </p>
      </Section>

      <Section title="Starting a battle">
        <p>
          Open <strong>Skirmish</strong> and choose your <strong>faction</strong>, the{" "}
          <strong>map</strong> type (open country, mixed, or a dense city), its{" "}
          <strong>size</strong>, and, if you care, its <strong>region</strong>. Press{" "}
          <strong>Deploy</strong>: a fresh map is generated for you. Both armies start with nothing
          on the field; every unit you fight with, you buy.
        </p>
      </Section>

      <Section title="Preparation">
        <p>
          The battle opens in <strong>preparation</strong>, shown at the top of the screen with a
          countdown of {skirmish.preparation_s} seconds. Nothing moves and nobody fires yet. This is
          your time to read the map and spend your opening credits.
        </p>
        <ul>
          <li>
            Look over the ground: the numbered objective flags, the roads, the woods and the towns.
            Your forces will arrive from <strong>your entry point</strong>, the road marker at your
            edge of the map; the enemy arrives from the opposite edge.
          </li>
          <li>
            Buy units and <strong>place</strong> them where you want them to go (see{" "}
            <em>Buying units</em>). A placement is an order, not a unit: when the battle starts,
            each one drives in from your entry point to the spot you chose, one unit every{" "}
            {skirmish.dispatch_interval_s} second, by the fastest route.
          </li>
          <li>
            Placements cannot be moved or taken back once confirmed, so place with care. Until the
            battle starts they show in your army bar as greyed cards, <em>Will be deployed</em>.
          </li>
          <li>The enemy never sees your placements, and you never see theirs.</li>
          <li>
            Press <strong>Start battle</strong> when you are ready. The battle also starts by itself
            when the countdown runs out.
          </li>
        </ul>
      </Section>

      <Section title="Credits">
        <p>
          Credits (<strong>CR</strong>, beside the Reinforcements button at the bottom left) are
          what you buy units with.
        </p>
        <ul>
          <li>
            You start with{" "}
            <strong>
              {/* The opening purse is that many minutes of income, as the simulation grants it. */}
              {(skirmish.credits_per_minute * skirmish.starting_minutes).toLocaleString("en")} CR
            </strong>
            .
          </li>
          <li>
            Once the battle starts, both sides earn{" "}
            <strong>{skirmish.credits_per_minute} CR a minute</strong>, steadily. Nothing is earned
            during preparation, and holding objectives does not pay credits: it pays score.
          </li>
          <li>
            Destroying an enemy unit pays you a share of what it cost (more for a supply truck).
          </li>
          <li>
            A unit you no longer need can be sent home with <strong>Refund · Return to base</strong>{" "}
            on the command bar. It drives back to your entry point and, on arrival, returns part of
            its price: more when it comes home healthy and with ammunition, less the longer it has
            served.
          </li>
          <li>
            You may field at most <strong>{skirmish.max_units} units</strong> at once, counting
            those still on their way in.
          </li>
        </ul>
      </Section>

      <Section title="Buying units">
        <ul>
          <li>
            Click <strong>Reinforcements</strong> at the bottom left. Your faction&apos;s shop opens
            with tabs by role: <strong>REC</strong> (reconnaissance), <strong>INF</strong>{" "}
            (infantry), <strong>VEH</strong> (fighting vehicles and tanks), <strong>SUP</strong>{" "}
            (supply trucks), and helicopters and aircraft, which are not available yet.
          </li>
          <li>
            Each card is a family of units and shows its cheapest price, which warns you when you
            cannot afford it. <strong>Hover</strong> a card to fan out its variants with their
            weapons and prices, and click the one you want; click the card itself to take the first
            one you can afford.
          </li>
          <li>
            A ghost of the unit follows the cursor (a squad shows where each soldier will stand).{" "}
            <kbd>Left-click</kbd> the ground to confirm, where it fits. <kbd>Right-click</kbd> or{" "}
            <kbd>Esc</kbd> cancels without spending anything.
          </li>
          <li>
            The price is paid when you confirm. Each placement buys one unit; open the shop again
            for the next.
          </li>
          <li>
            You can keep buying all battle: a unit bought mid-fight drives in from your entry point
            to where you placed it, so place reinforcements where the road can bring them.
          </li>
        </ul>
        <Shot name="buy">
          The vehicles tab, the M1 Abrams family fanned out above its card: each variant&apos;s
          weapons, ammunition and price.
        </Shot>
        <Shot name="place" loop>
          Placing a rifle squad: its ghost shows where each soldier will stand. Over a building the
          squad can&apos;t stand in, the ghost turns orange and a small red cross marks the cursor.
        </Shot>
        <p>
          Spend for a balanced force. Cheap reconnaissance finds the enemy, infantry holds towns and
          woods, armour wins open ground, and one or two supply trucks keep everything fighting.
        </p>
      </Section>

      <Section title="Selecting units">
        <ul>
          <li>
            <kbd>Left-click</kbd> a unit to select it. Left-click empty ground to clear the
            selection.
          </li>
          <li>
            <kbd>Left-drag</kbd> a box over the field to select every unit inside it.
          </li>
          <li>
            <kbd>Shift</kbd> + <kbd>Left-click</kbd> adds a unit to the selection, or removes it.{" "}
            <kbd>Shift</kbd> + <kbd>Left-drag</kbd> adds a whole box.
          </li>
          <li>
            <kbd>Double-click</kbd> a unit to select all your units of that type. Double-click it
            again (or <kbd>Ctrl</kbd> + double-click) to select every unit sharing its role.
          </li>
          <li>
            Every unit you own also has a card in the army bar along the bottom: click it to select
            that unit, <kbd>Shift</kbd>-click to add it, hover to see its details.
          </li>
        </ul>
      </Section>

      <Section title="Moving">
        <ul>
          <li>
            <kbd>Right-click</kbd> the ground to move the selection there by the shortest route.
          </li>
          <li>
            <kbd>Right-drag</kbd> to move and set the direction the units face when they arrive:
            drag from the destination towards where the enemy should be. Wheeled vehicles keep the
            heading they arrive on.
          </li>
          <li>
            <kbd>Double right-click</kbd> for a fast move: the units take the quickest route, using
            roads, rather than the most direct one.
          </li>
          <li>
            <kbd>Shift</kbd> + <kbd>Right-click</kbd> queues a waypoint after the current orders
            instead of replacing them.
          </li>
          <li>
            <kbd>{keyOf(CommandBindings.reverse_move)}</kbd>, or a right-click just behind a single
            selected vehicle, backs it up without turning round.
          </li>
          <li>
            <kbd>Right-click</kbd> a building with infantry selected to garrison it: one squad per
            building, and the whole squad must fit. <strong>Leave building</strong> on the command
            bar brings them out.
          </li>
          <li>
            <kbd>{CommandBindings.stop.label}</kbd> stops the selection where it stands.
          </li>
        </ul>
        <Shot name="move" loop>
          A right-drag: press where the tank should go, drag the way it should face, and its ghost
          shows both. Release, and it drives there.
        </Shot>
      </Section>

      <Section title="Fighting">
        <p>
          Units open fire on their own at anything they can see and hurt. Orders shape where they
          fight:
        </p>
        <ul>
          <li>
            <kbd>Right-click</kbd> an enemy to attack it, or a red contact area to fire into it.
          </li>
          <li>
            <kbd>{keyOf(CommandBindings.attack_move)}</kbd> then right-click (or <kbd>Ctrl</kbd> +{" "}
            <kbd>Right-click</kbd>) for an <strong>attack-move</strong>: the units advance and halt
            to fight whenever a weapon has a target.
          </li>
          <li>
            <kbd>{CommandBindings.attack_ground.label}</kbd> then left-click to fire on a patch of
            ground, such as a treeline you suspect. Right-click cancels.
          </li>
          <li>
            <kbd>{CommandBindings.toggle_fire_policy.label}</kbd> switches between{" "}
            <strong>fire at will</strong> and <strong>return fire only</strong>: hold fire to stay
            hidden in an ambush.
          </li>
          <li>
            <kbd>{CommandBindings.toggle_deployment.label}</kbd> deploys or packs a unit that must
            set up before working, such as a supply truck or a heavy missile team. A deployed unit
            packs up before it can move again.
          </li>
          <li>
            <kbd>{CommandBindings.disarm.label}</kbd> drops a command you have armed (like
            attack-move) without using it.
          </li>
          <li>
            <kbd>{ShowOrdersBinding.label}</kbd> to see every unit&apos;s route, destination and
            cover at once.
          </li>
        </ul>
        <p>
          The same commands are buttons on the command bar, which appears at the bottom when units
          are selected; hover a button to see its key. A command goes only to the selected units
          that can carry it out. An order the game refuses says why beside the cursor.
        </p>
      </Section>

      <Section title="The camera">
        <ul>
          <li>
            <kbd>W</kbd> <kbd>A</kbd> <kbd>S</kbd> <kbd>D</kbd> or the arrow keys pan; hold{" "}
            <kbd>Shift</kbd> to pan faster. Resting the pointer at the edge of the screen pans too.
          </li>
          <li>
            <kbd>Q</kbd> and <kbd>E</kbd> rotate the view. <kbd>Middle-drag</kbd> orbits and tilts.
          </li>
          <li>The mouse wheel zooms, from close over a squad to the whole battlefield.</li>
        </ul>
      </Section>

      <Section title="Seeing the enemy">
        <ul>
          <li>
            You see only what your own units can see. Woods, buildings and hills hide the enemy, and
            the enemy is just as blind to you.
          </li>
          <li>
            Gunfire gives a unit away. An enemy heard firing, or one that slips out of sight,
            becomes a red <strong>contact</strong>: an area where something is, not an exact
            position. Contacts fade after a while.
          </li>
          <li>
            Infantry in woods is hard to spot; a well-hidden squad shows <em>Hidden</em> on its
            panel. Vehicles hide far less well.
          </li>
          <li>
            Every unit has a floating panel with its name, its weapons and ammunition, and what it
            is doing: suppressed, deploying, resupplying, returning to base. An enemy&apos;s panel
            shows only what your side knows.
          </li>
        </ul>
        <Shot name="contact">
          Identified enemies, named in red, and a contact: a red area where an enemy was heard or
          last seen.
        </Shot>
      </Section>

      <Section title="Staying alive">
        <ul>
          <li>
            <strong>Cover:</strong> infantry behind walls, wrecks, trees or in craters is much
            harder to hit. A garrisoned building shelters a whole squad, until it collapses.
          </li>
          <li>
            <strong>Suppression:</strong> rounds landing close make soldiers <em>Suppressed</em> and
            then <em>Pinned</em>: slower to move, slower to reload, less accurate. It wears off a
            few seconds after the fire stops. Suppress a position before you assault it.
          </li>
          <li>
            <strong>Supply:</strong> a deployed supply truck refills ammunition, repairs vehicles
            and replaces fallen soldiers for friendly units that stop nearby and hold their fire.
            Its stock does not refill, so a truck that runs dry is best refunded.
          </li>
          <li>
            Turn your tanks&apos; fronts to the threat: reverse out of trouble rather than turning
            your back.
          </li>
        </ul>
      </Section>

      <Section title="How to win">
        <ul>
          <li>
            <strong>Capturing:</strong> move reconnaissance, infantry or fighting vehicles into an
            objective&apos;s ring (a squad counts once any soldier is inside). With only your side
            there, it is yours after 20 seconds; leave early and the count starts again. Supply
            trucks cannot capture.
          </li>
          <li>
            <strong>Contesting:</strong> with both sides inside a ring it is{" "}
            <strong>contested</strong>. Capture pauses, and an owned objective earns nothing for as
            long as an enemy stands in it.
          </li>
          <li>
            <strong>Score:</strong> each objective you own adds to your score every second, even
            with nobody guarding it. Holding a bare majority reaches 1,000 in about 30 minutes;
            holding more gets there sooner.
          </li>
          <li>
            <strong>Total control:</strong> own every objective with none contested and you win on
            the spot.
          </li>
          <li>
            The top bar shows both scores and every objective: whose it is, its capture progress, or{" "}
            <em>Contested</em>. It reads <strong>Victory</strong>, <strong>Defeat</strong> or{" "}
            <strong>Draw</strong> when the battle ends.
          </li>
        </ul>
        <Shot name="capture">
          Your unit alone in an objective&apos;s ring: its flag counts the capture up.
        </Shot>
        <p>
          Grab the nearest objectives early with fast units, then hold more than half. Scout ahead
          so you find the enemy before it finds you, keep your score ticking, and strike at the
          objective that hurts the enemy most to lose: a unit of yours standing in an enemy ring
          stops it scoring, just as an enemy in one of yours stops you.
        </p>
      </Section>

      <Section title="Pausing">
        <p>
          <kbd>Esc</kbd> (or the menu button at the top) pauses the battle. From the pause menu you
          can resume, restart, save a replay to watch later, change the sound, or return here.
        </p>
      </Section>
    </article>
  );
}
