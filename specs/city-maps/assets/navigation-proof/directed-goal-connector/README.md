# Destination connectors follow the direction of travel

Endpoint discovery tested both local connectors from the endpoint toward the road.
Destination admission must test road toward destination: a reverse-only refusal cannot
reject a passable forward journey. This fixes the proof core's directed-validator seam;
it adds no one-way game rule.

The regression uses one straight road and a validator that refuses travel out of the goal
but allows travel into it. The old source loses that corridor; the corrected one returns
the full four-point route at cost 100, and all eight small graph tests pass. Native and
Wasm outputs are 525 identical bytes, with the original 428-byte prefix unchanged.
No Battle rule, production navigation or original oracle changed.

Battle pending state and digest, counted graph work and a bounded fallback remain open.
