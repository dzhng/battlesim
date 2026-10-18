# Scrollback and requirement coverage

The original brief and completed interview map are preserved locally under assets. Later answers are authoritative over earlier proposals. All 80 map entries are reproduced in requirements.md; no chat link is needed. Named implementation resolutions live in contracts.md/decisions.md. This ledger is a planning traceability record, not a claim of passing implementation tests.

| Requirement | Owning slice(s) | First-checkpoint disposition |
|---|---|---|
| [S01](requirements.md#s01) | 03 | Village |
| [S02](requirements.md#s02) | 01 | Village |
| [S03](requirements.md#s03) | 01 | Village |
| [S04](requirements.md#s04) | 16,22 | Village foundation + continuation extension |
| [S05](requirements.md#s05) | 22 | Continuation |
| [S06](requirements.md#s06) | 21 | Continuation |
| [S07](requirements.md#s07) | 21 | Continuation |
| [S08](requirements.md#s08) | 21 | Continuation |
| [S09](requirements.md#s09) | 21 | Continuation |
| [S10](requirements.md#s10) | 21 | Continuation |
| [V01](requirements.md#v01) | 05,18,19 | Village foundation + continuation extension |
| [V02](requirements.md#v02) | 05,06 | Village |
| [V03](requirements.md#v03) | 05,09 | Village |
| [V04](requirements.md#v04) | 05 | Village |
| [V05](requirements.md#v05) | 18 | Continuation |
| [V06](requirements.md#v06) | 18,19 | Continuation |
| [V07](requirements.md#v07) | 05,08 | Village |
| [V08](requirements.md#v08) | 06 | Village |
| [V09](requirements.md#v09) | 06 | Village |
| [V10](requirements.md#v10) | 06,08 | Village |
| [V11](requirements.md#v11) | 06 | Village |
| [V12](requirements.md#v12) | 08 | Village |
| [V13](requirements.md#v13) | 06,08 | Village |
| [W01](requirements.md#w01) | 08 | Village |
| [W02](requirements.md#w02) | 08 | Village |
| [W03](requirements.md#w03) | 08 | Village |
| [W04](requirements.md#w04) | 08 | Village |
| [W05](requirements.md#w05) | 08 | Village |
| [W06](requirements.md#w06) | 08 | Village |
| [W07](requirements.md#w07) | 08 | Village |
| [W08](requirements.md#w08) | 08 | Village |
| [W09](requirements.md#w09) | 08 | Village |
| [W10](requirements.md#w10) | 08 | Village |
| [W11](requirements.md#w11) | 08,16 | Village |
| [W12](requirements.md#w12) | 08 | Village |
| [W13](requirements.md#w13) | 08 | Village |
| [W14](requirements.md#w14) | 08 | Village |
| [W15](requirements.md#w15) | 08,12 | Village |
| [W16](requirements.md#w16) | 04,08 | Village |
| [W17](requirements.md#w17) | 04,08 | Village |
| [W18](requirements.md#w18) | 08 | Village |
| [P01](requirements.md#p01) | 07 | Village |
| [P02](requirements.md#p02) | 07 | Village |
| [P03](requirements.md#p03) | 07,22 | Village foundation + continuation extension |
| [P04](requirements.md#p04) | 07 | Village |
| [P05](requirements.md#p05) | 10 | Village |
| [P06](requirements.md#p06) | 10 | Village |
| [P07](requirements.md#p07) | 19 | Continuation |
| [P08](requirements.md#p08) | 19 | Continuation |
| [P09](requirements.md#p09) | 07,09 | Village |
| [P10](requirements.md#p10) | 09 | Village |
| [P11](requirements.md#p11) | 08 | Village |
| [P12](requirements.md#p12) | 11 | Village |
| [P13](requirements.md#p13) | 09,11 | Village |
| [P14](requirements.md#p14) | 09 | Village |
| [M01](requirements.md#m01) | 04 | Village |
| [M02](requirements.md#m02) | 04 | Village |
| [M03](requirements.md#m03) | 02,04 | Village |
| [M04](requirements.md#m04) | 04 | Village |
| [M05](requirements.md#m05) | 04 | Village |
| [M06](requirements.md#m06) | 09,16 | Village |
| [M07](requirements.md#m07) | 02,09,11 | Village |
| [M08](requirements.md#m08) | 04,09 | Village |
| [M09](requirements.md#m09) | 02,04 | Village |
| [L01](requirements.md#l01) | 12 | Village |
| [L02](requirements.md#l02) | 12,13 | Village |
| [L03](requirements.md#l03) | 13 | Village |
| [L04](requirements.md#l04) | 13 | Village |
| [L05](requirements.md#l05) | 13 | Village |
| [L06](requirements.md#l06) | 17 | Continuation |
| [L07](requirements.md#l07) | 17 | Continuation |
| [L08](requirements.md#l08) | 11 | Village |
| [L09](requirements.md#l09) | 11 | Village |
| [L10](requirements.md#l10) | 11 | Village |
| [A01](requirements.md#a01) | 18 | Continuation |
| [A02](requirements.md#a02) | 20 | Continuation |
| [A03](requirements.md#a03) | 19 | Continuation |
| [U01](requirements.md#u01) | 04,08,11,17 | Village foundation + continuation extension |
| [U02](requirements.md#u02) | 14 | Village |
| [U03](requirements.md#u03) | 14 | Village |

## Additional interview context

N01 compatibility, N02 audience, N03 ambush taste, N04 first checkpoint, N05 host and N06 handoff are in requirements.md. The original role descriptions and provisional numbers are explicitly retained there. The rules-map kickoff prompt and OPEN list are marked superseded with links to this plan; the archived map is evidence only.

## Draft disagreements resolved

- Fewest-slices draft grouped the experience into five milestones. The final ladder retains that progression but separates the underlying failure-prone APIs and visual variables.
- Risk-first draft moved swept collision and knowledge filtering before tactical composition; the final dependency graph retains both gates.
- Proposed fixture sizes/ranges differed. The planner selected one provisional 1.6 km fixture and one numeric owner; incompatible alternatives are not left for the builder to accidentally combine.
- One draft proposed freezing deployment on Stop, which conflicts with the user's cancellation/reversal rule. The final contract reverses packing when Stop cancels movement.
- No draft's raw code-copy suggestion overrides the pinned dependency/license audit. Renderer composition is a reference, not a standalone import.

## Deliberate omissions from implementation, not from requirements

Village excludes playable aircraft, transport boarding, purchasing, full objective economy, general-purpose strategic AI, decks, campaign, multiplayer and finished assets. Continuation slices cover agreed full-game mechanics; decks/campaign/networking/final art remain outside this plan. No accepted battle rule is dropped merely because it is absent from the first scenario.
