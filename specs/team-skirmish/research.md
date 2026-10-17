# Team AI research

Sources gathered on 2026-10-16 for the team planner (slices 12–13). Each practice below says what it becomes in our design.

## Practices

| Practice | Source | What it becomes here |
|---|---|---|
| Three-layer command: the commander sets objectives, each squad turns its objective into orders and reports back up when it is succeeding or about to fail. | [Killzone 3 hierarchical AI, Game AI Pro ch. 29](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter29_Hierarchical_AI_for_Multiplayer_Bots_in_Killzone_3.pdf) | The team planner is the commander; each AI player is one squad. A player's report that it is losing becomes a request for help. |
| Sticky assignment: work out the ideal spread, take squads off over-assigned objectives, and prefer each squad's previous objective. | Same source | Lane assignment runs in player order and gives a bonus to keeping the current lane. |
| Regroup marker: squads gather near the objective before attacking, and pay a penalty for using paths other squads already use. | Same source | Each lane has a staging point. The main group gathers there before a push, which gives both concentration and a visible synchronized push. |
| Influence map per side, smoothed over time. | [Mark, Modular Tactical Influence Maps, Game AI Pro 2 ch. 30](https://www.gameaipro.com/GameAIPro2/GameAIPro2_Chapter30_Modular_Tactical_Influence_Maps.pdf) | A coarse grid per lane or objective: friendly strength minus visible enemy strength, blended with the previous value. Built from the side's observation only. |
| Goals priced by committed force: fund "just enough" units, and move units between goals only if that raises the total score. | [Dill & Papp, Kohan II, AIIDE 2005](https://ojs.aaai.org/index.php/AIIDE/article/download/18712/18490) | A player answers a request for help only with units beyond its own lane's need, times a margin. |
| Small fixed team signals: flank or pocket position, who is under attack, when help was last sent. | [AoE II HD AI script](https://gist.github.com/mayerwin/ac4a5ec62f51e94a3fa9) | `TeamPlan` is a small fixed-shape record, not a general shared memory. |
| Optimal one-to-one assignment. | [Gerkey & Matarić 2004](https://robotics.usc.edu/publications/347/); [Kuhn 1955](https://doi.org/10.1002/nav.3800020109) | With few players, brute-force the permutations, or use the Hungarian method if the count grows. Ties go to the lower player. Contract-net bidding ([Smith 1980](https://doi.org/10.1109/TC.1980.1675516)) is unnecessary with one central planner. |
| Asymmetric hysteresis: retreat at once on a predicted loss, re-engage only after a wait. | [UAlbertaBot Squad.cpp](https://github.com/davechurchill/ualbertabot/blob/master/UAlbertaBot/Source/Squad.cpp) | Falling back takes effect immediately; a push needs a dwell. |

## Pitfalls and the mitigations we adopt

- **Oscillation.** Kohan's "goal inertia", the inertia section of the [Game AI Pro utility chapter](https://www.gameaipro.com/GameAIPro/GameAIPro_Chapter09_An_Introduction_to_Utility_Theory.pdf), and Killzone's smoothing all address it. We use an incumbency bonus, a minimum dwell, and blending the influence map with its previous value.
- **Re-tasking units mid-fight looks wrong** (Kohan). A detachment in contact keeps its task until it finishes or retreats.
- **Help arrives too late.** A shipped AoE II script notes that its built-in cooperation "usually helps ally too late". Help requests fire when the lane's influence trend crosses a threshold, not when the objective falls.
- **Lanes starve, or everyone piles onto one objective** (Killzone). One lane per player. Only a player's surplus answers help.
- **Ignoring the human ally** (Kohan team commands). The human's inferred goals count as covered objectives, and their lane can raise requests for help.
- **Cheating through fog** (Kohan read enemy strength under fog). Forbidden here: the planner reads only the side's observation.

## Not found

There are no primary sources for team logic in Company of Heroes, Wargame/WARNO, Zero-K or the FAF AIs. The one lead not yet read is [M28AI](https://github.com/maudlin27/M28AI).
