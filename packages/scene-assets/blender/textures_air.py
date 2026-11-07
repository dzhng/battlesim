"""The aircraft and rotorcraft paint schemes, `textures` recipes like every
other (`textures.SCHEMES` names them); `textures` imports this module, which
registers them.
"""
from textures import Baked, _bands, blur, chips, fbm, mix, normals_from_height, recipe, scratches


# Aircraft and rotorcraft schemes: the air arms' greys and the army aviation
# greens, from each family's references. Lighter than ground paint (a grey jet
# is a light thing on screen) but kept off white so the battle's sun can't
# blow them out, and a shade cooler than they would be under a white sun (the
# battle's is warm); chipped edges in a paler tone of the same grey. Tactical
# paint is satin, not gloss: it sheens toward the sun at a grazing angle
# without mirroring the sky and the ground, so a curved skin keeps its grey
# to the silhouette.
AIR_ROUGH = 0.62


def _finished(col, seed, rough=AIR_ROUGH):
    """Finish an air scheme's colours as an airframe's paint, kept by its
    ground crew: an even coat, barely mottled, without the dust patches and
    the scratches of a ground vehicle's enamel (`_sprayed`), a fine scratch
    here and there. -> Baked."""
    mottle = fbm(24, seed, 4)
    col = col * (0.97 + 0.06 * mottle)[..., None]
    lines = scratches(seed + 50)
    col = mix(col, col * 1.25, lines * 0.2)
    h = fbm(96, seed + 20, 2) * 0.06 - lines * 0.2
    return Baked(col, chips(seed + 40, 14), normals_from_height(blur(h), 1.0), 0.94, rough + 0.04 * (mottle - 0.5))


def _air(colours, levels, seed, cells=(2, 3)):
    return _finished(_bands(colours, levels, seed, cells, edge=0.03, warp_px=18), seed + 2)


@recipe("us_compass_grey", tile=6.0, wear=(0.3, 0.31, 0.32, 0.8))
def us_compass_grey():
    """US Navy, Marine and Air Force tactical greys (FS 36375 over 36320): two
    close greys in broad soft patches."""
    return _air(((0.23, 0.25, 0.275), (0.19, 0.205, 0.226)), (0.6,), 1701)


@recipe("us_gunship_grey", tile=6.0, wear=(0.2, 0.2, 0.21, 0.8))
def us_gunship_grey():
    """US dark greys: the Strike Eagle's gunship grey (FS 36118), the F-35's
    and the A-10's darker tones; one grey, a little mottled."""
    return _air(((0.082, 0.09, 0.1), (0.096, 0.105, 0.116)), (0.66,), 1711)


@recipe("us_army_aviation", tile=4.0, wear=(0.08, 0.085, 0.07, 0.85))
def us_army_aviation():
    """US Army aviation green (FS 34031, near black green): Apache, Black Hawk,
    Chinook; the special-operations Little Birds are as dark."""
    return _air(((0.03, 0.035, 0.027), (0.038, 0.042, 0.032)), (0.62,), 1721)


@recipe("russian_air_blue", tile=6.0, wear=(0.28, 0.3, 0.32, 0.8))
def russian_air_blue():
    """Russian Flanker blue-greys: a pale blue-grey ground under darker
    grey-blue patches (the Su-27/30/35 and Su-34 schemes)."""
    return _air(((0.18, 0.21, 0.245), (0.1, 0.14, 0.2), (0.14, 0.17, 0.205)), (0.56, 0.7), 1731)


@recipe("russian_air_grey", tile=6.0, wear=(0.24, 0.25, 0.26, 0.8))
def russian_air_grey():
    """Russian greys: the MiG-29SMT's and MiG-31BM's light greys, the Su-57's
    splinter greys and the gunships' dark grey, as two greys in patches."""
    return _air(((0.15, 0.165, 0.185), (0.112, 0.124, 0.14)), (0.6,), 1741)


@recipe("russian_helicopter_camo", tile=6.0, wear=(0.17, 0.17, 0.13, 0.85))
def russian_helicopter_camo():
    """Russian army aviation camouflage: green and brown bands over a sand
    ground (the Mi-8AMTSh, the Su-25's older green-brown)."""
    return _air(((0.13, 0.12, 0.085), (0.055, 0.065, 0.04), (0.07, 0.05, 0.032)), (0.42, 0.66), 1751)


@recipe("chinese_air_grey", tile=6.0, wear=(0.22, 0.23, 0.24, 0.8))
def chinese_air_grey():
    """PLA air and army aviation greys: the Z-20's grey, the J-20's and Z-10's
    darker grey, as two greys in patches."""
    return _air(((0.115, 0.13, 0.147), (0.077, 0.087, 0.1)), (0.62,), 1761)


@recipe("chinese_air_light_grey", tile=6.0, wear=(0.3, 0.31, 0.32, 0.8))
def chinese_air_light_grey():
    """The PLA's light fighter grey (the J-10C): one pale grey, a little
    mottled, a shade bluer in broad patches."""
    return _air(((0.21, 0.23, 0.26), (0.19, 0.21, 0.24)), (0.62,), 1765)


@recipe("neutral_grey", tile=6.0, wear=(0.3, 0.315, 0.335, 0.8))
def neutral_grey():
    """A light grey of no air arm's hue (the test jet's): one grey, a little
    mottled, a shade cool so the battle's warm sun leaves it grey."""
    return _air(((0.2, 0.214, 0.232), (0.186, 0.199, 0.216)), (0.6,), 1797)


@recipe("store_grey", tile=3.0, wear=(0.3, 0.3, 0.3, 0.8))
def store_grey():
    """The flat neutral grey missiles, bombs and pods are painted: one grey,
    a little mottled; a store's own colour tints it."""
    return _air(((0.2, 0.2, 0.2), (0.19, 0.19, 0.19)), (0.6,), 1795)


@recipe("nato_air_grey", tile=6.0, wear=(0.27, 0.28, 0.29, 0.8))
def nato_air_grey():
    """European air force greys: the Typhoon's and Gripen's light greys, the
    Rafale's, Mirage 2000D's and Tornado's mid greys."""
    return _air(((0.155, 0.17, 0.19), (0.106, 0.12, 0.137)), (0.62,), 1771)


@recipe("french_air_camo", tile=6.0, wear=(0.2, 0.21, 0.19, 0.8))
def french_air_camo():
    """French strike camouflage (Mirage 2000D): a mid grey with dark grey-green
    patches."""
    return _air(((0.12, 0.125, 0.12), (0.06, 0.07, 0.055)), (0.56,), 1791)


@recipe("nato_helicopter_green", tile=4.0, wear=(0.1, 0.11, 0.09, 0.85))
def nato_helicopter_green():
    """European army aviation: the British and French dark green-greys
    (Wildcat, Merlin, Chinook HC6, Tiger, NH90)."""
    return _air(((0.042, 0.05, 0.038), (0.05, 0.055, 0.046)), (0.6,), 1781)
