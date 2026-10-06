# Starter balance baseline

Agent-selected planning data under the user's explicit numerical delegation.
Nothing in this file is implemented or playtested. This is the numerical starting
point for the [roster spec](README.md), not a second runtime catalog. Move values
into the existing authored catalog as implementation proceeds; remove this table
at closeout rather than maintaining competing copies.

Every membership below has a price, body/mobility/sensing profile and weapon pack.
Shared named platform variants reuse one tuning/model owner across factions;
membership is separate. A profile is an authoring shorthand, not a new combat rule.
Variants may share some starting stats while retaining independent identities,
prices and visible equipment. Trophy adds four charges and a three-second cooldown.

The intended first phase is ground REC/INF/VEH and resupply-only SUP. Drones,
helicopters, aircraft, artillery and air-defense-only units are disabled until
their relevant mechanics are implemented. Listed weapon packs can contain future
capabilities: unsupported entries remain disabled, rather than silently pretending
to have transport, anti-air, top-attack or electronic-warfare behavior.

## Body, mobility and sensing profiles

HP is per soldier for squads (100), total for hulls. Armor is front/side/rear/roof
in the engine's abstract protection units, not millimeters. Ground speeds are
off-road/road in km/h. A single airborne speed is a provisional cruise value and
cannot be loaded into the current ground-only mobility schema. Sight is initial
open-ground range in meters, subject to existing shape, occlusion and concealment.
Dimensions, muzzle sockets and pivots must be frozen from platform references
before model tasks; these gameplay profiles do not substitute for physical fit.

| Profile | Body / HP | Speeds km/h | Sight m | Armor F/S/R/T |
|---|---|---:|---:|---|
| scout | 4 soldiers / 100 each | 11 / 14 | 750 | — |
| elite-scout | 4 soldiers / 100 each | 11 / 14 | 800 | — |
| sniper | 2 soldiers / 100 each | 10 / 12 | 800 | — |
| rifle | 8 soldiers / 100 each | 11 / 14 | 600 | — |
| assault | 8 soldiers / 100 each | 11 / 14 | 550 | — |
| marksman | 6 soldiers / 100 each | 11 / 14 | 650 | — |
| at-team | 3 soldiers / 100 each | 10 / 12 | 600 | — |
| rpg-team | 3 soldiers / 100 each | 11 / 14 | 550 | — |
| aa-team | 3 soldiers / 100 each | 10 / 12 | 600 | — |
| light-car | hull / 45 | 32 / 100 | 700 | 10/8/6/5 |
| wheeled-recon | hull / 65 | 32 / 90 | 850 | 35/20/15/10 |
| tracked-recon | hull / 80 | 28 / 65 | 850 | 55/30/20/15 |
| apc | hull / 70 | 30 / 90 | 450 | 35/25/15/10 |
| heavy-apc | hull / 90 | 28 / 80 | 450 | 65/40/25/20 |
| light-ifv | hull / 70 | 28 / 75 | 500 | 45/25/15/10 |
| ifv | hull / 90 | 28 / 65 | 550 | 75/40/25/20 |
| wheeled-ifv | hull / 85 | 30 / 85 | 550 | 60/35/20/15 |
| advanced-ifv | hull / 100 | 30 / 70 | 600 | 90/50/30/25 |
| heavy-ifv | hull / 120 | 25 / 60 | 550 | 130/75/40/30 |
| missile-carrier | hull / 70 | 30 / 85 | 650 | 40/25/15/10 |
| older-mbt | hull / 100 | 24 / 55 | 450 | 160/90/55/30 |
| mbt | hull / 110 | 27 / 60 | 500 | 190/100/60/35 |
| improved-mbt | hull / 120 | 28 / 65 | 550 | 220/115/65/40 |
| heavy-mbt | hull / 130 | 24 / 55 | 500 | 230/120/65/40 |
| advanced-mbt | hull / 135 | 28 / 65 | 600 | 260/130/70/45 |
| light-tank | hull / 80 | 30 / 70 | 550 | 95/45/25/20 |
| truck | hull / 60 | 25 / 80 | 300 | 10/10/8/5 |
| tracked-howitzer | hull / 80 | 24 / 60 | 400 | 30/20/15/10 |
| wheeled-howitzer | hull / 60 | 25 / 90 | 400 | 15/10/8/5 |
| heavy-rocket | hull / 80 | 24 / 65 | 350 | 20/15/10/8 |
| light-rocket | hull / 60 | 25 / 85 | 350 | 10/10/8/5 |
| mortar-carrier | hull / 70 | 30 / 85 | 400 | 35/25/15/10 |
| aa-tracked | hull / 80 | 25 / 65 | 700 | 40/25/15/10 |
| aa-launcher | hull / 60 | 25 / 80 | 900 | 10/10/8/5 |
| scout-drone | hull / 15 | cruise 60 | 850 | 0/0/0/0 |
| thermal-drone | hull / 15 | cruise 60 | 1,000 | 0/0/0/0 |
| kamikaze-personnel | hull / 10 | cruise 80 | 450 | 0/0/0/0 |
| kamikaze-armor | hull / 10 | cruise 80 | 450 | 0/0/0/0 |
| armed-drone | hull / 50 | cruise 250 | 1,200 | 0/0/0/0 |
| light-helicopter | hull / 55 | cruise 180 | 700 | 5/5/5/5 |
| utility-helicopter | hull / 80 | cruise 220 | 700 | 8/8/6/5 |
| heavy-helicopter | hull / 110 | cruise 230 | 700 | 8/8/6/5 |
| attack-helicopter | hull / 90 | cruise 240 | 900 | 20/15/10/8 |
| armored-helicopter | hull / 110 | cruise 220 | 850 | 25/20/10/8 |
| fighter | hull / 90 | cruise 850 | 1,500 | 0/0/0/0 |
| heavy-fighter | hull / 110 | cruise 850 | 1,700 | 0/0/0/0 |
| strike-fighter | hull / 110 | cruise 800 | 1,500 | 0/0/0/0 |
| stealth-fighter | hull / 95 | cruise 850 | 1,700 | 0/0/0/0 |
| ew-aircraft | hull / 95 | cruise 800 | 1,500 | 0/0/0/0 |
| attack-aircraft | hull / 120 | cruise 550 | 1,200 | 20/15/10/8 |

Recon squads use the existing concealed-detection multiplier of 2; other infantry
use 1. Recon vehicles use 1.5. Infantry sights are all-round; ground hulls start
at front/side/rear sight multipliers 1/0.5/0.3, except reconnaissance 1/1/1.
Tracked turn rate starts at 45 degrees/second and reverse fraction 0.4; wheeled
turn rate 40 and reverse fraction 0.35. Wheeled turning radius starts at 6 m and
must be checked against the platform's frozen body dimensions. Use existing
ricochet rules, with ground hull face chances 0.10/0.10/0.05/0.15 below 100 front
armor and 0.50/0.35/0.20/0.60 above it. Do not assign soldier armor as a new mechanic.

## Weapon packs

`+` combines physical weapons; each pack name below describes a weapon or an
ammo pair. Counts are per physical launcher/gun; squad default guns are per soldier.
HP damage and penetration use current engine units. Ranges are gameplay-scaled,
not real-world claims. These are starter envelopes: derived flight data must pass
the existing ballistic admission before use; a listed range alone is not proof.

| Weapon | Range m | Damage | Penetration | Carried ammo | Reload s |
|---|---:|---:|---:|---|---:|
| rifle | 300 | 35 | 5 | unlimited | 0 |
| assault-rifle | 240 | 35 | 5 | unlimited | 0 |
| marksman-rifle | 450 | 45 | 8 | 120 | 3 |
| heavy-sniper | 700 | 100 | 30 | 30 | 2.5 |
| grenade | 150 | 20 | 20 | 8 | 5 |
| HMG | 400 | 20 | 20 | unlimited | 5 |
| autocannon | 650 | 20 | 65 | 240 | 6 |
| tank-gun (AP / HE) | 850 | 45 / 80 | 250 / 30 | 20 / 15 | 6 |
| advanced-tank-gun (AP / HE) | 900 | 50 / 80 | 300 / 30 | 20 / 15 | 6 |
| light-tank-gun (AP / HE) | 750 | 40 / 65 | 170 / 25 | 20 / 15 | 6 |
| Javelin | 900 | 55 | 320 | 3 | 10 |
| TOW | 900 | 45 | 300 | 4 | 9 |
| Akeron | 900 | 50 | 310 | 3 | 10 |
| Spike | 900 | 55 | 320 | 4 | 10 |
| IFV-HE | 650 | 65 | 25 | 20 | 6 |
| Kornet | 950 | 55 | 320 | 4 | 10 |
| RPG-light | 300 | 45 | 160 | 4 | 8 |
| RPG-heavy | 400 | 55 | 240 | 3 | 9 |
| MANPADS | 1,200 | 60 | 30 | 4 | 8 |
| howitzer | 2,000 | 100 | 35 | 24 | 10 |
| mortar | 1,000 | 55 | 20 | 30 | 6 |
| rocket-battery | 2,400 | 75 | 30 | 12 | 20 |
| precision-missile | 3,000 | 140 | 120 | 2 | 20 |
| AA-gun | 900 | 20 | 55 | 300 | 6 |
| SAM | 2,000 | 90 | 40 | 6 | 10 |
| helicopter-gun | 650 | 20 | 50 | 300 | 6 |
| helicopter-AT | 1,000 | 55 | 320 | 8 | 8 |
| rocket-pod | 900 | 30 | 25 | 24 | 6 |
| fighter-gun | 600 | 25 | 60 | 240 | 6 |
| attack-gun | 750 | 25 | 100 | 360 | 6 |
| AAM | 2,000 | 80 | 30 | 4 | 8 |
| air-AT | 1,200 | 60 | 320 | 4 | 8 |
| bomb | 1,000 delivery envelope | 120 | 60 | 2 | 10 |
| drone-HE | 1,000 mission envelope | 50 | 15 | 1, consumes drone | — |
| drone-AT | 1,000 mission envelope | 65 | 300 | 1, consumes drone | — |
| Trophy | 8 interception distance | ordinary threat detonates early | — | 4 | 3 cooldown |

Keep rifle/HMG unlimited reserves in this first baseline, as existing mechanics
do; finite heavy ammo supplies the requested logistics pressure. Marksman guns
use a 20-round magazine, one-shot bursts at 0.5 s; autocannons use 30-round
magazines, three-shot bursts at 0.2 s. Other magazine/aim/scatter/flight defaults
derive from the closest existing weapon row, with admission rechecked for changed
range. For single-shot new rows, start with 2 s aim unless the donor's supported
guidance requires otherwise. Direct rockets use existing unguided flight; guided
missiles reuse existing guidance until separate top-attack behavior is admitted.
Do not claim a Javelin has its final trajectory merely because its card exists.

Future artillery, drone, air-defense, air-combat, transport and EW numbers are
explicit placeholders, never permission to implement their rules in the ground
phase. EW aircraft stats do not imply a jamming capability exists. Airborne sensing
and armor interpretation await their future mechanics.

## Resupply baseline

All three trucks start with 250 supply stock, 80 m service radius, 3 s setup and
1 s packing, preserving the existing service baseline. Stock remains finite and
cannot be replenished by another truck or by returning to base in this phase.
Truck retirement uses the ordinary refund flow; a new truck is a new purchase.

Existing ammunition costs are grenade 2, AP/HE 5, guided anti-tank missile 20.
New per-round stock costs: marksman 1, sniper 2, autocannon/AA/aircraft gun 1,
RPG 10, MANPADS 20, howitzer 10, mortar 5, artillery rocket 15, precision missile
60, helicopter rocket 5, SAM/AAM 30, air-to-ground missile 20 and bomb 30.
Drone rounds are not resupplied; the drone is consumed. Every admitted finite
weapon row must have a service cost. Ordinary ammo uses the existing one-round
per-second service; Trophy is a separate ten-second item costing 20 stock.
Vehicle repair remains 2 HP/s for 1 stock/HP; replacement soldiers take 5 s and
15 stock. Preserve existing recipient restrictions and deterministic service order.

For refunds, H is remaining squad member HP divided by the full original squad
HP, including dead members as zero, or hull HP/max HP. A averages remaining/full
fractions of finite carried ammunition rows, using original authored carriers
and capacities so lost weapons do not improve the fraction; ignore unlimited
rows. If there are no finite ammo rows, A = 1. Include Trophy charges as a finite
row. Truck stock contributes as another finite-resource row, preventing empty
trucks being refunded as fully supplied. Clamp all fractions to [0,1]. Field age
starts at physical spawn and includes withdrawal; condition is measured on arrival.

## Per-entry prices and profile assignments

Each section has exactly 50 faction memberships, including variants. Shared
U.S./European Javelin and Stinger teams, Apache AH-64E, F-16C Block 50 and F-35A
use the same price/profile/weapon pack. Generic drone loadouts can share planning
stats while their concrete platforms/model identities remain to be chosen.

### U.S. faction

| Category | Family | Variant / roster role | Credits | Profile | Weapons |
|---|---|---|---:|---|---|
| REC | Army Scouts | Light patrol | 80 | scout | rifle |
| REC | Force Recon | Recon patrol | 100 | elite-scout | rifle+grenade |
| REC | Scout Snipers | M107 heavy sniper team | 100 | sniper | heavy-sniper |
| REC | M3 Bradley CFV | M3A3 | 210 | tracked-recon | autocannon+TOW |
| REC | M1127 Stryker RV | Reconnaissance vehicle | 130 | wheeled-recon | HMG |
| REC | Scout Drone | Standard | 60 | scout-drone | none |
| REC | Scout Drone | Thermal | 90 | thermal-drone | none |
| INF | Rifle Squad | Standard infantry | 90 | rifle | rifle+grenade |
| INF | Marine Squad | Close-quarters infantry | 110 | assault | assault-rifle+grenade |
| INF | Marksman Squad | M110-equipped longer-range infantry | 110 | marksman | marksman-rifle |
| INF | ATGM Team | FGM-148 Javelin | 150 | at-team | Javelin |
| INF | ATGM Team | BGM-71 TOW 2A | 130 | at-team | TOW |
| INF | Air-Defense Team | FIM-92 Stinger | 100 | aa-team | MANPADS |
| VEH | M1 Abrams | SEP v2 | 350 | mbt | tank-gun+HMG |
| VEH | M1 Abrams | SEP v2 Trophy | 410 | mbt | tank-gun+HMG+Trophy |
| VEH | M1 Abrams | SEP v3 Trophy | 450 | advanced-mbt | advanced-tank-gun+HMG+Trophy |
| VEH | M2 Bradley IFV | M2A4 | 230 | ifv | autocannon+TOW |
| VEH | Stryker | M1126 ICV | 140 | apc | HMG |
| VEH | Stryker | M1296 Dragoon | 200 | wheeled-ifv | autocannon |
| VEH | Stryker | M1134 ATGM | 190 | missile-carrier | TOW |
| VEH | ACV | ACV-P | 170 | heavy-apc | HMG |
| VEH | LAV | LAV-25A2 | 160 | light-ifv | autocannon |
| VEH | LAV | LAV-AT | 180 | missile-carrier | TOW |
| VEH | M1151 HMMWV | HMG | 90 | light-car | HMG |
| SUP | M109 Paladin | M109A7 | 280 | tracked-howitzer | howitzer |
| SUP | M270 MLRS | M270A2 | 350 | heavy-rocket | rocket-battery |
| SUP | M142 HIMARS | GMLRS | 300 | light-rocket | rocket-battery |
| SUP | M142 HIMARS | PrSM | 400 | light-rocket | precision-missile |
| SUP | M1129 Stryker Mortar | 120 mm mortar carrier | 180 | mortar-carrier | mortar |
| SUP | Stryker M-SHORAD | Gun-and-missile air-defense vehicle | 240 | wheeled-ifv | AA-gun+MANPADS |
| SUP | M977 HEMTT | General resupply | 90 | truck | none |
| SUP | Kamikaze Drone | Anti-personnel | 60 | kamikaze-personnel | drone-HE |
| SUP | Kamikaze Drone | Anti-armor | 90 | kamikaze-armor | drone-AT |
| HEL | AH-64 Apache | AH-64E Guardian | 360 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | UH-60 Black Hawk | UH-60M | 180 | utility-helicopter | HMG |
| HEL | CH-47 Chinook | CH-47F | 240 | heavy-helicopter | HMG |
| HEL | AH-6/MH-6 Little Bird | AH-6M | 180 | light-helicopter | helicopter-gun+rocket-pod |
| HEL | AH-6/MH-6 Little Bird | MH-6M | 120 | light-helicopter | HMG |
| HEL | AH-1Z Viper | Attack helicopter | 320 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | UH-1Y Venom | Utility transport | 160 | utility-helicopter | HMG |
| AIR | F-16 Fighting Falcon | F-16C Block 50 | 350 | fighter | fighter-gun+AAM+bomb |
| AIR | F-15 Eagle | F-15E Strike Eagle | 400 | strike-fighter | fighter-gun+AAM+bomb |
| AIR | F-15 Eagle | F-15EX Eagle II | 430 | heavy-fighter | fighter-gun+AAM+bomb |
| AIR | F-22 Raptor | F-22A | 500 | stealth-fighter | fighter-gun+AAM |
| AIR | F-35 Lightning II | F-35A | 470 | stealth-fighter | fighter-gun+AAM+bomb |
| AIR | F-35 Lightning II | F-35B | 490 | stealth-fighter | fighter-gun+AAM+bomb |
| AIR | F/A-18 Super Hornet | F/A-18E | 370 | fighter | fighter-gun+AAM+bomb |
| AIR | EA-18G Growler | Electronic-warfare aircraft | 380 | ew-aircraft | AAM |
| AIR | A-10 Warthog | A-10C — retained as an iconic exception if needed | 300 | attack-aircraft | attack-gun+air-AT+bomb |
| AIR | MQ-9 Reaper | MQ-9A | 260 | armed-drone | air-AT |

### European faction

| Category | Family | Variant / roster role | Credits | Profile | Weapons |
|---|---|---|---:|---|---|
| REC | Fennek | Reconnaissance vehicle | 130 | wheeled-recon | HMG |
| REC | EBRC Jaguar | Armed reconnaissance vehicle | 230 | wheeled-recon | autocannon+Akeron |
| REC | Ajax | Tracked reconnaissance vehicle | 220 | tracked-recon | autocannon |
| REC | VBL | Machine-gun scout | 90 | light-car | HMG |
| REC | Recon Patrol | Light patrol | 80 | scout | rifle |
| REC | Scout Drone | Standard | 60 | scout-drone | none |
| REC | Scout Drone | Thermal | 90 | thermal-drone | none |
| INF | Rifle Squad | Standard infantry | 90 | rifle | rifle+grenade |
| INF | Assault Squad | Close-quarters infantry | 110 | assault | assault-rifle+grenade |
| INF | Marksman Squad | Longer-range infantry | 110 | marksman | marksman-rifle |
| INF | ATGM Team | FGM-148 Javelin | 150 | at-team | Javelin |
| INF | ATGM Team | Akeron MP | 150 | at-team | Akeron |
| INF | Air-Defense Team | FIM-92 Stinger | 100 | aa-team | MANPADS |
| VEH | Leopard 2 | 2A6 | 340 | mbt | tank-gun+HMG |
| VEH | Leopard 2 | 2A7V | 390 | improved-mbt | tank-gun+HMG |
| VEH | Leopard 2 | 2A8 | 450 | advanced-mbt | advanced-tank-gun+HMG+Trophy |
| VEH | Challenger | Challenger 2 TES | 390 | heavy-mbt | tank-gun+HMG |
| VEH | Challenger | Challenger 3† | 450 | advanced-mbt | advanced-tank-gun+HMG+Trophy |
| VEH | Leclerc | XLR | 390 | improved-mbt | tank-gun+HMG |
| VEH | CV90 | CV9040C | 220 | ifv | autocannon |
| VEH | CV90 | Mk IV | 260 | advanced-ifv | autocannon |
| VEH | Puma | Level C | 260 | heavy-ifv | autocannon+Spike |
| VEH | Boxer | APC | 170 | heavy-apc | HMG |
| VEH | Boxer | RCT30 | 240 | wheeled-ifv | autocannon |
| VEH | VBCI | Infantry fighting vehicle | 220 | wheeled-ifv | autocannon |
| VEH | KF51 Panther† | Prototype main battle tank | 460 | advanced-mbt | advanced-tank-gun+HMG |
| SUP | PzH 2000 | Tracked 155 mm howitzer | 310 | tracked-howitzer | howitzer |
| SUP | CAESAR | 8×8 | 260 | wheeled-howitzer | howitzer |
| SUP | MARS II | Multiple-launch rocket system | 340 | heavy-rocket | rocket-battery |
| SUP | Gepard | 1A2 | 200 | aa-tracked | AA-gun |
| SUP | Boxer Skyranger 30 | Gun-and-missile air defense | 250 | wheeled-ifv | AA-gun+MANPADS |
| SUP | NASAMS | NASAMS 3 | 300 | aa-launcher | SAM |
| SUP | MAN HX | General resupply | 90 | truck | none |
| SUP | Kamikaze Drone | Anti-personnel | 60 | kamikaze-personnel | drone-HE |
| SUP | Kamikaze Drone | Anti-armor | 90 | kamikaze-armor | drone-AT |
| HEL | EC665 Tiger | UHT | 330 | attack-helicopter | helicopter-AT+rocket-pod |
| HEL | EC665 Tiger | HAD | 350 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | NH90 | TTH | 190 | utility-helicopter | HMG |
| HEL | AW101 Merlin | HC4 | 230 | heavy-helicopter | HMG |
| HEL | AW159 Wildcat | AH1 | 170 | light-helicopter | HMG |
| HEL | AH-64 Apache | AH-64E Guardian | 360 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | CH-47 Chinook | HC6 | 240 | heavy-helicopter | HMG |
| AIR | Eurofighter Typhoon | Tranche 3 | 410 | fighter | fighter-gun+AAM+bomb |
| AIR | Rafale | C F4 | 420 | fighter | fighter-gun+AAM+bomb |
| AIR | JAS 39 Gripen | E | 350 | fighter | fighter-gun+AAM+bomb |
| AIR | Tornado | IDS | 310 | strike-fighter | fighter-gun+AAM+bomb |
| AIR | Tornado | ECR | 340 | ew-aircraft | AAM |
| AIR | Mirage 2000 | 2000D RMV | 290 | strike-fighter | fighter-gun+AAM+bomb |
| AIR | F-16 Fighting Falcon | F-16C Block 50 | 350 | fighter | fighter-gun+AAM+bomb |
| AIR | F-35 Lightning II | F-35A | 470 | stealth-fighter | fighter-gun+AAM+bomb |

### Eastern faction

| Category | Family | Variant / roster role | Credits | Profile | Weapons |
|---|---|---|---:|---|---|
| REC | Scouts | Light patrol | 80 | scout | rifle |
| REC | Sniper Team | ASVK heavy sniper team | 100 | sniper | heavy-sniper |
| REC | BRM | BRM-3K | 180 | tracked-recon | autocannon |
| REC | Tigr | Tigr-M | 90 | light-car | HMG |
| REC | Orlan-10 | Reconnaissance drone | 80 | scout-drone | none |
| REC | Scout Drone | Thermal | 90 | thermal-drone | none |
| INF | Rifle Squad | Standard infantry | 90 | rifle | rifle+grenade |
| INF | Assault Squad | Close-quarters infantry | 110 | assault | assault-rifle+grenade |
| INF | Marksman Squad | SVD-equipped longer-range infantry | 110 | marksman | marksman-rifle |
| INF | RPG Team | RPG-7 | 90 | rpg-team | RPG-light |
| INF | RPG Team | RPG-29 | 110 | rpg-team | RPG-heavy |
| INF | ATGM Team | Kornet | 140 | at-team | Kornet |
| INF | Air-Defense Team | Igla-S | 100 | aa-team | MANPADS |
| INF | Air-Defense Team | FN-6 | 100 | aa-team | MANPADS |
| VEH | T-72 | T-72B3 (2016) | 280 | older-mbt | tank-gun+HMG |
| VEH | T-80 | T-80BVM | 310 | mbt | tank-gun+HMG |
| VEH | T-90 | T-90M Proryv | 360 | improved-mbt | tank-gun+HMG |
| VEH | T-14 Armata† | Main battle tank | 440 | advanced-mbt | advanced-tank-gun+HMG |
| VEH | BMP IFV Family | BMP-2M Berezhok | 190 | light-ifv | autocannon+Kornet |
| VEH | BMP IFV Family | BMP-3 | 210 | ifv | IFV-HE+autocannon+Kornet |
| VEH | BTR | BTR-82A | 140 | light-ifv | autocannon |
| VEH | Type 99 | ZTZ-99A | 360 | improved-mbt | tank-gun+HMG |
| VEH | Type 15 | ZTQ-15 light tank | 230 | light-tank | light-tank-gun+HMG |
| VEH | ZBL-08 | Wheeled IFV | 190 | wheeled-ifv | autocannon |
| VEH | T-15 Armata† | Heavy IFV | 300 | heavy-ifv | autocannon+Kornet |
| SUP | 2S19 Msta-S | 2S19M2 | 270 | tracked-howitzer | howitzer |
| SUP | BM-21 Grad | BM-21 rocket artillery | 220 | light-rocket | rocket-battery |
| SUP | 9K515 Tornado-S | Heavy rocket artillery | 360 | heavy-rocket | rocket-battery |
| SUP | Pantsir | Pantsir-SM | 250 | aa-launcher | AA-gun+SAM |
| SUP | Buk | Buk-M3 medium-range air defense | 300 | aa-launcher | SAM |
| SUP | PLZ-05 | Chinese tracked 155 mm howitzer | 280 | tracked-howitzer | howitzer |
| SUP | Ural-4320 | General resupply | 90 | truck | none |
| SUP | Kamikaze Drone | Anti-personnel | 60 | kamikaze-personnel | drone-HE |
| SUP | Kamikaze Drone | Anti-armor | 90 | kamikaze-armor | drone-AT |
| HEL | Mi-8 Hip | Mi-8AMTSh | 170 | utility-helicopter | HMG |
| HEL | Mi-24/Mi-35 Hind | Mi-35M | 270 | armored-helicopter | helicopter-gun+helicopter-AT |
| HEL | Mi-28 Havoc | Mi-28NM | 330 | armored-helicopter | helicopter-gun+helicopter-AT |
| HEL | Ka-52 Alligator | Ka-52M | 360 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | Z-10 | Attack helicopter | 300 | attack-helicopter | helicopter-gun+helicopter-AT |
| HEL | Z-20 | Utility transport | 180 | utility-helicopter | HMG |
| AIR | Sukhoi Flanker Family | Su-27SM3 | 310 | fighter | fighter-gun+AAM |
| AIR | Sukhoi Flanker Family | Su-30SM2 | 370 | strike-fighter | fighter-gun+AAM+bomb |
| AIR | Sukhoi Flanker Family | Su-35S | 410 | heavy-fighter | fighter-gun+AAM |
| AIR | Sukhoi Flanker Family | Su-34 | 390 | strike-fighter | fighter-gun+AAM+bomb |
| AIR | MiG-29 Fulcrum | MiG-29SMT | 280 | fighter | fighter-gun+AAM+bomb |
| AIR | MiG-31 Foxhound | MiG-31BM | 350 | heavy-fighter | fighter-gun+AAM |
| AIR | Su-25 Frogfoot | Su-25SM3 | 270 | attack-aircraft | attack-gun+air-AT+bomb |
| AIR | Su-57 Felon | Stealth multirole fighter | 480 | stealth-fighter | fighter-gun+AAM+bomb |
| AIR | J-10 Vigorous Dragon | J-10C | 340 | fighter | fighter-gun+AAM+bomb |
| AIR | J-20 Mighty Dragon | Stealth fighter | 490 | stealth-fighter | fighter-gun+AAM |

## Opening-force checks, by arithmetic only

These are illustrative affordable purchases, not battle results or mandatory decks.

| Faction | Mixed opening | Cost / units | Tank opening | Cost / units |
|---|---|---|---|---|
| U.S. | 2 Rifle Squads, Marines, Marksmen, Army Scouts, TOW team, JLTV, HEMTT, Stryker ICV | 930 / 9 | 2 SEP v2 Trophy, Army Scouts, HEMTT | 990 / 4 |
| Europe | 2 Rifle Squads, Assault, Marksmen, Recon Patrol, Javelin, VBL, MAN HX, Boxer APC | 980 / 9 | 2 Leopard 2A7V, Recon Patrol, MAN HX | 950 / 4 |
| Eastern | 2 Rifle Squads, Assault, Marksmen, Scouts, 2 RPG-7 teams, Tigr, Ural, BTR-82A | 980 / 10 | 2 T-90M, Scouts, Ural | 890 / 4 |

Recheck these examples after any price change. At 200 passive credits/minute,
prices of 90/200/400 correspond to 27/60/120 seconds of passive income. Thirty
active minutes inject 6,000 credits before combat rewards. That is intentionally
more replenishment than the rejected 60/minute proposal; the 30-unit cap and
logistics must be evaluated for force density and churn. Do not silently lower
income or invent upkeep if a trial is crowded: report the paired evidence and
tune delegated unit prices first.
